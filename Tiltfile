load('ext://restart_process', 'docker_build_with_restart')
load('ext://configmap', 'configmap_create')
load ('./tilt/features.Tiltfile', 'get_enabled_components')
load ('./tilt/config.Tiltfile', 'copy_example')
load('./tilt/rust_local_binary.Tiltfile', 'rust_local_binary')
load('./tilt/hot_reload.Tiltfile', 'rust_hot_reload_docker_build', 'file_sync')

# Define how higher-level 'features' to map onto lower-level 'components'
# that are depended on by potentially more than one feature.
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
    docker_build('shard-image', '.', dockerfile='shard/Dockerfile')
    k8s_yaml('shard/kube/dev/shard.yaml')
    k8s_resource('shard')

if 'manager' in enabled:
    docker_build('manager-image', '.', dockerfile='manager/Dockerfile')
    k8s_yaml('manager/kube/dev/manager.yaml')
    k8s_resource('manager')

if 'db' in enabled:
    docker_build('db-image', '.', dockerfile='db/Dockerfile')
    k8s_yaml('db/kube/dev/db.yaml')
    k8s_resource('postgres', port_forwards=5432)

if 'sandbox' in enabled:
    copy_example(path='sandbox/.env')
    configmap_create('sandbox-config', from_env_file='sandbox/.env')

    docker_build('sandbox-image', '.', dockerfile='sandbox/Dockerfile')
    k8s_yaml('sandbox/kube/dev/sandbox.yaml')
    k8s_resource('sandbox', port_forwards=1337)

if 'rabbit' in enabled:
    docker_build('rabbit-image', '.', dockerfile='rabbitmq/Dockerfile')
    k8s_yaml('rabbitmq/kube/dev/rabbit.yaml')
    k8s_resource('rabbit', port_forwards=[8090, 15672])

if 'dbmanager' in enabled:
    docker_build('dbmanager-image', '.', dockerfile='dbmanager/Dockerfile')
    k8s_yaml('dbmanager/kube/dev/dbmanager.yaml')
    k8s_resource('dbmanager')

if 'gateway' in enabled:
    docker_build('gateway-image', '.', dockerfile='gateway/Dockerfile')
    k8s_yaml('gateway/kube/dev/gateway.yaml')
    k8s_resource('gateway', port_forwards=6000)

if 'api' in enabled:
    docker_build('api-image', '.', dockerfile='api/Dockerfile')
    k8s_yaml('api/kube/dev/api.yaml')
    k8s_resource('api', port_forwards=5000)

if 'lavalink' in enabled:
    docker_build('lavalink-image', '.', dockerfile='lavalink/Dockerfile')
    k8s_yaml('lavalink/kube/dev/lavalink.yaml')
    k8s_resource('lavalink', port_forwards=5001)

if 'redis' in enabled:
    docker_build('redis-image', '.', dockerfile='redis/Dockerfile')
    k8s_yaml('redis/kube/dev/redis.yaml')
    k8s_resource('redis', port_forwards=6379)

if 'feature-gate' in enabled:
    if rust_hot_reload:
        copy_example(path='feature-gate/config.toml', example_path='feature-gate/config.default.toml')
        # Build locally and then use a simplified Dockerfile that just copies the binary into a container
        binary_path = rust_local_binary(
            crate_path='feature-gate',
            additional_dependencies=['lib/proto/feature-gate.proto'],
        )
        rust_hot_reload_docker_build(
            ref='feature-gate-image',
            binary_path=binary_path,
            apt_packages=['libpq-dev', 'libssl1.1'],
            file_syncs=[file_sync('feature-gate/config.toml', '/etc/architus/config.toml')],
            arguments=['/etc/architus/config.toml'],
        )
    else:
        docker_build('feature-gate-image', '.', dockerfile='feature-gate/Dockerfile')
    k8s_yaml('feature-gate/kube/dev/feature-gate.yaml')
    k8s_resource('feature-gate')
