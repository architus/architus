__DEFAULT_EXAMPLE_SUFFIX = '.example'


def copy_example(*, path, example_path=None):
    """
    Ensures that the file exists at the given path
    by copying the 'example' file that should also exist
    at example_path, which by default is the same path
    with '.example' at the end.
    """

    if not example_path:
        example_path = path + __DEFAULT_EXAMPLE_SUFFIX

    # Ensure that the file exists; otherwise copy it from the example
    if not os.path.exists(path):
        local(['cp', example_path, path])
