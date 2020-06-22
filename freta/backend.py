# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""
MSAL based RESTful API wrapper
"""

from typing import Dict, Optional
import sys
import os
import contextlib
import json
import logging
import atexit
import time
import jwt
import msal
import requests
from requests.utils import urlparse, urlunparse
from azure.storage.blob import BlockBlobService

_ACCESSTOKENCACHE_UMASK = 0o077

LOGGER = logging.getLogger("nsv-backend")


@contextlib.contextmanager
def _temporary_umask(new_umask):
    prev_umask = None
    try:
        prev_umask = os.umask(new_umask)
        yield
    finally:
        if prev_umask is not None:
            os.umask(prev_umask)


class Backend:  # pylint: disable=unused-variable
    """ MSAL based RESTful API wrapper implementation """

    def __init__(
        self, config_path, token_path, config: Optional[Dict[str, str]] = None
    ):
        self.config_path = os.path.expanduser(config_path)
        self.token_path = os.path.expanduser(token_path)
        self.config = config or {}
        self.token_cache = None
        self.init_cache()
        self.app = None
        self.token_expires = 0
        self.load_config()
        self.session = requests.Session()

        atexit.register(self.save_cache)

    def load_config(self):
        """ Update current config with saved data """

        if os.path.exists(self.config_path):
            with open(self.config_path, "r") as handle:
                self.config.update(json.load(handle))

    def save_config(self):
        """ Save a config as json """

        with open(self.config_path, "w") as handle:
            json.dump(self.config, handle)

    def init_cache(self):
        """ Initialize MSAL token cache """

        # Ensure the token_path directory exists
        try:
            dir_name = os.path.dirname(self.token_path)
            with _temporary_umask(_ACCESSTOKENCACHE_UMASK):
                os.makedirs(dir_name)
        except FileExistsError:
            pass

        self.token_cache = msal.SerializableTokenCache()
        if os.path.exists(self.token_path):
            with open(self.token_path, "r") as handle:
                self.token_cache.deserialize(handle.read())

    def save_cache(self):
        """ Save the MSAL token cache as appropriate """

        if self.token_cache is None:
            return

        if self.token_path is None:
            return

        with _temporary_umask(_ACCESSTOKENCACHE_UMASK):
            with open(self.token_path, "w") as handle:
                handle.write(self.token_cache.serialize())

    def logout(self):
        """Forget any saved access token. Deletes the `token_path` file."""

        # NOTE: MSAL documentation suggests that just removing the account is enough.
        # https://msal-python.readthedocs.io/en/latest/#msal.ClientApplication.remove_account
        #
        # That said, as we are not supporting multiple logins, remove the cache file as well.

        LOGGER.debug("logout()")

        self.app = None
        self.token_cache = None
        if os.path.exists(self.token_path):
            os.unlink(self.token_path)
        return True

    def headers(self):
        """ Build the HTTP Headers with an Authentication header as appropriate """

        value = {}
        if self.config["client_id"] is not None:
            access_token = self.get_access_token()
            value["Authorization"] = "%s %s" % (
                access_token["token_type"],
                access_token["access_token"],
            )
        return value

    def get_access_token(self):
        """ Get an Oauth2 token """

        scopes = self.config["scopes"]
        if "client_secret" in self.config:
            return self.client_secret(scopes)
        return self.device_login(scopes)

    def get_my_id(self):
        """ Extract the current user's OID """

        access_token = self.get_access_token()
        decoded = jwt.decode(access_token["access_token"], verify=False)
        return "%s-%s" % (decoded["tid"], decoded["oid"])

    def client_secret(self, scopes):
        """ Create a MSAL app based on a client id & secret """

        if not self.app:
            self.app = msal.ConfidentialClientApplication(
                self.config["client_id"],
                authority=self.config["authority"],
                client_credential=self.config["client_secret"],
                token_cache=self.token_cache,
            )
        result = self.app.acquire_token_for_client(scopes=scopes)
        if "error" in result:
            raise Exception(
                "error: %s\n'%s'"
                % (result.get("error"), result.get("error_description"))
            )
        return result

    def device_login(self, scopes):
        """ Create a MSAL app using the devicelogin workflow """

        if not self.app:
            self.app = msal.PublicClientApplication(
                self.config["client_id"],
                authority=self.config["authority"],
                token_cache=self.token_cache,
            )

        accounts = self.app.get_accounts()
        if accounts:
            access_token = self.app.acquire_token_silent(scopes, account=accounts[0])
            if access_token:
                return access_token

        LOGGER.info("Attempting interactive device login")
        print("Please login")
        flow = self.app.initiate_device_flow(scopes=scopes)
        message = flow.get("message")
        if message:
            print(message)
            access_token = self.app.acquire_token_by_device_flow(flow)
        elif flow.get("error"):
            raise Exception(
                "error: %s\n'%s'" % (flow.get("error"), flow.get("error_description"))
            )
        else:
            raise Exception("Interactive device authentication failed: '%s'" % flow)

        if access_token:
            LOGGER.info("Interactive device authentication succeeded")
            print("Login succeeded")

        return access_token

    def request(
        self, method, path, json_data=None, params=None,
    ):
        """ Perform a RESTful API call using the MSAL app as appropriate """

        if not self.config["endpoint"]:
            raise Exception("endpoint not configured")
        url = self.config["endpoint"] + path
        headers = self.headers()

        response = None
        for backoff in range(1, 10):
            try:
                LOGGER.debug("request %s %s %s", method, url, repr(json_data))
                response = self.session.request(
                    method,
                    url,
                    headers=headers,
                    json=json_data,
                    params=params,
                    timeout=(10.0, 10.0),
                )
                break

            except requests.exceptions.ConnectionError as err:
                LOGGER.info("request connection error: %s", err)
            except requests.exceptions.ReadTimeout as err:
                LOGGER.info("request timed out: %s", err)

            time.sleep(1.5 ** backoff)

        if response is None:
            raise Exception("request failed: %s %s" % (method, url))

        if response.status_code / 100 != 2:
            error_text = str(
                response.content, encoding="utf-8", errors="backslashreplace"
            )
            raise Error(
                "request did not succeed: HTTP %s - %s"
                % (response.status_code, error_text)
            )

        return response.json()

    def get_url(self, url):
        """ Get a URL directly without the MSAL integration """

        return self.session.get(url, stream=True).content

    @classmethod
    def get_sas_parts(cls, sas_uri):
        """ Parse a SAS URL into component parts """

        parsed = urlparse(sas_uri)
        account_name, service, endpoint_suffix = parsed.hostname.split(".", 2)
        if service != "blob":
            raise ValueError
        query = parsed.query
        path = parsed.path.split("/")[1:]
        container = path[0]
        blob_name = "/".join(path[1:])
        return account_name, endpoint_suffix, container, blob_name, query

    def upload_blob(self, sas_token, path):
        """ Upload a blob to a given sas path """
        (
            account_name,
            endpoint_suffix,
            container,
            blob_name,
            sas_query,
        ) = self.get_sas_parts(sas_token)
        blob = BlockBlobService(
            account_name=account_name,
            endpoint_suffix=endpoint_suffix,
            sas_token=sas_query,
        )
        blob.create_blob_from_path(container, blob_name, path)


def wait(func, frequency=1.0):  # pylint: disable=unused-variable
    """
    Wait until the provided func returns True

    Provides user feedback via a spinner if stdout is a TTY.
    """

    isatty = sys.stdout.isatty()
    frames = ["-", "\\", "|", "/"]
    waited = False
    message = ""
    last_message = message
    result = None
    try:
        while True:
            result = func()
            if isinstance(result, tuple):
                if result[0]:
                    break
                message = result[1]
            elif result:
                break

            if isatty:
                frames.sort(key=frames[0].__eq__)
                sys.stdout.write("\r%s %s\033[K" % (frames[0], message))
                sys.stdout.flush()
            elif last_message != message:
                print(message, flush=True)
            waited = True
            time.sleep(frequency)
            last_message = message
    finally:
        if waited and isatty:
            print()


class Error(BaseException):
    """ API error """

    def __init__(self, value):
        super().__init__()
        LOGGER.error(value)
        self.value = value

    def __str__(self):
        return repr(self.value)
