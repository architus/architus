load('ext://restart_process', 'docker_build_with_restart')

# Resolves features to components
component_map = {
    'core': [
        'db',
        'shard',
        'manager',
        'starlark-reactor',
        'rabbit',
        'dbmanager',
        'lavalink'
    ],
    'feature-gate': ['feature-gate', 'db'],
    'gateway': ['gateway'],
    'api': ['api'],
    'logs': [
        'feature-gate',
        'logs-gateway-ingress',
        'logs-gateway-normalize',
        'logs-search',
        'logs-submission',
        'logs-uptime',
        'elasticsearch',
        'rabbit',
        'db'
    ]
}

config.define_bool("no-core")
config.define_bool("rust-hot-reload")
config.define_string_list("enable")
cfg = config.parse()
no_core = cfg.get('no-core', False)
rust_hot_reload = cfg.get('rust-hot-reload', False)
enabled_features_list = cfg.get('enable', [] if no_core else ["core"])

# Use a dict of key: True as a set
enabled_features = {f: True for f in enabled_features_list}
if any([f != 'all' and f not in component_map for f in enabled_features]):
    fail("Invalid components specified: " + repr(enabled_features_list)
         + ".\nValid components: " + repr([f for f in component_map] + ['all']))

# Use all features if 'all' was specified
if 'all' in enabled_features:
    enabled_features = {f: True for f in component_map}

# Convert the enabled features to components
enabled = {}
for f in enabled_features:
    for component in component_map[f]:
        enabled[component] = True


# Base resources
# ===================

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

if 'starlark-reactor' in enabled:
    docker_build('starlark-reactor-image', '.', dockerfile='starlark-reactor/Dockerfile', ignore=["*", "!starlark-reactor/*", "!lib/**"])
    k8s_yaml('starlark-reactor/kube/dev/starlark-reactor.yaml')
    k8s_resource('starlark-reactor', port_forwards=1337)

if 'rabbit' in enabled:
    docker_build('rabbit-image', '.', dockerfile='rabbitmq/Dockerfile', ignore=["*", "!rabbitmq/*", "!lib/**"])
    k8s_yaml('rabbitmq/kube/dev/rabbit.yaml')
    k8s_resource('rabbit')

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
    k8s_yaml('lavalink/kube/dev/lavalink.yaml')

if 'feature-gate' in enabled:
    if rust_hot_reload:
        # Build locally and then use a simplified Dockerfile that just copies the binary into a container
        # Additionally, use hot reloading where the service process is restarted in-place upon rebuilds
        # From https://docs.tilt.dev/example_go.html
        local_resource('feature-gate-compile', 'cargo build --manifest-path=feature-gate/Cargo.toml',
                       deps=['feature-gate/Cargo.toml', 'feature-gate/Cargo.lock', 'feature-gate/build.rs', 'feature-gate/src'])
        docker_build_with_restart('feature-gate-image', '.', dockerfile='feature-gate/Dockerfile.tilt', only=["feature-gate/target/debug/feature-gate"],
                                  entrypoint='/usr/bin/feature-gate', live_update=[sync('feature-gate/target/debug/feature-gate', '/usr/bin/feature-gate')])
    else:
        docker_build('feature-gate-image', '.', dockerfile='feature-gate/Dockerfile', ignore=["*", "!feature-gate/**", "!lib/**"])
    k8s_yaml('feature-gate/kube/dev/feature-gate.yaml')
    k8s_resource('feature-gate')

if 'elasticsearch' in enabled:
    k8s_yaml('elasticsearch/kube/dev/elasticsearch.yaml')
    k8s_resource('elasticsearch', port_forwards=[9200, 9300])

