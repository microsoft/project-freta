#!/usr/bin/env python
#
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.
#
# Re-analyze all images that don't have latest version of the analysis available

from freta.api import Freta


def main():
    freta = Freta()
    versions = freta.versions()
    for image in freta.image.list():
        if (
            image["state"] == "Report available"
            and image["analysis_version"] != versions["analysis"]
        ):
            print("redoing %s" % image["image_id"])
            freta.image.analyze(image["image_id"])


if __name__ == "__main__":
    main()
