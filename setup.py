#!/usr/bin/env python

# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

import setuptools
import sys


def get_version():
    freta = {}
    with open("freta/__version__.py") as fh:
        exec(fh.read(), freta)
    version = freta["__version__"]
    if "-v" in sys.argv:
        index = sys.argv.index("-v")
        sys.argv.pop(index)
        version += ".dev" + sys.argv.pop(index)
    return version


with open("requirements.txt") as f:
    requirements = f.read().splitlines()

setuptools.setup(
    name="freta",
    version=get_version(),
    description="Freta Client Library for Python",
    url="https://dev.azure.com/msresearch/Freta",
    author="Project Freta",
    author_email="project-freta@microsoft.com",
    license="MIT",
    packages=["freta"],
    entry_points={"console_scripts": ["freta = freta.__main__:main"]},
    install_requires=requirements,
    zip_safe=False,
)
