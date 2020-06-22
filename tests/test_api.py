#!/usr/bin/env python

# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

import logging
import inspect
import unittest
from unittest.mock import patch
from contextlib import contextmanager

from freta.backend import Backend, Error
from freta.api import Freta
from .freta_test_utils import default_vcr, check_image, check_uuid

VCR = default_vcr()


@contextmanager
def mock_logins():
    patches = []
    to_patch = {
        "headers": {"return_value": {"Authorization": "bearer AAAAA"}},
        "get_my_id": {
            "return_value": "00000000-0000-0000-0000-000000000000-00000000-0000-0000-0000-000000000000"
        },
        "upload_blob": {"return_value": True},
        "get_url": {"return_value": "some content"},
    }

    frame = inspect.currentframe().f_back.f_back.f_back
    if "cassette" in frame.f_locals and frame.f_locals["cassette"].rewound:
        logging.debug("mocking patches %s", to_patch)
        patches = [patch.object(Backend, x, **to_patch[x]) for x in to_patch]
    else:
        logging.debug("NOT mocking")

    for i in patches:
        i.start()

    yield

    for item in patches:
        item.stop()


class TestAPI(unittest.TestCase):
    """ Test Freta API using PyVCR """

    @VCR.use_cassette()
    @mock_logins()
    def test_api_formats(self):
        data = Freta().image.formats()
        self.assertIsInstance(data, dict)
        self.assertIn("lime", data)
        self.assertEqual(data["lime"], "LiME image")

    @VCR.use_cassette()
    @mock_logins()
    def test_api_regions(self):
        data = Freta().regions()
        self.assertIsInstance(data, dict)
        self.assertIn("eastus", data)
        self.assertEqual(data["eastus"]["name"], "East US")

    @VCR.use_cassette()
    @mock_logins()
    def test_api_versions(self):
        data = Freta().versions()
        self.assertIsInstance(data, dict)
        self.assertIn("analysis", data)

    @VCR.use_cassette()
    @mock_logins()
    def test_api_search_filters(self):
        data = Freta().image.search_filters()
        self.assertIsInstance(data, list)
        self.assertIn("my_images_and_samples", data)

    # https://docs.microsoft.com/en-us/azure/storage/common/storage-dotnet-shared-access-signature-part-1#service-sas-uri-example
    def verify_sas(self, sas):
        # encrypted
        self.assertTrue(sas.startswith("https://"))

        # write only
        self.assertIn("&sp=w&", sas)

        # resource is a blob
        self.assertIn("&sr=b&", sas)

    @VCR.use_cassette()
    @mock_logins()
    def test_api_upload(self):
        freta = Freta()
        data = freta.image.upload(
            "freta-cli-unit-test upload", "lime", "eastus", "README.md"
        )
        self.assertIn("image_id", data)
        freta.image.delete(data["image_id"])

        data = freta.image.upload_sas("freta-cli-unit-test upload", "lime", "eastus")
        self.assertIn("image_id", data)
        self.verify_sas(data["image"]["sas_url"])
        self.verify_sas(data["profile"]["sas_url"])
        freta.image.delete(data["image_id"])

    @VCR.use_cassette()
    @mock_logins()
    def test_api_get_image_artifacts(self):
        freta = Freta()
        images = freta.image.list()
        for image in images:
            self.assertTrue(check_image(image))

        image = [x for x in images if x["state"] == "Report available"][0]
        artifacts = freta.image.artifact.list(image["image_id"])
        self.assertIsInstance(artifacts, list)
        self.assertIn("report.json", artifacts)

        # TODO: add report content validation
        content = freta.image.artifact.get(image["image_id"], "report.json")
        self.assertGreater(len(content), 10)

    def check_diffs(self, image_a, image_b, expected):
        self.assertTrue(check_image(image_a))
        self.assertTrue(check_image(image_b))
        diffs = [
            x for x in image_a if image_a[x] != image_b[x] and x not in ["Timestamp"]
        ]
        self.assertEqual(sorted(diffs), sorted(expected))
        return image_b

    @VCR.use_cassette()
    @mock_logins()
    def test_api_create_update_analyze_delete(self):
        freta = Freta()
        image_sas = freta.image.upload_sas(
            "freta-cli-unittest-upload", "lime", "eastus"
        )
        self.assertIn("sas_url", image_sas["image"])
        image_id = image_sas["image_id"]
        self.assertTrue(check_uuid(image_id))

        # no change
        image = freta.image.status(image_id)
        freta.image.update(image_id, name="this is a test name")
        # TODO machine_id is force set in mocking, so we only verify we didn't assert
        # image = self.check_diffs(image, freta.status(image_id), ["machine_id"])

        image = freta.image.status(image_id)
        freta.backend.upload_blob(image_sas["image"]["sas_url"], "README.md")
        freta.image.analyze(image_id)
        image = self.check_diffs(image, freta.image.status(image_id), ["state"])
        freta.image.delete(image_id)

    # artifacts are only available to download when there is a successful scan
    @VCR.use_cassette()
    @mock_logins()
    def test_api_artfacts_without_scan(self):
        freta = Freta()
        image = freta.image.upload(
            "freta-cli-unittest upload", "lime", "eastus", "README.md"
        )
        with self.assertRaises(Error):
            freta.image.artifact.get(image["image_id"], "image.lime")
        freta.image.delete(image["image_id"])


if __name__ == "__main__":
    unittest.main()
