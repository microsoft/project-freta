# Project Freta

## Summary

The `Freta library for Python` enables access to [Project Freta](https://freta.azurewebsites.net), a service used to inspect volatile memory images.

Included in this library is a utility, `freta`, which provides command line access to the [Project Freta](https://freta.azurewebsites.net) service.

## Prerequisites

Python 3.6 ad 3.7 are fully supported and tested.  Other versions may work, but are untested.


## Installing
To install, use `pip`

```bash
pip install .
```

## Using the Client

```
$ freta image formats
Please login
To sign in, use a web browser to open the page https://microsoft.com/devicelogin and enter the code XXXXXXXX to authenticate.
Login succeeded
{
    "lime": "LiME image",
    "raw": "Raw Physical Memory Dump",
    "vmrs": "Hyper-V Memory Snapshot"
}
$ freta image upload "my first image" lime eastus ./image.lime
{
    "image_id": "11111111-1111-1111-1111-111111111111",
    "owner_id": "00000000-0000-0000-0000-000000000000"
}
$
```

## Using the API

```python
import json
from freta import Freta

freta = Freta()
response = freta.formats():
print(json.dumps(response, indent=4, sort_keys=True))
response = freta.image.upload("my first image", "lime", "eastus", "./image.lime")
print(json.dumps(response, indent=4, sort_keys=True))
```

## Local Development

Development within a virtual environment is recommended

    # setup virtual environment
    python3 -m venv ~/freta-venv
    source ~/freta-venv/bin/activate

    # install dev prereqs
    python -m pip install -r requirements-dev.txt
    python -m pip install -e .

## Testing

As provided from source, tests are verified against pre-recorded API interactins recorded using [pyvcr](https://pypi.org/project/vcrpy/).

    # Run unit tests
    pytest

To test against the live API:

    # Delete recorded API sessions
    rm tests/fixtures/*
    
    # Use the API to ensure you're logged in
    freta regions

    # Run unit tests
    pytest

## Versions

This library follows [Semantic Versioning](http://semver.org/).

# Contributing

This project welcomes contributions and suggestions.  Most contributions require you to agree to a
Contributor License Agreement (CLA) declaring that you have the right to, and actually do, grant us
the rights to use your contribution. For details, visit https://cla.opensource.microsoft.com.

When you submit a pull request, a CLA bot will automatically determine whether you need to provide
a CLA and decorate the PR appropriately (e.g., status check, comment). Simply follow the instructions
provided by the bot. You will only need to do this once across all repos using our CLA.

This project has adopted the [Microsoft Open Source Code of Conduct](https://opensource.microsoft.com/codeofconduct/).
For more information see the [Code of Conduct FAQ](https://opensource.microsoft.com/codeofconduct/faq/) or
contact [opencode@microsoft.com](mailto:opencode@microsoft.com) with any additional questions or comments.

# Legal Notices

Microsoft and any contributors grant you a license to the Microsoft documentation and other content
in this repository under the [Creative Commons Attribution 4.0 International Public License](https://creativecommons.org/licenses/by/4.0/legalcode),
see the [LICENSE](LICENSE) file, and grant you a license to any code in the repository under the [MIT License](https://opensource.org/licenses/MIT), see the
[LICENSE-CODE](LICENSE-CODE) file.

Microsoft, Windows, Microsoft Azure and/or other Microsoft products and services referenced in the documentation
may be either trademarks or registered trademarks of Microsoft in the United States and/or other countries.
The licenses for this project do not grant you rights to use any Microsoft names, logos, or trademarks.
Microsoft's general trademark guidelines can be found at http://go.microsoft.com/fwlink/?LinkID=254653.

Privacy information can be found at https://privacy.microsoft.com/en-us/

Microsoft and any contributors reserve all other rights, whether under their respective copyrights, patents,
or trademarks, whether by implication, estoppel or otherwise.
