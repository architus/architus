load('ext://restart_process', 'docker_build_with_restart')
load('ext://configmap', 'configmap_create')
load ('./tilt/features.Tiltfile', 'get_enabled_components')
load ('./tilt/config.Tiltfile', 'copy_example')

# Define how higher-level 'features' to map onto lower-level 'components'
# that are dependened on by potentially more than one feature.
# 'all' is a special built-in feature that causes all other features
# to be considered enabled no matter what the other enabled features are.
features_to_components = {
    'core': [
        'db',
        'shard',
        'manager',
        'sandbox',
        'rabbit',
        'dbmanager',
        'lavalink',
        'redis',
    ],
    'feature-gate': [
        'feature-gate',
        'db',
    ],
    'gateway': ['gateway'],
    'api': ['api'],
}

config.define_bool("no-core")
config.define_bool("rust-hot-reload")
config.define_string_list("enable")

cfg = config.parse()

no_core = cfg.get('no-core', False)
rust_hot_reload = cfg.get('rust-hot-reload', False)
enabled_features = cfg.get('enable', [] if no_core else ["core"])

# Convert the enabled features to components
enabled = get_enabled_components(features_to_components, enabled_features)


# Base resources
# ===================

copy_example(path='secret.yaml')
k8s_yaml('secret.yaml')


# Components
# ===================

if 'shard' in enabled:
    docker_build('shard-image', '.', dockerfile='shard/Dockerfile', ignore=["*", "!shard/**", "!lib/**"])
    k8s_yaml('shard/kube/dev/shard.yaml')
    k8s_resource('shard')

if 'manager' in enabled:
    docker_build('manager-image', '.', dockerfile='manager/Dockerfile', ignore=["*", "!manager/*", "!lib/**"])
    k8s_yaml('manager/kube/dev/manager.yaml')
    k8s_resource('manager')

if 'db' in enabled:
    docker_build('db-image', '.', dockerfile='db/Dockerfile', ignore=["*", "!db/*", "!lib/**"])
    k8s_yaml('db/kube/dev/db.yaml')
    k8s_resource('postgres', port_forwards=5432)

if 'sandbox' in enabled:
    copy_example(path='sandbox/.env')
    configmap_create('sandbox-config', from_env_file='sandbox/.env')

    docker_build('sandbox-image', '.', dockerfile='sandbox/Dockerfile.tilt', ignore=["*", "!sandbox/*", "!lib/**"])
    k8s_yaml('sandbox/kube/dev/sandbox.yaml')
    k8s_resource('sandbox', port_forwards=1337)

if 'rabbit' in enabled:
    docker_build('rabbit-image', '.', dockerfile='rabbitmq/Dockerfile', ignore=["*", "!rabbitmq/*", "!lib/**"])
    k8s_yaml('rabbitmq/kube/dev/rabbit.yaml')
    k8s_resource('rabbit', port_forwards=8090)

if 'dbmanager' in enabled:
    docker_build('dbmanager-image', 'dbmanager', dockerfile='dbmanager/Dockerfile')
    k8s_yaml('dbmanager/kube/dev/dbmanager.yaml')
    k8s_resource('dbmanager')

if 'gateway' in enabled:
    docker_build('gateway-image', '.', dockerfile='gateway/Dockerfile', ignore=["*", "!gateway/**", "!lib/**"])
    k8s_yaml('gateway/kube/dev/gateway.yaml')
    k8s_resource('gateway', port_forwards=6000)

if 'api' in enabled:
    docker_build('api-image', '.', dockerfile='api/Dockerfile', ignore=["*", "!api/*", "!lib/**"])
    k8s_yaml('api/kube/dev/api.yaml')
    k8s_resource('api', port_forwards=5000)

if 'lavalink' in enabled:
    docker_build('lavalink-image', '.', dockerfile='lavalink/Dockerfile', ignore=["*", "!lavalink/*"])
    k8s_yaml('lavalink/kube/dev/lavalink.yaml')
    k8s_resource('lavalink', port_forwards=5001)

if 'redis' in enabled:
    docker_build('redis-image', '.', dockerfile='redis/Dockerfile', ignore=["*", "!redis/*"])
    k8s_yaml('redis/kube/dev/redis.yaml')
    k8s_resource('redis', port_forwards=6379)

if 'feature-gate' in enabled:
    if rust_hot_reload:
        # Build locally and then use a simplified Dockerfile that just copies the binary into a container
        # Additionally, use hot reloading where the service process is restarted in-place upon rebuilds
        # From https://docs.tilt.dev/example_go.html
        copy_example(path='feature-gate/config.toml', example_path='feature-gate/config.default.toml')
        local_resource(
            name='feature-gate-compile',
            cmd='cargo build --manifest-path=feature-gate/Cargo.toml',
            deps=[
                'feature-gate/Cargo.toml',
                'feature-gate/Cargo.lock',
                'feature-gate/build.rs',
                'feature-gate/src',
            ],
        )
        docker_build_with_restart(
            ref='feature-gate-image',
            context='.',
            dockerfile='feature-gate/Dockerfile.tilt',
            only=[
                "feature-gate/target/debug/feature-gate",
                "feature-gate/config.toml",
            ],
            entrypoint=['/usr/bin/feature-gate', '/etc/architus/config.toml'],
            live_update=[
                sync('feature-gate/target/debug/feature-gate', '/usr/bin/feature-gate'),
                sync('feature-gate/config.toml', '/etc/architus/config.toml'),
            ],
        )
    else:
        docker_build(
            ref='feature-gate-image',
            context='.',
            dockerfile='feature-gate/Dockerfile',
            ignore=["*", "!feature-gate/**","!lib/**"],
        )
    k8s_yaml('feature-gate/kube/dev/feature-gate.yaml')
    k8s_resource('feature-gate')
