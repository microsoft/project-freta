# Copyright (c) Microsoft Corporation. All rights reserved.
# Licensed under the MIT License.

"""
Custom typing wrappers for use in auto-cli generation
"""

# pylint: disable=unused-variable

from typing import NewType

File = NewType("File", str)
Directory = NewType("Directory", str)
