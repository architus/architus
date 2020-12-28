# syntax=docker/dockerfile:experimental

# Build service
FROM rust:1.48 as builder
RUN rustup component add rustfmt
ENV REPO_ROOT=/opt/architus
ENV SERVICE=logs/submission
ENV LOGS_LIB=lib/id-rs
ENV CONFIG_BACKOFF_LIB=lib/config-backoff-rs
ENV PROTO=lib/ipc/proto
# Copy logs/submission service itself
WORKDIR $REPO_ROOT/$SERVICE
COPY $SERVICE/src src
COPY $SERVICE/Cargo.lock .
COPY $SERVICE/Cargo.toml .
COPY $SERVICE/build.rs .
# Copy shared id library
WORKDIR $REPO_ROOT/$LOGS_LIB
COPY $LOGS_LIB/src src
COPY $LOGS_LIB/Cargo.lock .
COPY $LOGS_LIB/Cargo.toml .
# Copy shared config backoff library
WORKDIR $REPO_ROOT/$CONFIG_BACKOFF_LIB
COPY $CONFIG_BACKOFF_LIB/src src
COPY $CONFIG_BACKOFF_LIB/Cargo.lock .
COPY $CONFIG_BACKOFF_LIB/Cargo.toml .
# Copy protobuf definitions
WORKDIR $REPO_ROOT/$PROTO
COPY $PROTO/logs/submission.proto logs/submission.proto
COPY $PROTO/logs/revision.proto logs/revision.proto
# Build via Cargo
WORKDIR $REPO_ROOT/$SERVICE
RUN --mount=type=cache,target=/usr/local/cargo/registry \
	--mount=type=cache,target=$REPO_ROOT/$SERVICE/target \
	--mount=type=cache,target=$REPO_ROOT/$LOGS_LIB/target \
	--mount=type=cache,target=$REPO_ROOT/$CONFIG_BACKOFF_LIB/target \
	cargo build --release
RUN cp $REPO_ROOT/$SERVICE/target/release/logs-submission /opt/logs-submission

# Create minimal deployment
FROM debian:buster-slim as deployment
ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update -q \
    && apt-get install -y -q "libssl1.1" "tini" \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /opt/logs-submission /usr/bin/logs-submission
ENV RUST_BACKTRACE=1
ENTRYPOINT ["/usr/bin/tini", "--", "/usr/bin/logs-submission"]