if 'logs-gateway-ingress' in enabled:
    if rust_hot_reload:
        # Build locally and then use a simplified Dockerfile that just copies the binary into a container
        # Additionally, use hot reloading where the service process is restarted in-place upon rebuilds
        # From https://docs.tilt.dev/example_go.html
        if not os.path.exists('logs/gateway-ingress/config.toml'):
            # Create a local copy of the config file if needed
            local(['cp', 'logs/gateway-ingress/config.default.toml', 'logs/gateway-ingress/config.toml'])
        local_resource('logs-gateway-ingress-compile', 'cargo build --manifest-path=logs/gateway-ingress/Cargo.toml',
                       deps=['logs/gateway-ingress/Cargo.toml', 'logs/gateway-ingress/Cargo.lock', 'logs/gateway-ingress/build.rs', 'logs/gateway-ingress/src', 'lib/ipc/proto/logs/uptime.proto', 'lib/ipc/proto/feature-gate.proto', 'logs/gateway-queue-lib/Cargo.lock', 'logs/gateway-queue-lib/Cargo.toml', 'logs/gateway-queue-lib/src', 'lib/id-rs/Cargo.lock', 'lib/id-rs/Cargo.toml', 'lib/id-rs/src', 'lib/config-backoff-rs/Cargo.lock', 'lib/config-backoff-rs/Cargo.toml', 'lib/config-backoff-rs/src', 'lib/amqp-pool-rs/Cargo.lock', 'lib/amqp-pool-rs/Cargo.toml', 'lib/amqp-pool-rs/src'])
        docker_build_with_restart('logs-gateway-ingress-image', '.', dockerfile='logs/gateway-ingress/Dockerfile.tilt', only=["logs/gateway-ingress/target/debug/logs-gateway-ingress", "logs/gateway-ingress/config.toml"],
                                  entrypoint=['/usr/bin/logs-gateway-ingress', '/etc/architus/config.toml'], live_update=[sync('logs/gateway-ingress/target/debug/logs-gateway-ingress', '/usr/bin/logs-gateway-ingress'), sync('logs/gateway-ingress/config.toml', '/etc/architus/config.toml')])
    else:
        docker_build('logs-gateway-ingress-image', '.', dockerfile='logs/gateway-ingress/Dockerfile', ignore=["*", "!logs/gateway-ingress/**", "!lib/**"])
    k8s_yaml('logs/gateway-ingress/kube/dev/logs-gateway-ingress.yaml')
    k8s_resource('logs-gateway-ingress')

if 'logs-gateway-normalize' in enabled:
    if rust_hot_reload:
        # Build locally and then use a simplified Dockerfile that just copies the binary into a container
        # Additionally, use hot reloading where the service process is restarted in-place upon rebuilds
        # From https://docs.tilt.dev/example_go.html
        if not os.path.exists('logs/gateway-normalize/config.toml'):
            # Create a local copy of the config file if needed
            local(['cp', 'logs/gateway-normalize/config.default.toml', 'logs/gateway-normalize/config.toml'])
        local_resource('logs-gateway-normalize-compile', 'cargo build --manifest-path=logs/gateway-normalize/Cargo.toml',
                       deps=['logs/gateway-normalize/Cargo.toml', 'logs/gateway-normalize/Cargo.lock', 'logs/gateway-normalize/build.rs', 'logs/gateway-normalize/src', 'lib/ipc/proto/logs/submission.proto', 'lib/ipc/proto/logs/event.proto', 'logs/gateway-queue-lib/Cargo.lock', 'logs/gateway-queue-lib/Cargo.toml', 'logs/gateway-queue-lib/src', 'lib/id-rs/Cargo.lock', 'lib/id-rs/Cargo.toml', 'lib/id-rs/src', 'lib/config-backoff-rs/Cargo.lock', 'lib/config-backoff-rs/Cargo.toml', 'lib/config-backoff-rs/src'])
        docker_build_with_restart('logs-gateway-normalize-image', '.', dockerfile='logs/gateway-normalize/Dockerfile.tilt', only=["logs/gateway-normalize/target/debug/logs-gateway-normalize", "logs/gateway-normalize/config.toml"],
                                  entrypoint=['/usr/bin/logs-gateway-normalize', '/etc/architus/config.toml'], live_update=[sync('logs/gateway-normalize/target/debug/logs-gateway-normalize', '/usr/bin/logs-gateway-normalize'), sync('logs/gateway-normalize/config.toml', '/etc/architus/config.toml')])
    else:
        docker_build('logs-gateway-normalize-image', '.', dockerfile='logs/gateway-normalize/Dockerfile', ignore=["*", "!logs/gateway-normalize/**", "!lib/**"])
    k8s_yaml('logs/gateway-normalize/kube/dev/logs-gateway-normalize.yaml')
    k8s_resource('logs-gateway-normalize')

