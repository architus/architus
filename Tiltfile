load('ext://restart_process', 'docker_build_with_restart')

optional_features = {
    'feature-gate': ['feature-gate'],
    'gateway': ['gateway'],
    'api': ['api'],
}

config.define_bool("rust-hot-reload")
config.define_string_list("enable")
cfg = config.parse()
rust_hot_reload = cfg.get('rust-hot-reload', False)
enabled_features_list = cfg.get('enable', [])

# Use a dict of key: True as a set
enabled_features = {f: True for f in enabled_features_list}
if any([f != 'all' and f not in optional_features for f in enabled_features]):
    fail("Invalid components specified: " + repr(enabled_features_list)
         + ".\nValid components: " + repr([f for f in optional_features] + ['all']))

# Core components
# ===============

docker_build('shard-image', '.', dockerfile='shard/Dockerfile', ignore=["*", "!shard/**", "!lib/**"])
docker_build('manager-image', '.', dockerfile='manager/Dockerfile', ignore=["*", "!manager/*", "!lib/**"])
docker_build('db-image', '.', dockerfile='db/Dockerfile', ignore=["*", "!db/*", "!lib/**"])
docker_build('sandbox-image', '.', dockerfile='sandbox/Dockerfile', ignore=["*", "!sandbox/*", "!lib/**"])
docker_build('rabbit-image', '.', dockerfile='rabbitmq/Dockerfile', ignore=["*", "!rabbitmq/*", "!lib/**"])
docker_build('dbmanager-image', 'dbmanager', dockerfile='dbmanager/Dockerfile')

k8s_yaml('secret.yaml')
k8s_yaml('shard/kube/dev/shard.yaml')
k8s_yaml('manager/kube/dev/manager.yaml')
k8s_yaml('rabbitmq/kube/dev/rabbit.yaml')
k8s_yaml('db/kube/dev/db.yaml')
k8s_yaml('sandbox/kube/dev/sandbox.yaml')
k8s_yaml('dbmanager/kube/dev/dbmanager.yaml')

k8s_resource('postgres', port_forwards=5432)
k8s_resource('dbmanager')
k8s_resource('shard')
k8s_resource('sandbox', port_forwards=1337)
k8s_resource('manager')
k8s_resource('rabbit')

# Optional components
# ===================

# Use all features if 'all' was specified
if 'all' in enabled_features:
    enabled_features = {f: True for f in optional_features}

# Convert the enabled features to components
enabled = {}
for f in enabled_features:
    for component in optional_features[f]:
        enabled[component] = True

if 'feature-gate' in enabled:
    if rust_hot_reload:
        # Build locally and then use a simplified Dockerfile that just copies the binary into a container
        # Additionally, use hot reloading where the service process is restarted in-place upon rebuilds
        # From https://docs.tilt.dev/example_go.html
        local_resource('feature-gate-compile', 'cargo build --manifest-path=feature-gate/Cargo.toml',
                       deps=['feature-gate/Cargo.toml', 'feature-gate/Cargo.lock', 'feature-gate/build.rs', 'feature-gate/src'])
        docker_build_with_restart('feature-gate-image', '.', dockerfile='feature-gate/tilt-build/Dockerfile', only=["feature-gate/target/debug/feature-gate"],
                                  entrypoint='/usr/bin/feature-gate', live_update=[sync('feature-gate/target/debug/feature-gate', '/usr/bin/feature-gate')])
    else:
        docker_build('feature-gate-image', '.', dockerfile='feature-gate/Dockerfile', ignore=["*", "!feature-gate/**", "!lib/**"])
    k8s_yaml('feature-gate/kube/dev/feature-gate.yaml')
    k8s_resource('feature-gate')

if 'gateway' in enabled:
    docker_build('gateway-image', '.', dockerfile='gateway/Dockerfile', ignore=["*", "!gateway/**", "!lib/**"])
    k8s_yaml('gateway/kube/dev/gateway.yaml')
    k8s_resource('gateway', port_forwards=6000)

if 'api' in enabled:
    docker_build('api-image', '.', dockerfile='api/Dockerfile', ignore=["*", "!api/*", "!lib/**"])
    k8s_yaml('api/kube/dev/api.yaml')
    k8s_resource('api', port_forwards=5000)
