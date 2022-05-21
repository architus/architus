ARG REPO_ROOT=/opt/architus
ARG SERVICE=shard-rs

# Build service
FROM rust:1.58-bullseye as builder
RUN rustup component add rustfmt
ARG REPO_ROOT
ARG SERVICE
# Copy all files in the build context to the container.
# These files are defined in './Dockerfile.dockerignore'.
COPY . $REPO_ROOT
# Build via Cargo
WORKDIR $REPO_ROOT/$SERVICE
RUN cargo build --release
RUN cp $REPO_ROOT/$SERVICE/target/release/shard-rs /opt/shard-rs

# Create minimal deployment
FROM debian:bullseye-slim as deployment
ARG SERVICE
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update -q \
    && apt-get install -y -q "libssl1.1" "tini" "libpq-dev" "ca-certificates" \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /opt/shard-rs /usr/bin/shard-rs
COPY $SERVICE/config.default.toml /etc/architus/config.toml
ENV RUST_BACKTRACE=1
ENTRYPOINT ["/usr/bin/tini", "--", "/usr/bin/shard-rs"]
CMD ["/etc/architus/config.toml"]