if 'logs-submission' in enabled:
    if rust_hot_reload:
        # Build locally and then use a simplified Dockerfile that just copies the binary into a container
        # Additionally, use hot reloading where the service process is restarted in-place upon rebuilds
        # From https://docs.tilt.dev/example_go.html
        if not os.path.exists('logs/submission/config.toml'):
            # Create a local copy of the config file if needed
            local(['cp', 'logs/submission/config.default.toml', 'logs/submission/config.toml'])
        local_resource('logs-submission-compile', 'cargo build --manifest-path=logs/submission/Cargo.toml',
                       deps=['logs/submission/Cargo.toml', 'logs/submission/Cargo.lock', 'logs/submission/build.rs', 'logs/submission/src', 'lib/ipc/proto/logs/submission.proto', 'lib/ipc/proto/logs/event.proto', 'lib/id-rs/Cargo.lock', 'lib/id-rs/Cargo.toml', 'lib/id-rs/src', 'lib/config-backoff-rs/Cargo.lock', 'lib/config-backoff-rs/Cargo.toml', 'lib/config-backoff-rs/src'])
        docker_build_with_restart('logs-submission-image', '.', dockerfile='logs/submission/Dockerfile.tilt', only=["logs/submission/target/debug/logs-submission", "logs/submission/config.toml", "logs/submission/schema/index_config.json"],
                                  entrypoint=['/usr/bin/logs-submission', '/etc/architus/config.toml'], live_update=[sync('logs/submission/target/debug/logs-submission', '/usr/bin/logs-submission'), sync('logs/submission/config.toml', '/etc/architus/config.toml'), sync('logs/submission/schema/index_config.json', '/etc/architus/index_config.json')])
    else:
        docker_build('logs-submission-image', '.', dockerfile='logs/submission/Dockerfile', ignore=["*", "!logs/submission/**", "!lib/**"])
    k8s_yaml('logs/submission/kube/dev/logs-submission.yaml')
    k8s_resource('logs-submission')

if 'logs-uptime' in enabled:
    if rust_hot_reload:
        # Build locally and then use a simplified Dockerfile that just copies the binary into a container
        # Additionally, use hot reloading where the service process is restarted in-place upon rebuilds
        # From https://docs.tilt.dev/example_go.html
        if not os.path.exists('logs/uptime/config.toml'):
            # Create a local copy of the config file if needed
            local(['cp', 'logs/uptime/config.default.toml', 'logs/uptime/config.toml'])
        local_resource('logs-uptime-compile', 'cargo build --manifest-path=logs/uptime/Cargo.toml',
                       deps=['logs/uptime/Cargo.toml', 'logs/uptime/Cargo.lock', 'logs/uptime/build.rs', 'logs/uptime/src', 'lib/ipc/proto/logs/uptime.proto'])
        docker_build_with_restart('logs-uptime-image', '.', dockerfile='logs/uptime/Dockerfile.tilt', only=["logs/uptime/target/debug/logs-uptime", "logs/uptime/config.toml"],
                                  entrypoint=['/usr/bin/logs-uptime', '/etc/architus/config.toml'], live_update=[sync('logs/uptime/target/debug/logs-uptime', '/usr/bin/logs-uptime'), sync('logs/uptime/config.toml', '/etc/architus/config.toml')])
    else:
        docker_build('logs-uptime-image', '.', dockerfile='logs/uptime/Dockerfile', ignore=["*", "!logs/uptime/**", "!lib/**"])
    k8s_yaml('logs/uptime/kube/dev/logs-uptime.yaml')
    k8s_resource('logs-uptime')

if 'logs-search' in enabled:
    if rust_hot_reload:
        # Build locally and then use a simplified Dockerfile that just copies the binary into a container
        # Additionally, use hot reloading where the service process is restarted in-place upon rebuilds
        # From https://docs.tilt.dev/example_go.html
        if not os.path.exists('logs/search/config.toml'):
            # Create a local copy of the config file if needed
            local(['cp', 'logs/search/config.default.toml', 'logs/search/config.toml'])
        local_resource('logs-search-compile', 'cargo build --manifest-path=logs/search/Cargo.toml',
                       deps=['logs/search/Cargo.toml', 'logs/search/Cargo.lock', 'logs/search/build.rs', 'logs/search/src', 'lib/ipc/proto/logs/event.proto', 'lib/id-rs/Cargo.lock', 'lib/id-rs/Cargo.toml', 'lib/id-rs/src'])
        docker_build_with_restart('logs-search-image', '.', dockerfile='logs/search/Dockerfile.tilt', only=["logs/search/target/debug/logs-search", "logs/search/config.toml"],
                                  entrypoint=['/usr/bin/logs-search', '/etc/architus/config.toml'], live_update=[sync('logs/search/target/debug/logs-search', '/usr/bin/logs-search'), sync('logs/search/config.toml', '/etc/architus/config.toml')])
    else:
        docker_build('logs-search-image', '.', dockerfile='logs/search/Dockerfile', ignore=["*", "!logs/search/**", "!lib/**"])
    k8s_yaml('logs/search/kube/dev/logs-search.yaml')
    k8s_resource('logs-search', port_forwards=8174)
