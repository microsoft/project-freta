.. vim: set sw=4 ts=4 et ai ff=unix:

Bulk Upload Client
==================

Summary
-------

`freta` is a command line utility supporting scriptable bulk upload to the Freta volatile
memory image inspection service.

Interactive device authentication, similar to that of `az login`_,  will be requested
as needed. The resulting access token will be cached in the user's home directory.

.. _az login: https://docs.microsoft.com/en-us/cli/azure/authenticate-azure-cli?view=azure-cli-latest

.. code::

    usage: freta [-h] [-v] [--format {json,raw}] [--query QUERY] [--version]
                {image,config,logout,regions,versions} ...

    optional arguments:
    -h, --help            show this help message and exit
    -v, --verbose         increase output verbosity
    --format {json,raw}   output format
    --query QUERY         JMESPath query string. See http://jmespath.org/ for
                            more information and examples.
    --version             show program's version number and exit

    subcommands:
    {image,config,logout,regions,versions}
        image               Interact with Images
        config              Configure Freta CLI
        logout              Forget any saved access token. Deletes the
                            `token_path` file.
        regions             Get the list of Azure regions currently supported by
                            Freta for image storage and analysis.
        versions            Get the versions of various Freta components.

logout
-----------
Delete any saved Freta access token.

Example:

   $ freta logout

image list
-----------
Lists images and their statuses.

Optional argument:

      --search_filter     Filter search results

Run ``freta image search_filters`` to get the currently supported search filters.

Example output:

.. code:: Json

   [
       {
           "Timestamp": "2019-05-13 18:50:01",
           "analysis_version": "0.0.0",
           "failure_code": "",
           "image_id": "11111111-1111-1111-1111-111111111111",
           "image_type": "lime",
           "machine_id": "ubuntu-16.04-4.15.0-1040-azure",
           "owner_id": "00000000-0000-0000-0000-000000000000",
           "region": "eastus",
           "state": "Report available"
       }
   ]

.. _Freta Portal: https://freta.azurewebsites.net/

image status
------
Get the status of a single image.

    freta image status 11111111-1111-1111-1111-111111111111

Example output:

.. code:: Json

    {
        "Timestamp": "2019-06-21 03:43:45",
        "analysis_version": "0.0.0",
        "failure_code": "",
        "image_id": "11111111-1111-1111-1111-111111111111",
        "image_type": "raw",
        "machine_id": "example-01",
        "owner_id": "00000000-0000-0000-0000-000000000000",
        "region": "westus2",
        "state": "Report ready"
    }

image search_filters
--------------
Get the available search filters supported by ``freta list``.

    freta search_filters

Example output:

.. code:: Json

    [
        "my_images",
        "my_images_and_samples",
    ]

image upload
------

Upload an image file and queue it for analysis. Outputs the *image_id*.

    usage: freta image upload [-h] [--profile PROFILE] file format region name

Arguments:

     | ``name``    name of the image
     | ``format``  Format of the image (see 'formats' for allowed values)
     | ``region``  Region in which to store and process the image.
     | ``file``    Path to image file

Optional argument:

      --profile PROFILE  kernel profile

Example:

.. code::

   $ freta image upload 'example memory image' raw westus2  ~/example/image.raw --profile ~/example/kernel.profile
   {
       "image_id": "11111111-1111-1111-1111-111111111111",
       "owner_id": "00000000-0000-0000-0000-000000000000"
   }


image upload_sas
----------
Obtain SAS URIs authorizing (only) upload of an image and a profile.

This does not queue the image for analysis. Invoke the *analyze* command
with the returned *image_id* after writing the image data.

    usage: freta image upload-sas [-h] format region name

Use the resulting image.sas_url with an azure blob store utility, such as azcopy, to upload the image.

Arguments:

 | ``name``        Name of the image
 | ``format``      Format of the image (see 'formats' for allowed values)
 | ``region``      Region in which to store and process the image

Example:

