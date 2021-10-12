__DEBUG = False
__CRATE_POSSIBLE_DEPS = ["Cargo.toml", "Cargo.lock", "build.rs", "src"]


def rust_local_binary(*, crate_path, additional_dependencies = []):
    """
    Compiles a Rust binary using the cargo toolchain installed on the local computer.
    Always run using the dev profile (non-`--release`).

    Arguments:
    - `crate_path` - the local path (from the root of the repo)
      to the current crate's parent folder (where the `Cargo.toml` file is located).
    - `additional_dependencies` - additional file dependencies
      for the crate's local compilation resource

    Returns:
    The expected path that the binary should reside at once compiled.
    """

    # Set up logging
    log = lambda m: print('[rust--%s] %s' % (crate_path, m))
    log_fail = lambda m: fail('[rust--%s] %s' % (crate_path, m))

    # Get the crate's metadata using `cargo metadata`
    metadata = __crate_metadata(crate_path=crate_path)

    # Search through the list of packages, looking for both:
    # - the current crate's metadata;
    #   this contains info about the current crate that for some reason
    #   isn't contained in the root-level metadata struct,
    #   such as the crate's actual name.
    # - all local crate dependencies for the current crate;
    #   specifically, their metadata structs.
    #   These are used to infer the file dependencies
    #   for the local compilation resource.

    local_crates = __find_local_crates(crate_path=crate_path, packages=metadata["packages"])
    log("Detected local dependencies:")
    if local_crates:
        for local_crate in local_crates:
            log(' => %s (%s)' % (local_crate["name"], __crate_relative_path(local_crate)))
    else:
        log(' (None)')

    current_crate_metadata = __find_current_crate(crate_path=crate_path, packages=metadata["packages"])
    if not current_crate_metadata:
        log_fail("Could not find current crate's package entry in $(cargo metadata)")

    # Compile the crate with the inferred and explicit dependencies
    local_resource(
        name='%s-compile' % crate_path.replace('/', '-'),
        cmd=[
            'cargo',
            'build',
            '--manifest-path=%s' % os.path.join(crate_path, 'Cargo.toml'),
        ],
        deps=__collect_crate_dependencies(
            crate_path=crate_path,
            local_crate_dependencies=[__crate_relative_path(c) for c in local_crates],
            additional_dependencies=additional_dependencies,
        ),
    )

    # Infer the expected path of the binary file
    target_dir = metadata['target_directory']
    binary_filename = current_crate_metadata["name"]
    expected_binary_path = os.path.relpath(os.path.join(target_dir, 'debug', binary_filename))
    log('Expected binary path is: %s' % expected_binary_path)
    return expected_binary_path


def __crate_metadata(*, crate_path):
    """
    Runs `cargo metadata` to obtain the JSON-formatted root-level metadata
    for the given crate. This contains information about all dependencies,
    as well as the target directory of the current crate.

    Arguments:
    - `crate_path` - the local path (from the root of the repo)
      to the current crate's parent folder (where the `Cargo.toml` file is located).

    Returns:
    The parsed JSON from the output of the command.
    """

    return decode_json(local(
        command=[
            'cargo',
            'metadata',
            '--manifest-path=%s' % os.path.join(crate_path, 'Cargo.toml'),
            '--format-version=1',
        ],
        quiet=True,
        echo_off=not __DEBUG,
    ))


def __find_current_crate(*, crate_path, packages):
    """
    Finds the cargo package metadata for the current crate.
    This contains info about the current crate that for some reason
    isn't contained in the root-level metadata struct,
    such as the crate's actual name.

    Arguments:
    - `crate_path` - the local path (from the root of the repo)
      to the current crate's parent folder (where the `Cargo.toml` file is located).
    - `packages` - the `.packages` field of the cargo metadata JSON result.

    Returns:
    The package metadata struct for the current crate,
    or `None` if it wasn't found.
    """

    abs_crate_manifest_path = os.path.abspath(os.path.join(crate_path, 'Cargo.toml'))
    for package in packages:
        if abs_crate_manifest_path == package["manifest_path"]:
            return package
    return None


def __find_local_crates(*, crate_path, packages):
    """
    Finds all local crates:
    this is defined as crates with manifest paths
    starting at the Tiltfile parent directory's path
    (that are also not the current crate).

    Arguments:
    - `crate_path` - the local path (from the root of the repo)
      to the current crate's parent folder (where the `Cargo.toml` file is located).
    - `packages` - the `.packages` field of the cargo metadata JSON result.

    Returns:
    A list of package metadata structs for each local dependency.
    """

    abs_crate_manifest_path = os.path.abspath(os.path.join(crate_path, 'Cargo.toml'))
    local_crate_dependencies = []
    for package in packages:
        manifest_path = package["manifest_path"]
        if abs_crate_manifest_path != manifest_path and manifest_path.startswith(config.main_dir):
            local_crate_dependencies.append(package)
    return local_crate_dependencies


def __crate_relative_path(crate_metadata):
    """
    Finds the repository-relative path for the given crate
    by reading its package metadata struct.

    Arguments:
    - `crate_metadata` - the package metadata struct for the crate in question.

    Returns:
    A relative path (as a string) for the given crate.
    """

    crate_root = os.path.dirname(crate_metadata["manifest_path"])
    return os.path.relpath(crate_root)


def __collect_crate_dependencies(*, crate_path, local_crate_dependencies, additional_dependencies):
    """
    Creates a list of all possible dependencies for a crate,
    including the list of local crate dependencies
    and any other explicit file dependencies as specified.

    Arguments:
    - `crate_path` - the local path (from the root of the repo)
      to the current crate's parent folder (where the `Cargo.toml` file is located).
    - `local_crate_dependencies` - a list of local paths
      to the parent folders of any other local crates that the current one depends on
      for the crate's local compilation resource
    - `additional_dependencies` - additional file dependencies
      for the crate's local compilation resource

    Returns:
    A list of paths that should be file or folder dependencies
    for the crate's local compilation resource.
    """

    # Over-including dependencies
    # (such as including a build.rs file when there is none)
    # is okay, so we just over-include them instead of ensuring they exist.
    deps = __crate_dependencies(crate_path=crate_path)
    for local_crate_dependency in local_crate_dependencies:
        deps.extend(__crate_dependencies(crate_path=local_crate_dependency))
    deps.extend(additional_dependencies)

    return deps


def __crate_dependencies(*, crate_path):
    """
    Arguments:
    - `crate_path` - the local path (from the root of the repo)
      to a crate's parent folder (where the `Cargo.toml` file is located).

    Returns:
    A list of paths that should be file or folder dependencies
    for the crate or any dependent crate's local compilation resource
    """

    return [os.path.join(crate_path, p) for p in __CRATE_POSSIBLE_DEPS]
