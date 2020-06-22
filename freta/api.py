# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""
Python interface to the Freta service.
"""

import logging
import os

from typing import Optional
from .backend import Backend
from .types import File

DEFAULT_CONFIG_PATH = os.path.join("~", ".cache", "freta", "config.json")
DEFAULT_TOKEN_PATH = os.path.join("~", ".cache", "freta", "access_token.json")

# Freta service public endpoint configuration
DEFAULT_CONFIG = {
    "client_id": "5248a6f3-492f-4921-af11-b0f2faa2dca8",
    "authority": "https://login.microsoftonline.com/common",
    "endpoint": "https://freta.azure-api.net/freta/0.0.1/",
    "scopes": ["https://microsoft.onmicrosoft.com/freta-api/.default"],
}


class Endpoint:  # pylint: disable=too-few-public-methods
    """ Base class for Freta API """

    def __init__(self, freta):
        self.freta = freta
        self.logger = freta.logger
        self.backend = freta.backend


class Artifact(Endpoint):
    """ Interact with Artifacts """

    def list(self, image_id: str, owner_id: Optional[str] = None):
        """Get the list of artifacts associated with a specific image.

        :param image_id: Image identifier.
        :param owner_id: For group-owned images, userid of the image owner.

        :return: [str]

        TODO example
        """
        if not owner_id:
            owner_id = self.backend.get_my_id()
        self.logger.debug("artifacts(image_id=%s, owner_id=%s)", image_id, owner_id)

        return self.backend.request(
            "POST", "list_artifacts", {"owner_id": owner_id, "image_id": image_id}
        )

    def get(self, image_id: str, filename: str, owner_id: Optional[str] = None):
        """Get an artifact related to the image.

        :param image_id: Freta identifier for the image
        :param filename: Name of the artifact file.
        :param owner_id: Optional userid of the image owner (For group owned images)
        :return: Bytes-like object containing the content of the artifact.

        .. TODO Can we link to the binary return type?
        """

        if not owner_id:
            owner_id = self.backend.get_my_id()
        self.logger.debug(
            "artifact(image_id=%s, filename=%s, owner_id=%s)",
            image_id,
            filename,
            owner_id,
        )

        url = self.backend.request(
            "POST",
            "get_artifact",
            {"owner_id": owner_id, "image_id": image_id, "filename": filename},
        )["url"]

        return self.backend.get_url(url)


class Image(Endpoint):
    """ Interact with Images """

    def __init__(self, freta):
        super().__init__(freta)
        self.artifact = Artifact(freta)

    def formats(self):
        """Get the list of currently supported image formats.

        :return: dict

        Example result:

        .. code:: Python

           {
               "lime": "LiME image",
               "raw": "Raw Physical Memory Dump",
               "vmrs": "Hyper-V Memory Snapshot"
           }
        """
        self.logger.debug("formats()")
        return self.backend.request("GET", "image_formats")

    def list(self, search_filter: Optional[str] = None):
        """Get the list of images and their statuses.

        :param search_filter: Filter search results, call :py:meth:`~freta.Freta.search_filters` for allowed values.
        :return: List of dicts having keys as shown below.

        .. _Freta Portal: https://freta.azurewebsites.net/

        Example result:

        .. code:: Python

           [
               {
                   "Timestamp": "2019-05-13 18:50:01",
                   "image_id": "7fe75a61-b346-4a64-81f1-6389d12901f2",
                   "image_type": "lime",
                   "machine_id": "ubuntu-16.04-4.15.0-1040-azure",
                   "owner_id": "986c3ebe-18e9-4c89-afad-1178c21603e1",
                   "region": "eastus",
                   "state": "Report available"
               }
           ]
        """

        if search_filter is None:
            search_filter = "my_images"
        self.logger.debug("list_images(search_filter=%s)", search_filter)

        return self.freta.backend.request(
            "POST", "list_images", {"filter": search_filter}
        )

    def update(
        self, image_id: str, owner_id: Optional[str] = None, name: Optional[str] = None,
    ):
        """Update metadata for an image.

        :param image_id: Freta identifier for the image
        :param owner_id: For group owned images, the userid of the image owner.
        :param name: Optionally set the user specified machine identifer for the image

        TODO example
        """
        if not owner_id:
            owner_id = self.backend.get_my_id()
        self.logger.debug(
            "update(%s, owner_id=%s, name=%s)", image_id, owner_id, name,
        )

        data = {"owner_id": owner_id, "image_id": image_id}

        if name:
            data["name"] = name

        return self.backend.request("PATCH", "get_image", data)

    def delete(self, image_id: str, owner_id: Optional[str] = None):
        """Delete an image along with any of its generated reports and other artifacts.

        :param image_id: Freta identifier for the image.
        :param owner_id: Optional userid of the image owner (For group owned images)
        :return: True
        """

        if not owner_id:
            owner_id = self.backend.get_my_id()
        self.logger.debug("delete(image_id=%s, owner_id=%s)", image_id, owner_id)

        return self.backend.request(
            "POST", "delete_image", {"owner_id": owner_id, "image_id": image_id}
        )

    def analyze(self, image_id: str, owner_id: Optional[str] = None):
        """Analyze (or re-analyze) an uploaded image.

        :param image_id: Image identifier.
        :param owner_id: For group-owned images, userid of the image owner.

        :return: True

        TODO example
        """
        if not owner_id:
            owner_id = self.backend.get_my_id()
        self.logger.debug("analyze(%s, %s)", image_id, owner_id)

        return self.backend.request(
            "POST", "analyze", {"owner_id": owner_id, "image_id": image_id}
        )

    def cancel_analysis(self, image_id: str, owner_id: Optional[str] = None):
        """Cancel analyze an uploaded image.

        :param image_id: Image identifier.
        :param owner_id: For group-owned images, userid of the image owner.

        :return: True

        TODO example
        """
        if not owner_id:
            owner_id = self.backend.get_my_id()
        self.logger.debug("cancel_analysis(%s, %s)", image_id, owner_id)

        return self.backend.request(
            "POST", "cancel_analysis", {"owner_id": owner_id, "image_id": image_id}
        )

    def status(self, image_id: str, owner_id: Optional[str] = None):
        """Get the status of a single image.

        :param image_id: Freta identifier for the image.
        :param owner_id: userid of the image owner (For group-owned images)
        :returns: dict

        Example result:

        .. code:: Python

           {
               "Timestamp": "2019-06-11 19:03:17",
               "analysis_version": "0.0.0",
               "image_id": "23ca6dbe-4c6f-41c0-898e-82cdd56fcf4e",
               "image_type": "vmrs",
               "machine_id": "testing_upload_sas",
               "owner_id": "309fc32f-a06b-4821-a97b-194c271f9cc5",
               "region": "australiaeast",
               "state": "Upload started"
           }
        """
        if not owner_id:
            owner_id = self.backend.get_my_id()
        self.logger.debug("status(image_id=%s, owner_id=%s)", image_id, owner_id)

        return self.backend.request(
            "POST", "get_image", {"owner_id": owner_id, "image_id": image_id}
        )

    # pylint: disable=too-many-arguments
    def upload(
        self,
        name: str,
        image_type: str,
        region: str,
        image: File,
        profile: Optional[File] = None,
    ):
        """Upload an image file and submit it for analysis.

        :param name: User-specified name for the image.
        :param image_type: Format of the image. See :py:meth:`~freta.Freta.formats` for allowed values.
        :param region: Region within which to store and process the image.
                       See :py:meth:`~freta.Freta.regions` for allowed values.
        :param image: Filesystem path to the image file.
        :param profile: Filesystem path to kernel profile. (Optional)
        :returns: dict

        Example result:

        .. code:: Python

           {
               'image_id': '[guid string]',
               'owner_id': '[guid string]'
           }
        """

        self.logger.debug(
            "upload(name=%s, iamge_type=%s, region=%s, image=%s, profile=%s)",
            name,
            image_type,
            region,
            image,
            profile,
        )

        tokens = self.upload_sas(name, image_type, region)

        if profile:
            self.backend.upload_blob(tokens["profile"]["sas_url"], profile)

        self.backend.upload_blob(tokens["image"]["sas_url"], image)

        image_id = tokens["image_id"]
        owner_id = self.backend.get_my_id()
        self.analyze(image_id, owner_id=owner_id)
        return {"image_id": image_id, "owner_id": owner_id}

    def upload_sas(self, name: str, image_type: str, region: str):
        """Obtain SAS URIs authorizing (only) upload of an image and a profile.

        This method does not queue the image for analysis. Call self.analyze()
        with the returned *image_id* after writing the image data.

        :param name: Name for the image.
        :param image_type: Format of the image. See :py:meth:`~freta.Freta.formats` for allowed values.
        :param region: Region within which to store and process the image. See Freta.regions for allowed values. TODO link
        :return: dict

        Example result:

        .. code:: Python

             {
                "image": {
                    "sas_url": "https://fretaNNNN.blob.core.windows.net/..."
                },
                "image_id": "23ca6dbe-4c6f-41c0-898e-82cdd56fcf4e",
                "profile": {
                    "sas_url": "https://fretaNNNN.blob.core.windows.net/..."
                },
                "result": True
            }
        """
        self.logger.debug(
            "upload_sas(name=%s, image_type=%s, region=%s)", name, image_type, region
        )

        tokens = self.backend.request(
            "POST",
            "get_upload_token",
            {"machine_id": name, "image_type": image_type, "region": region},
        )

        return tokens

    def search_filters(self):
        """Lists the currently supported search filters.

        :return: [str]

        Example result:

        .. code:: Python

           [
               "my_images",
               "my_images_and_samples"
           ]
        """
        self.logger.debug("search_filters()")
        return self.backend.request("GET", "search_filters")


class Freta:  # pylint: disable=unused-variable
    """Python interface to the Freta service.

    Example:

    .. code:: Python

        from freta import Freta

        freta = Freta()
        freta.login()
        for image in freta.list_images():
            print("id: %s state: %s" % (image['image_id'], image['state']))
    """

    def __init__(self):
        self.logger = logging.getLogger("freta")
        self.backend = Backend(
            config_path=DEFAULT_CONFIG_PATH,
            token_path=DEFAULT_TOKEN_PATH,
            config=DEFAULT_CONFIG,
        )

        self.image = Image(self)

    def logout(self):
        """Forget any saved access token. Deletes the `token_path` file."""

        self.backend.logout()

    def regions(self):
        """Get the list of Azure regions currently supported by Freta for image storage and analysis.

        :return: dict

                key: region
                value: region details

        Example result:

        .. code:: Python

            {
                "australiaeast": {"default": false, "name": "Australia East"},
                "...many...": "...more...",
                "westus2": {"default": false, "name": "West US 2"}
            }
        """
        self.logger.debug("regions()")
        return self.backend.request("GET", "regions")

    def versions(self):
        """Get the versions of various Freta components.

        :return: dict

                key: component name
                value: version

        Example result:

        .. code:: Python

            {
                "analysis": "0.0.1"
            }
        """
        self.logger.debug("versions()")
        return self.backend.request("GET", "versions")

    def config(
        self,
        endpoint: Optional[str] = None,
        authority: Optional[str] = None,
        client_id: Optional[str] = None,
        client_secret: Optional[str] = None,
    ):
        """ Configure Freta CLI """
        self.logger.debug("set config")

        if endpoint is not None:
            self.backend.config["endpoint"] = endpoint
        if authority is not None:
            self.backend.config["authority"] = authority
        if client_id is not None:
            self.backend.config["client_id"] = client_id
        if client_secret is not None:
            self.backend.config["client_secret"] = client_secret

        print(self.backend.get_my_id())
        self.backend.app = None
        self.backend.save_config()

        data = self.backend.config.copy()
        if "client_secret" in data:
            data["client_secret"] = "***"

        if not data["endpoint"]:
            self.logger.warning("endpoint not configured yet")

        return data
