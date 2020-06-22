#!/usr/bin/env python
#
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.
#
# Sample code to glue Hyper-V and Freta together.
#

import os
import distutils.spawn
import logging
import logging.config
import argparse
import uuid
from subprocess import check_output, DEVNULL
from base64 import b64encode
import json
from freta.api import Freta

IS_WSL = distutils.spawn.find_executable("wslpath")


def cleanup(data):
    if isinstance(data, list):
        return [cleanup(x) for x in data]
    if isinstance(data, dict):
        for key in data:
            if key == "Path" and IS_WSL:
                logging.debug("converting path to wslpath: %s", data[key])
                data[key] = (
                    check_output(["wslpath", data[key]], stderr=DEVNULL)
                    .decode("utf8")
                    .strip()
                )
        return data
    return data


def run(cmd):
    cmd = cmd + " | ConvertTo-Json"
    cmd = b64encode(cmd.encode("utf_16_le")).decode("ascii")
    cmd = [
        "powershell.exe",
        "-NonInteractive",
        "-NoProfile",
        "-NoLogo",
        "-EncodedCommand",
        cmd,
    ]

    if IS_WSL:
        cmd += ["-WindowStyle", "Hidden"]
    logging.debug("launching: %s", " ".join(cmd))
    result = check_output(cmd, stderr=DEVNULL).decode("latin-1")

    if not result:
        return None

    return cleanup(json.loads(result))


def vm_list(query=None):
    cmd = ["Get-VM", "Select VMId,VMName"]
    if query is not None:
        cmd.insert(1, query)
    data = run(" | ".join(cmd))

    # we get a dict if it's only one instance.  we still want a list
    if isinstance(data, dict):
        data = [data]
    return data


def snapshot_create(vm, snapshot):
    return run(
        'Checkpoint-VM -VMName "{}" -SnapshotName "{}" -Confirm:$false'.format(
            vm, snapshot
        )
    )


def snapshot_remove(vm, snapshot):
    return run(
        'Get-VMSnapshot -VMName "{}" -Name "{}" | Remove-VmSnapshot'.format(
            vm, snapshot
        )
    )


def snapshot_list(vm):
    return run('Get-VMSnapshot -VMName "{}" | Select ID,Name,Path'.format(vm))


def snapshot_upload(vm, snapshot, region):
    data = run(
        'Get-VMSnapshot -VMName "{}" -Name "{}" | Select Id,Path'.format(vm, snapshot)
    )

    if isinstance(data, list):
        data = data[-1]

    path = os.path.join(data["Path"], "Snapshots", "%s.VMRS" % data["Id"].upper())
    name = "{} : {}".format(vm, snapshot)
    freta = Freta()
    upload = freta.image.upload(name, "vmrs", region, path)
    freta.image.analyze(upload["image_id"])


def auto(region):
    for entry in vm_list(query="where {$_.State -eq 'Running'}"):
        logging.info("creating snapshot for %s", entry["VMName"])
        snapshot = str(uuid.uuid4())
        run(
            'Checkpoint-VM -VMName "{}" -SnapshotName "{}"'.format(
                entry["VMName"], snapshot
            )
        )
        snapshot_upload(entry["VMName"], snapshot, region)
        snapshot_remove(entry["VMName"], snapshot)


def main():
    parser = argparse.ArgumentParser(
        description="Image VMs from Hyper-V on the local computer"
    )
    parser.add_argument(
        "-v", "--verbose", help="increase output verbosity", action="store_true"
    )
    subparsers = parser.add_subparsers(metavar="command")
    sub = subparsers.add_parser("vm_list", help="vm_list")
    sub.set_defaults(func=lambda x: vm_list())

    sub = subparsers.add_parser("snapshot_list", help="list snapshots")
    sub.add_argument("vm")
    sub.set_defaults(func=lambda x: snapshot_list(x.vm))

    sub = subparsers.add_parser("snapshot_create", help="create snapshots")
    sub.add_argument("vm")
    sub.add_argument("snapshot")
    sub.set_defaults(func=lambda x: snapshot_create(x.vm, x.snapshot))

    sub = subparsers.add_parser("snapshot_remove", help="snapshot_remove")
    sub.add_argument("vm")
    sub.add_argument("snapshot_id")
    sub.set_defaults(func=lambda x: snapshot_remove(x.vm, x.snapshot_id))

    sub = subparsers.add_parser("snapshot_upload", help="snapshot_upload")
    sub.add_argument("vm")
    sub.add_argument("snapshot_id")
    sub.add_argument("region")
    sub.set_defaults(func=lambda x: snapshot_upload(x.vm, x.snapshot_id, x.region))

    sub = subparsers.add_parser(
        "auto", help="create a new snapshot for every running VM and upload it to freta"
    )
    sub.add_argument("region")
    sub.set_defaults(func=lambda x: auto(x.region))

    args = parser.parse_args()
    logging.config.dictConfig({"version": 1, "disable_existing_loggers": True})
    if args.verbose:
        logging.getLogger().setLevel(logging.DEBUG)
    else:
        logging.getLogger().setLevel(logging.INFO)

    if not hasattr(args, "func"):
        logging.error("no command specified")
        return

    result = args.func(args)
    if result:
        print(json.dumps(result, indent=4, sort_keys=True))


if __name__ == "__main__":
    main()
