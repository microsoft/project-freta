# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

"""
Command line interface to the Freta volatile memory inspection service.
"""

import sys
from freta.__version__ import __version__
from freta.cli import execute_api
from freta.api import Freta, Endpoint


def main():
    """ Build & Execute Freta CLI """
    return execute_api(Freta(), [Endpoint], __version__)


if __name__ == "__main__":
    sys.exit(main())
