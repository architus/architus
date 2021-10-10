load('ext://restart_process', 'docker_build_with_restart')

__DEV_BASE_IMAGE = "ubuntu:hirsute"


def rust_local_binary(*, crate_path, local_crate_dependencies = [], additional_dependencies = []):
    """
    Compiles a Rust binary using the cargo toolchain installed on the local computer.
    Always run using the dev profile (non-`--release`).

    Arguments:
    - `crate_path` - the local path (from the root of the repo)
      to the crate parent folder (where the `Cargo.toml` file is located).
    - `local_crate_dependencies` - additional crate paths for local crates
      that the main crate depends on. These are included as additional dependencies
      for the build.
    - `additional_dependencies` - additional file dependencies
    """

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

    return rust_local_binary_path(crate_path=crate_path)


def rust_local_binary_path(*, crate_path):
    """
    Finds the local path of an already-compiled Rust binary crate.
    Only works on Unix-like systems
    (please stop using Windows for development).
    """

    metadata = decode_json(local(
        command=[
            'cargo',
            'metadata',
            '--manifest-path=%s/Cargo.toml' % crate_path,
            '--format-version=1',
        ],
        quiet=True,
    ))
    target_dir = metadata['target_directory']
    executable_files = str(local(command=[
        'find',
        target_dir,
        '-maxdepth', '2',
        '-type', 'f',
        '-executable',
        '-print',
    ])).rstrip('\n').split('\n')

    if not executable_files:
        fail('no executable found in Rust crate path "%s"' % crate_path)
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

    binary_path = os.path.relpath(binary_path)
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

