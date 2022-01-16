load('ext://restart_process', 'docker_build_with_restart')

__DEV_BASE_IMAGE = "ubuntu:hirsute"


def file_sync(local_path="", container_path=""):
    """
    Creates an additional sync descriptor for use with `hot_reload_docker_build`.

    Arguments:
    - `local_path` - the local path (relative or absolute)
      to the source file that should be synced to the container.
    - `container_path` - the absolute path
      that the file should exist at in the container.

    Returns:
    A struct to pass to `hot_reload_docker_build`'s `file_syncs` parameter.
    """

    return struct(local_path = local_path, container_path = container_path)


def rust_hot_reload_docker_build(*,
    env={},
    **kwargs):
    """
    Runs an existing local Rust binary in a simple Dockerfile
    that uses the `docker_build_with_restart` extension
    to hot-reload the container in-place whenever the binary
    (or other sync file) changes.

    Always adds `RUST_BACKTRACE` when the binary is run
    to ensure that panic backtraces are displayed.

    Arguments:
    - `env` - a dictionary of (key, value) pairs
      that will be added as environment variables when the binary is executed.
    - `**kwargs` - will be passed to the underlying `hot_reload_docker_build` call
    """

    # Copy the env and then add `RUST_BACKTRACE` so that panic backtraces are displayed
    env_copy = {k: env[k] for k in env}
    env_copy["RUST_BACKTRACE"] = "1"

    hot_reload_docker_build(
        env=env_copy,
        **kwargs,
    )


def hot_reload_docker_build(*,
    ref,
    binary_path,
    apt_packages=[],
    file_syncs=[],
    arguments=[],
    base_image=__DEV_BASE_IMAGE,
    env={}):
    """
    Runs an existing local binary in a simple Dockerfile
    that uses the `docker_build_with_restart` extension
    to hot-reload the container in-place whenever the binary
    (or other sync file) changes.

    Arguments:
    - `ref` - name for this image (e.g. ‘myproj/backend’ or ‘myregistry/myproj/backend’).
      If this image will be used in a k8s resource(s),
      this ref must match the spec.container.image param for that resource(s).
    - `binary_path` - path (relative or absolute) to the binary
      that will be synced into the container and executed as the entrypoint.
    - `apt_packages` - list of apt package names that will be installed in the container
    - `file_syncs` - list of `file_sync(...)`-produced structs
      that describe additional local files that should be added to
      and synced in the container
    - `arguments` - a list of arguments to the binary when it is run
    - `base_image` - the name of the Docker image to use as the base
    - `env` - a dictionary of (key, value) pairs
      that will be added as environment variables when the binary is executed
    """

    # The binary is always synced, so add it to an 'all_file_syncs' list
    binary_path_filename = os.path.basename(binary_path)
    binary_path_in_container = os.path.join('/usr/bin', binary_path_filename)
    all_file_syncs = [file_sync(local_path=binary_path, container_path=binary_path_in_container)]
    all_file_syncs.extend(file_syncs)

    # The dockerfile is formulaic, so just construct it here
    dockerfile_lines = __create_hot_reload_dockerfile_lines(
        base_image=base_image,
        file_syncs=all_file_syncs,
        apt_packages=apt_packages,
        env=env,
    )

    # The entrypoint command should be in the format:
    # /usr/bin/[binary_name] [...arguments]
    entrypoint = [binary_path_in_container]
    entrypoint.extend(arguments)

    docker_build_with_restart(
        ref=ref,
        context='.',
        dockerfile_contents='\n'.join(dockerfile_lines),
        only=[s.local_path for s in all_file_syncs],
        entrypoint=entrypoint,
        live_update=[sync(s.local_path, s.container_path) for s in all_file_syncs],
    )


def __create_hot_reload_dockerfile_lines(*, base_image, file_syncs, apt_packages, env):
    """
    Creates the formulaic Dockerfile used to set up the hot reload Docker container.

    Arguments:
    - `apt_packages` - list of apt package names that will be installed in the container
    - `file_syncs` - list of `file_sync(...)`-produced structs
      that describe all local files that should be added to the container
    - `base_image` - the name of the Docker image to use as the base
    - `env` - a dictionary of (key, value) pairs
      that will be added as environment variables when the binary is executed

    Returns:
    A list of lines that make up the Dockerfile.
    To convert them into the file contents, use `'\\n'.join(lines)`.
    """

    dockerfile = ['FROM %s' % base_image]
    # Include all specified apt packages
    if apt_packages:
        quoted_apt_packages = ['"%s"' % p for p in apt_packages]
        dockerfile.append('ENV DEBIAN_FRONTEND=noninteractive')
        dockerfile.append('RUN ' + ' && '.join([
            'apt-get update -q',
            'apt-get install -y -q %s' % ' '.join(quoted_apt_packages),
            'rm -rf /var/lib/apt/lists/*',
        ]),)
    # Add a COPY line for each file sync
    for file_sync in file_syncs:
        dockerfile.append('COPY %s %s' % (file_sync.local_path, file_sync.container_path))
    # Add all environment variables
    for env_key in env:
        dockerfile.append('ENV %s=%s' % (env_key, env[env_key]))

    return dockerfile
