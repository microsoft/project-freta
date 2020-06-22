#!/usr/bin/env python
#
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.
#
# Search reports with JMESPath
#
# Requires the additional python packages to be installed:
#   jmespath
#
# Example:
#   $ search-reports.py --query 'info.Info.kernel' --all
#   "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa": {
#       "name": "my-fancy-vm-name",
#       "result": "Linux version 4.9.0-0.bpo.2-amd64 (debian-kernel@lists.debian.org) (gcc version 4.9.2 (Debian 4.9.2-10) ) #1 SMP Debian 4.9.18-1~bpo8+1 (2017-04-10)\n"
#   }
#   $

import argparse
import json
import jmespath
from freta.api import Freta


def main():
    parser = argparse.ArgumentParser(
        description="Image VMs from the current 'az login' subscription"
    )
    parser.add_argument("--query", help="Search query", required=True)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--ids", nargs="+", help="Freta Image IDs")
    group.add_argument("--all", action="store_true", help="Search all VMs")
    args = parser.parse_args()

    expression = jmespath.compile(args.query)

    results = {}
    freta = Freta()
    for image in freta.image.list():
        if args.ids and image["image_id"] not in args.ids:
            continue
        if image["state"] != "Report available":
            continue
        raw_report = freta.image.artifact.get(image["image_id"], "report.json")
        report = json.loads(raw_report)
        result = expression.search(report)
        if not result:
            continue
        results[image["image_id"]] = {"name": image["machine_id"], "result": result}

    if results:
        print(json.dumps(results, sort_keys=True, indent=4))


if __name__ == "__main__":
    main()
