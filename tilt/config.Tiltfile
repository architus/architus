__DEFAULT_EXAMPLE_SUFFIX = '.example'


def copy_example(*, path, example_path=None):
    """
    Ensures that the file exists at the given path
    by copying the 'example' file that should also exist
    at example_path, which by default is the same path
    with '.example' at the end.

    Arguments:
    - `path` - the path of the file that should exist;
    otherwise its example will be copied to this location.
    - `example_path` - an override of the path of the example file.
    If this is `None` or not given, then the example file
    will be assumed to reside at `${path}.example`.
    """

    if not example_path:
        example_path = path + __DEFAULT_EXAMPLE_SUFFIX

    # Ensure that the file exists; otherwise copy it from the example
    if not os.path.exists(path):
        local(['cp', example_path, path])
