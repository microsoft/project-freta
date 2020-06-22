# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

import json
import re
import vcr

DEFAULT_TIMESTAMP = "2020-02-02 20:02:20"
DEFAULT_NAME = "TEST-NAME"
DEFAULT_UUID = "00000000-0000-0000-0000-000000000000"
DEFAULT_USERID = (
    "00000000-0000-0000-0000-000000000000-00000000-0000-0000-0000-000000000000"
)
DEFAULT_SAS = "https://contoso.com/path/file.txt?se=2019-07-22T14%3A21%3A15Z&sp=w&sv=2018-03-28&sr=b&sig=SIGNATURE"

UUID_RE = re.compile(
    r"^[a-f0-9]{8}-?[a-f0-9]{4}-?[a-f0-9]{4}-?[a-f0-9]{4}-?[a-f0-9]{12}\Z", re.I
)

USER_ID = re.compile(
    r"^[a-f0-9]{8}-?[a-f0-9]{4}-?[a-f0-9]{4}-?[a-f0-9]{4}-?[a-f0-9]{12}-[a-f0-9]{8}-?[a-f0-9]{4}-?[a-f0-9]{4}-?[a-f0-9]{4}-?[a-f0-9]{12}\Z",
    re.I,
)


def check_uuid(uuid):
    return bool(UUID_RE.match(uuid))


def check_user_id(user_id):
    return bool(USER_ID.match(user_id))


def check_image(image):
    if not isinstance(image, dict):
        return False

    fields = [
        "image_id",
        "image_type",
        "machine_id",
        "owner_id",
        "region",
        "state",
        "Timestamp",
    ]

    for field in fields:
        if field not in image:
            print("missing field %s", field)
            return False

    if not check_user_id(image["owner_id"]):
        print("bad owner_id")
        return False

    if not check_uuid(image["image_id"]):
        print("bad image_id")
        return False

    return True


def cleanup_body(data):
    if isinstance(data, list):
        cleaned = [cleanup_body(x) for x in data]

        # don't bother with huge numbers of arficacts
        cleaned = [
            x
            for x in cleaned
            if (not isinstance(x, str) or not x.startswith("artifacts/"))
        ]

        # if the response is a huge list of images, only a handful of images
        if all([check_image(x) for x in cleaned]):
            cleaned = [x for x in cleaned if x["state"] == "Report available"][:2]

        return cleaned

    defaults = {
        "owner_id": DEFAULT_USERID,
        "image_id": DEFAULT_UUID,
        "machine_id": DEFAULT_NAME,
        "Timestamp": DEFAULT_TIMESTAMP,
        "url": DEFAULT_SAS,
        "sas_url": DEFAULT_SAS,
    }

    if isinstance(data, dict):
        for field in data:
            if field in defaults:
                data[field] = defaults[field]

            if isinstance(data, (list, dict)):
                data[field] = cleanup_body(data[field])
    return data


def filter_request(request):
    # print("MAKING REQUEST", repr(request))
    if not request.uri.startswith("https://freta.azure-api.net/"):
        return None
    if request.body:
        body = json.loads(request.body)
        request.body = json.dumps(cleanup_body(body))
        # request.headers
    return request


def filter_response(response):
    for to_delete in ["Request-Context", "Date"]:
        if to_delete in response["headers"]:
            del response["headers"][to_delete]

    # only cleanup body if the response is positive
    if response["status"]["code"] == 200:
        body = json.loads(response["body"]["string"])
        body = cleanup_body(body)
        body = bytes(json.dumps(body), encoding="utf-8")
        response["body"]["string"] = body
        response["headers"]["Content-Length"] = [str(len(body))]

    return response


vcr.serializers.jsonserializer.serialize = lambda x: json.dumps(
    x, indent=4, sort_keys=True
)


def default_vcr():
    my_vcr = vcr.VCR(
        serializer="json",
        cassette_library_dir="tests/fixtures",
        record_mode="new_episodes",
        filter_headers=["authorization"],
        before_record_request=filter_request,
        before_record_response=filter_response,
        match_on=["uri", "method"],
    )
    return my_vcr
