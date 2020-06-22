#!/usr/bin/env python
#
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.
#
# Re-analyze all images that failed analysis

from freta.api import Freta


def main():
    freta = Freta()
    for image in freta.image.list():
        if image["state"] == "Failed":
            print("redoing %s" % image["image_id"])
            freta.image.analyze(image["image_id"])


if __name__ == "__main__":
    main()
