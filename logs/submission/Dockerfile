ARG REPO_ROOT=/opt/architus
ARG SERVICE=logs/submission
ARG CONFIG_BACKOFF_LIB=lib/config-backoff-rs
ARG PROTO=lib/ipc/proto

# Build service
FROM rust:1.48 as builder
ARG REPO_ROOT
ARG SERVICE
ARG CONFIG_BACKOFF_LIB
ARG PROTO
RUN rustup component add rustfmt
# Copy logs/submission service itself
WORKDIR $REPO_ROOT/$SERVICE
COPY $SERVICE/src src
COPY $SERVICE/Cargo.lock .
COPY $SERVICE/Cargo.toml .
COPY $SERVICE/build.rs .
# Copy shared config backoff library
WORKDIR $REPO_ROOT/$CONFIG_BACKOFF_LIB
COPY $CONFIG_BACKOFF_LIB/src src
COPY $CONFIG_BACKOFF_LIB/Cargo.toml .
# Copy protobuf definitions
WORKDIR $REPO_ROOT/$PROTO
COPY $PROTO/logs/submission.proto logs/submission.proto
COPY $PROTO/logs/event.proto logs/event.proto
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
