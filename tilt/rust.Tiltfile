load('ext://restart_process', 'docker_build_with_restart')

__DEV_BASE_IMAGE = "ubuntu:hirsute"
__DEBUG = False


def rust_local_binary(*, crate_path, additional_dependencies = []):
    """
    Compiles a Rust binary using the cargo toolchain installed on the local computer.
    Always run using the dev profile (non-`--release`).

    Arguments:
    - `crate_path` - the local path (from the root of the repo)
      to the crate parent folder (where the `Cargo.toml` file is located).
    - `additional_dependencies` - additional file dependencies
    """

    abs_crate_manifest_path = os.path.abspath(os.path.join(crate_path, 'Cargo.toml'))

    # Set up logging
    log = lambda m: print('[rust--%s] %s' % (crate_path, m))

    # Get the crate's metadata using `cargo metadata`
    metadata = __crate_metadata(crate_path=crate_path)

    # Search through the list of packages, looking for both:
    # - the current crate (to find its name)
    # - all local crate dependencies for the current crate
    #   (this is defined as crates with manifest paths
    #   starting at the Tiltfile parent directory's path
    #   (that are also not the current crate))
    current_crate_metadata = None
    local_crate_dependencies = []
    log("Detecting local dependencies:")
    for package in metadata["packages"]:
        manifest_path = package["manifest_path"]
        if abs_crate_manifest_path == manifest_path:
            current_crate_metadata = package
        elif manifest_path.startswith(config.main_dir):
            relative_package_path = os.path.relpath(os.path.dirname(manifest_path))
            local_crate_dependencies.append(relative_package_path)
            log(' => %s (%s)' % (package["name"], relative_package_path))
    if not current_crate_metadata:
        fail('%s Could not find current crate\'s package entry in $(cargo metadata)')
    if not local_crate_dependencies:
        log(' (None)')

    # Compile the crate with the inferred and explicit dependencies
    local_resource(
        name='%s-compile' % crate_path.replace('/', '-'),
        cmd=[
            'cargo',
            'build',
            '--manifest-path=%s/Cargo.toml' % crate_path,
        ],
        deps=__collect_crate_dependencies(
            crate_path,
            local_crate_dependencies,
            additional_dependencies,
        ),
    )

    # Infer the expected path of the binary file
    target_dir = metadata['target_directory']
    binary_filename = current_crate_metadata["name"]
    expected_binary_path = os.path.relpath(os.path.join(target_dir, 'debug', binary_filename))
    log('Expected binary path is: %s' % expected_binary_path)
    return expected_binary_path


def __crate_metadata(*, crate_path):
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


def __binary_path(*, target_dir):
    """
    Finds the local path of an already-compiled Rust binary crate.
    Only works on Unix-like systems
    (please stop using Windows for development).
    """

    find_output = str(local(
        command=[
            'find',
            target_dir,
            '-maxdepth', '2',
            '-type', 'f',
            '-executable',
            '-print',
        ],
        quiet=True,
        echo_off=not __DEBUG,
    ))
    executable_files = find_output.rstrip('\n').split('\n')

    if not executable_files:
        return None
    return executable_files[0]


def rust_file_sync(local_path="", container_path=""):
    """
    Creates an additional sync descriptor for use with
    `rust_hot_reload_docker_build`'s `file_syncs` parameter.
    """

    return struct(local_path = local_path, container_path = container_path)


def rust_hot_reload_docker_build(*, ref, binary_path, apt_packages=[], file_syncs=[],
    additional_arguments=[]):

    """
    Adds an existing local Rust binary to a simple Dockerfile
    that uses the `docker_build_with_restart` extension
    to hot-reload the container in-place whenever the binary
    (or other sync file) changes.

    To supply additional file syncs,
    provide a list of `rust_file_sync` instances in the `file_syncs` parameter.
    To supply additional arguments to the service command,
    use `additional_arguments`.
    """

    binary_path_filename = os.path.basename(binary_path)
    binary_path_in_container = '/usr/bin/%s' % binary_path_filename

    deps = [binary_path]
    live_update = [sync(binary_path, binary_path_in_container)]

    for file_sync in file_syncs:
        deps.append(file_sync.local_path)
        live_update.append(sync(file_sync.local_path, file_sync.container_path))

    all_file_syncs = [rust_file_sync(
        local_path=binary_path,
        container_path=binary_path_in_container,
    )]
    all_file_syncs.extend(file_syncs)

    # The dockerfile is formulaic, so just construct it here
    dockerfile_lines = __create_hot_reload_dockerfile_lines(
        file_syncs=all_file_syncs,
        apt_packages=apt_packages,
    )

    arguments = [binary_path_in_container]
    arguments.extend(additional_arguments)

    docker_build_with_restart(
        ref=ref,
        context='.',
        dockerfile_contents='\n'.join(dockerfile_lines),
        only=deps,
        entrypoint=arguments,
        live_update=live_update,
    )


def __create_hot_reload_dockerfile_lines(*, apt_packages, file_syncs):
    dockerfile = ['FROM %s' % __DEV_BASE_IMAGE]
    # Include all specified apt packages
    if apt_packages:
        dockerfile.append('ENV DEBIAN_FRONTEND=noninteractive')
        dockerfile.append('RUN ' + ' && '.join([
            'apt-get update -q',
            'apt-get install -y -q %s' % ' '.join(['"%s"' % p for p in apt_packages]),
            'rm -rf /var/lib/apt/lists/*',
        ]),)
    # Add a COPY line for each file sync
    for file_sync in file_syncs:
        dockerfile.append('COPY %s %s' % (file_sync.local_path, file_sync.container_path))
    # Run the main binary with `RUST_BACKTRACE` so that panic backtraces are displayed
    dockerfile.append('ENV RUST_BACKTRACE=1')

    return dockerfile


def __collect_crate_dependencies(crate_path, local_crate_dependencies, additional_dependencies):
    # Over-including dependencies
    # (such as including a build.rs file when there is none)
    # is okay, so we just over-include them instead of ensuring they exist.
    deps = __crate_dependencies(crate_path)
    for local_crate_dependency in local_crate_dependencies:
        deps.extend(__crate_dependencies(local_crate_dependency))
    deps.extend(additional_dependencies)

    return deps


def __crate_dependencies(crate_path):
    return [
        '%s/Cargo.toml' % crate_path,
        '%s/Cargo.lock' % crate_path,
        '%s/build.rs' % crate_path,
        '%s/src' % crate_path,
    ]

