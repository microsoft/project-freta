# Copyright (c) Microsoft Corporation. All rights reserved.
# Licensed under the MIT License.

"""
Simple CLI builder on top of a defined API
"""

from typing import Optional, List, Dict
import os
import sys
import json
import logging
import argparse
import inspect
import jmespath
from .types import Directory, File

LOGGER = logging.getLogger("cli")

JMES_HELP = (
    "JMESPath query string. See http://jmespath.org/ "
    "for more information and examples."
)


def get_arg_names(func):
    """ Get function argument names """
    spec = inspect.getfullargspec(func)
    return spec.args[1:]


def call_func(func, args):
    """ Get the arguments for the specified function and call it """
    myargs = {}
    for arg in get_arg_names(func):
        if hasattr(args, arg):
            myargs[arg] = getattr(args, arg)
    return func(**myargs)


class AsDict(argparse.Action):  # pylint: disable=too-few-public-methods
    """ A key/value pair based argparse action """

    def __call__(self, parser, namespace, values, option_string=None):
        as_dict = {}
        for value in values:
            key, value = value.split("=", 1)
            as_dict[key] = value
        setattr(namespace, self.dest, as_dict)


def arg_dir(arg):
    """ Verify the specified argument is a directory """
    if not os.path.isdir(arg):
        raise argparse.ArgumentTypeError("not a directory: %s" % arg)
    return arg


def arg_file(arg):
    """ Verify the specified argument is a file """

    if not os.path.isfile(arg):
        raise argparse.ArgumentTypeError("not a file: %s" % arg)
    return arg


def arg_bool(arg):
    """ Verify the specified argument is either true or false """

    if arg.lower() == "true":
        return True

    if arg.lower() == "false":
        return False

    raise argparse.ArgumentTypeError("not a boolean: %s" % arg)


def add_func_args(parser, impl):  # pylint: disable=too-many-branches
    """ Convert a function signature into argparse parameters

    This uses python type annotations to inform how the argparse params
    are created.
    """
    sig = inspect.signature(impl)
    for arg in sig.parameters:
        if arg == "self":
            continue

        args = [arg]
        kwargs = {}

        if not (
            isinstance(sig.parameters[arg].default, bool)
            or sig.parameters[arg].default
            in [None, inspect._empty]  # pylint: disable=protected-access
        ):
            kwargs["help"] = "(default: %(default)s)"
            kwargs["default"] = sig.parameters[arg].default

        if sig.parameters[arg].default is True:
            args[0] = "--" + args[0]
            kwargs["action"] = "store_false"
        elif sig.parameters[arg].default is False:
            args[0] = "--" + args[0]
            kwargs["action"] = "store_true"
        elif sig.parameters[arg].annotation == Optional[Dict[str, str]]:
            args[0] = "--" + args[0]
            kwargs["action"] = AsDict
            kwargs["nargs"] = "+"
            kwargs["metavar"] = "key=val"
        elif sig.parameters[arg].annotation == Optional[Directory]:
            args[0] = "--" + args[0]
            kwargs["type"] = arg_dir
        elif sig.parameters[arg].annotation == Directory:
            kwargs["type"] = arg_dir
        elif sig.parameters[arg].annotation == Optional[File]:
            args[0] = "--" + args[0]
            kwargs["type"] = arg_file
        elif sig.parameters[arg].annotation == File:
            kwargs["type"] = arg_file
        elif sig.parameters[arg].annotation == Optional[bool]:
            args[0] = "--" + args[0]
            kwargs["type"] = arg_bool
            kwargs["choices"] = [True, False]
        elif sig.parameters[arg].annotation == Optional[str]:
            args[0] = "--" + args[0]
            kwargs["type"] = str
        elif sig.parameters[arg].annotation == str:
            kwargs["type"] = str
        elif sig.parameters[arg].annotation == Optional[int]:
            args[0] = "--" + args[0]
            kwargs["type"] = int
        elif sig.parameters[arg].annotation == int:
            kwargs["type"] = int
        elif sig.parameters[arg].annotation == Optional[List[str]]:
            args[0] = "--" + args[0]
            kwargs["nargs"] = "*"
        elif sig.parameters[arg].annotation == List[str]:
            kwargs["nargs"] = "*"
        else:
            raise NotImplementedError("unsupported argument type: %s" % arg)

        parser.add_argument(*args, **kwargs)


def add_base(parser):
    """ add basic arguments that should always be available """

    parser.add_argument(
        "-v", "--verbose", action="count", help="increase output verbosity", default=0
    )
    parser.add_argument(
        "--format", choices=["json", "raw"], default="json", help="output format"
    )
    parser.add_argument("--query", help=JMES_HELP)


def get_help_text(impl):
    """ get the help text based on pydoc strings """

    text = impl.__doc__ or ""
    return text.strip().split("\n")[0].strip()


