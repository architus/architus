ARG REPO_ROOT=/opt/architus
ARG SERVICE=logs/submission

# Build service
FROM rust:1.51 as builder
RUN rustup component add rustfmt
ARG REPO_ROOT
ARG SERVICE
# Copy all files in the build context to the container.
# These files are defined in './Dockerfile.dockerignore'.
COPY . $REPO_ROOT
# Build via Cargo
WORKDIR $REPO_ROOT/$SERVICE
RUN cargo build --release
RUN cp $REPO_ROOT/$SERVICE/target/release/logs-submission /opt/logs-submission

# Create minimal deployment
FROM debian:buster-slim as deployment
ARG SERVICE
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update -q \
    && apt-get install -y -q "libssl1.1" "tini" \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /opt/logs-submission /usr/bin/logs-submission
COPY $SERVICE/config.default.toml /etc/architus/config.toml
COPY $SERVICE/schema/index_config.json /etc/architus/index_config.json
ENV RUST_BACKTRACE=1
ENTRYPOINT ["/usr/bin/tini", "--", "/usr/bin/logs-submission", "/etc/architus/config.toml"]
