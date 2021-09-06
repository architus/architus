__EXAMPLE_SUFFIX = '.example'


def copy_example(*, path):
    """
    Ensures that the file exists at the given path
    by copying the 'example' file that should also exist
    at the same path with '.example' at the end.
    """

    # Ensure that the file exists; otherwise copy it from the example
    if not os.path.exists(path):
        local(['cp', path + __EXAMPLE_SUFFIX, path])