def add_funcs(inst, subparser):
    """ Add functions at the implementation layer """

    funcs = [
        func
        for func in dir(inst)
        if callable(getattr(inst, func)) and not func.startswith("_")
    ]

    for func in funcs:
        impl = getattr(inst, func)
        sub = subparser.add_parser(func, help=get_help_text(impl))
        add_base(sub)
        add_func_args(sub, impl)
        sub.set_defaults(func=impl)


def add_nested(main_parser, parent_parser, inst, api_types, level=0):
    """ Recurse through objects in a given class instance as argparse subcommands """

    endpoint_subparsers = main_parser.add_subparsers(
        title="subcommands", dest="level_%d" % level
    )

    for api_type in api_types:
        endpoints = [
            func for func in dir(inst) if isinstance(getattr(inst, func), api_type)
        ]
        for endpoint in endpoints:
            endpoint_impl = getattr(inst, endpoint)

            endpoint_parser = endpoint_subparsers.add_parser(
                endpoint, help=get_help_text(endpoint_impl), parents=[parent_parser]
            )

            method_subparser = endpoint_parser.add_subparsers(
                title="subcommands", dest="level_%d" % (level + 1)
            )

            nested_endpoints = [
                x
                for x in dir(endpoint_impl)
                if isinstance(getattr(endpoint_impl, x), api_type)
            ]
            for nested_endpoint in nested_endpoints:
                method_impl = getattr(endpoint_impl, nested_endpoint)
                nested = method_subparser.add_parser(
                    nested_endpoint,
                    help=get_help_text(method_impl),
                    parents=[parent_parser],
                )
                add_nested(
                    nested, parent_parser, method_impl, api_types, level=level + 2
                )

            methods = [
                x
                for x in dir(endpoint_impl)
                if callable(getattr(endpoint_impl, x)) and not x.startswith("_")
            ]
            for method in methods:
                method_impl = getattr(endpoint_impl, method)
                method_parser = method_subparser.add_parser(
                    method, help=get_help_text(method_impl), parents=[parent_parser]
                )
                add_func_args(method_parser, method_impl)
                method_parser.set_defaults(func=method_impl)

    add_funcs(inst, endpoint_subparsers)


def build_arg_parser(inst, api_types, version):
    """ Top level argparse creation """

    parent_parser = argparse.ArgumentParser(add_help=False)
    add_base(parent_parser)

    main_parser = argparse.ArgumentParser()
    add_base(main_parser)
    main_parser.add_argument(
        "--version",
        action="version",
        version="%(prog)s {version}".format(version=version),
    )

    add_nested(main_parser, parent_parser, inst, api_types)

    return main_parser


def set_logging(api, verbose):
    """ Set log verbosity """

    if verbose == 0:
        logging.basicConfig(level=logging.WARNING)
        api.logger.setLevel(logging.INFO)
    elif verbose == 1:
        logging.basicConfig(level=logging.WARNING)
        api.logger.setLevel(logging.INFO)
        logging.getLogger("nsv-backend").setLevel(logging.DEBUG)
    elif verbose == 2:
        logging.basicConfig(level=logging.INFO)
        api.logger.setLevel(logging.DEBUG)
        logging.getLogger("nsv-backend").setLevel(logging.DEBUG)
    elif verbose >= 3:
        logging.basicConfig(level=logging.DEBUG)
        api.logger.setLevel(logging.DEBUG)


def print_help(parser, args):
    """ find the appropriate help from subparsers """

    level = 0
    while True:
        choices = parser._subparsers._actions[  # pylint: disable=protected-access
            -1
        ].choices
        value = getattr(args, "level_%d" % level)
        if value is None:
            parser.print_help()
            return 1

        parser = choices[value]
        level += 1


def execute_api(api, api_types, version):  # pylint: disable=unused-variable
    """ Expose an API via a light-weight CLI """

    parser = build_arg_parser(api, api_types, version)
    args = parser.parse_args()

    set_logging(api, args.verbose)

    if not hasattr(args, "func"):
        LOGGER.error("no command specified")
        print_help(parser, args)
        return 1

    if args.query:
        try:
            expression = jmespath.compile(args.query)
        except jmespath.exceptions.ParseError as err:
            LOGGER.error("unable to parse query: %s", err)
            return 1
    else:
        expression = None

    try:
        result = call_func(args.func, args)
    except Exception as err:  # pylint: disable=broad-except
        LOGGER.error("command failed: %s", " ".join(err.args))
        return 1

    if isinstance(result, bytes):
        sys.stdout.buffer.write(result)
    else:
        if expression is not None:
            result = expression.search(result)
        if result is not None:
            if args.format == "json":
                print(json.dumps(result, indent=4, sort_keys=True))
            else:
                print(result)

    return 0
