FROM rust:1.44

WORKDIR /app
COPY ./feature-gate/ /app
RUN apt update -y
RUN apt install -y musl-dev libpq-dev
RUN rustup update
RUN rustup component add rustfmt

ENV RUST_LOG=Info
RUN cargo build --release
CMD /app/target/release/feature-gate
