#!/usr/bin/env python
#
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.
#
# utility to wait for images to finish processing
#

import argparse
import time
from freta.api import Freta


def main():
    parser = argparse.ArgumentParser(description="Wait for images to finish processing")
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--ids", nargs="+", help="Freta Image IDs")
    group.add_argument("--all", action="store_true", help="Search all VMs")
    args = parser.parse_args()

    freta = Freta()

    while True:
        done = True
        for image in freta.image.list():
            if args.ids and image["image_id"] not in args.ids:
                continue
            if image["state"] not in ["Report available", "Failed"]:
                print(
                    "waiting on", image["image_id"], image["machine_id"],
                )
                done = False

        if done:
            break
        time.sleep(10)

    print("done")


if __name__ == "__main__":
    main()