.. code::

  $ freta image upload_sas 'example memory image' lime westus2 
  {
      "image_id": "11111111-1111-1111-1111-111111111111",
      "image": {
          "sas_url": "https://IMAGE_SAS_URL_HERE/...",
      },
      "profile": {
          "sas_url": "https://PROFILE_SAS_URL_HERE/..."
      },
  }

  $ azcopy copy ./path/to/file.lime "https://IMAGE_SAS_URL_HERE/..."
  ...
  $ freta image analyze 11111111-1111-1111-1111-111111111111

image update
------

Update metadata for an image.

Metadata fields currently available:

  name          Name of the image

  usage: freta image update
            [--owner-id OWNER_ID]
            [--name NAME]
            image_id

Argument:

   | ``image_id`` IMAGE_ID   Value returned by 'upload' or 'list'

Example:

.. code::

  $ freta image status 11111111-1111-1111-1111-111111111111
  {
    ...
    "machine_id": "[TESTING OK TO DELETE] example-01",
    ...
  }

  $ freta image update 11111111-1111-1111-1111-111111111111 --name 'new name'

  $ freta image status 11111111-1111-1111-1111-111111111111
  {
    ...
    "machine_id": "new name",
    ...
  }

image analyze
-------
Analyze (or re-analyze) an uploaded image.

Arguments:

    | ``image_id`` The *image_id* value returned by 'upload' or 'list' commands
    | ``format``   Format of the image (see 'formats' for allowed values)

Optional argument:

    --owner-id OWNER_ID  For group owned images, the user_id of the image owner.

Example:

.. code::

  $ freta image analyze 11111111-1111-1111-1111-111111111111 && echo ok
  ok

image cancel_analysis
-------
Cancel analysis of an image

Arguments:

    | ``image_id`` The *image_id* value returned by 'upload' or 'list' commands
    | ``format``   Format of the image (see 'formats' for allowed values)

Optional argument:

    --owner-id OWNER_ID  For group owned images, the user_id of the image owner.

Example:

.. code::

  $ freta image cancel_analysis 11111111-1111-1111-1111-111111111111 && echo ok
  ok

image artifact get
--------
Download an artifact related to the image.

Arguments:

   | ``image_id``   The *image_id* value returned by 'upload' or 'list'
   | ``filename``   Name of the artifact file, e.g. 'report.json'

Optional argument:

    --owner-id OWNER_ID  For group owned images, the user_id of the image owner.

Outputs artifact file content, which may be text or binary.

Example:

.. code::

  $ freta image artifact get 11111111-1111-1111-1111-111111111111 report.json
  {
      [report content]
  }

image artifact list
--------
List available artifacts

Arguments:

   | ``image_id``   The *image_id* value returned by 'upload' or 'list'

Optional argument:

    --owner-id OWNER_ID  For group owned images, the user_id of the image owner.

List available artifacts for an image

Example:

.. code::

  $ freta image artifact list 11111111-1111-1111-1111-111111111111 
  [
      ...
      "image.lime",
      "report.json"
  ]

image delete
------
Delete an image along with any of its generated reports and other artifacts.

Arguments:

 | ``image_id``   *image_id* value returned by 'upload' or 'list'

Optional argument:

  --owner-id OWNER_ID  For group owned images, the user_id of the image owner.

Example:

.. code::

    $ freta image delete 11111111-1111-1111-1111-111111111111 && echo ok
    ok

formats
-------
List the currently supported image formats.

Example:

.. code::

    $ freta formats
    {
        "lime": "LiME image",
        "raw": "Raw Physical Memory Dump",
        "vmrs": "Hyper-V Memory Snapshot"
    }

regions
-------
List the Azure regions currently supported by Freta for image storage and analysis.

Example output:

.. code::

    $ freta regions
    {
        "australiaeast": {
            "default": false,
            "name": "Australia East"
        },

        [...more...]

        "westus2": {
            "default": false,
            "name": "West US 2"
        }
    }

versions
-------
List the Freta component versions.

Example output:

.. code::

    $ freta versions
    {
        "analysis": "0.1.4"
    }

