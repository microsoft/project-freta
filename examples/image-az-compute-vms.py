#!/usr/bin/env python
#
# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.
#
# Image & analyze Azure Compute VMs in the current 'az login' provided
# subscription.
#
# NOTE: this requires the VM be able to connect outbound to multiple HTTPS
# nodes.
#
# Uses AVML for imaging
#   See https://github.com/microsoft/avml
#
# Uses Azure Custom Script VM Extension to launch AVML
#   See https://github.com/Azure/custom-script-extension-linux
#
# Requires the additional python packages to be installed:
#   azure-cli-core
#   azure-mgmt-compute

import argparse
import sys
import time
from azure.common.client_factory import get_client_from_cli_profile
from azure.mgmt.compute import ComputeManagementClient
from azure.mgmt.compute.models import VirtualMachineExtension
from msrestazure.tools import parse_resource_id
from freta.api import Freta
from freta.backend import wait


class Runner:
    def __init__(self):
        self.az = get_client_from_cli_profile(ComputeManagementClient)
        self.freta = Freta()
        self.publisher = "Microsoft.Azure.Extensions"
        self.extension = "customScript"

    def get_vm_by_id(self, vm_id):
        vm = parse_resource_id(vm_id)
        return self.az.virtual_machines.get(vm["resource_group"], vm["resource_name"])

    def get_vm(self, group, name):
        return self.az.virtual_machines.get(group, name)

    def list_vms(self, group=None):
        if group:
            return self.az.virtual_machines.list(group)
        return self.az.virtual_machines.list_all()

    def get_extension_version(self, vm):
        # versions are a.b.c, azure only wants a.b
        versions = [
            x.name
            for x in self.az.virtual_machine_extension_images.list_versions(
                vm.location, self.publisher, self.extension
            )
        ]
        return versions[-1].rsplit(".", 1)[0]

    def image_vm(self, vm):
        group = parse_resource_id(vm.id)["resource_group"]
        tokens = self.freta.image.upload_sas(vm.name, "lime", "eastus")

        settings = {
            "fileUris": [
                "https://github.com/microsoft/avml/releases/download/v0.2.0/avml"
            ]
        }

        cmd = (
            "./avml --sas_block_size 20 --delete --compress --sas_url '%s' /root/image.lime.%f"
            % (tokens["image"]["sas_url"], time.time())
        )
        protected_settings = {"commandToExecute": cmd}

        version = self.get_extension_version(vm)

        extension = VirtualMachineExtension(
            location=vm.location,
            publisher=self.publisher,
            virtual_machine_extension_type=self.extension,
            settings=settings,
            protected_settings=protected_settings,
            type_handler_version=version,
            auto_upgrade_minor_version=False,
        )

        poller = self.az.virtual_machine_extensions.create_or_update(
            group, vm.name, self.extension, extension
        )
        print("imaging: {} ({})".format(repr(vm.name), tokens["image_id"]))
        return (tokens["image_id"], poller)


def build_arg_parser():
    parser = argparse.ArgumentParser(
        description="Image VMs from the current 'az login' subscription"
    )
    subparsers = parser.add_subparsers(title="Available commands", metavar="command")

    vm = subparsers.add_parser("vm", help="Image the specified VM")
    vm.add_argument("group", help="Resource Group the VM resides")
    vm.add_argument("name", help="Name of the VM to image")
    vm.set_defaults(method="vm")

    group = subparsers.add_parser("group", help="Image the specified Group")
    group.add_argument("group", help="Resource Group the VM resides")
    group.set_defaults(method="group")

    by_id = subparsers.add_parser("by_id", help="Image VMs based on the VM ID")
    by_id.add_argument("ids", nargs="+", help="VM IDs")
    by_id.set_defaults(method="by_id")

    all_images = subparsers.add_parser("all", help="Image all VMs in a subscription")
    all_images.set_defaults(method="all")
    return parser


def get_vms(args, runner):
    vms = []
    if args.method == "vm":
        vms.append(runner.get_vm(args.group, args.name))
    elif args.method == "group":
        for vm in runner.list_vms(args.group):
            vms.append(vm)
    elif args.method == "by_id":
        for item in args.ids:
            vms.append(runner.get_vm_by_id(item))
    else:
        for vm in runner.list_vms():
            vms.append(vm)
    return vms


def image_vms(vms, runner):
    checking = {}
    for vm in vms:
        image_id, poller = runner.image_vm(vm)
        checking[image_id] = {"vm": vm, "poller": poller}

    def check():
        if not checking:
            return True

        waiting = {"imaging": 0, "analysis": 0, "done": 0}

        for image_id in sorted(checking.keys()):
            name = checking[image_id]["vm"].name
            poller = checking[image_id].get("poller")
            if poller:
                if poller.done():
                    result = poller.result()
                    if result.provisioning_state != "Succeeded":
                        print(
                            "imaging failed: {} ({})",
                            repr(name),
                            result.provisioning_state,
                        )
                        del checking[image_id]
                        continue

                    print("analyzing: {} {}".format(repr(name), image_id))
                    waiting["analysis"] += 1
                    runner.freta.image.analyze(image_id)
                    del checking[image_id]["poller"]
                else:
                    waiting["imaging"] += 1
            else:
                state = runner.freta.image.status(image_id)["state"]
                if "available" in state.lower() or "fail" in state.lower():
                    print("analyzed: {} {} ({})".format(repr(name), image_id, state))
                    del checking[image_id]
                else:
                    waiting["analysis"] += 1

        message = []
        for key in sorted(waiting.keys()):
            if waiting[key]:
                message.append("{}: {}".format(key, waiting[key]))
        if message:
            return (False, "{} ".format(" ".join(message)))
        return True

    wait(check)


def main():
    parser = build_arg_parser()
    args = parser.parse_args()
    if not hasattr(args, "method"):
        print("no method selected")
        parser.print_help()
        return 1

    runner = Runner()
    vms = get_vms(args, runner)

    image_vms(vms, runner)
    return 0


if __name__ == "__main__":
    sys.exit(main())
