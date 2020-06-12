FROM rust:1.44

WORKDIR /app
COPY ./feature-gate/ /app
RUN apt update -y
RUN apt install -y musl-dev libpq-dev

RUN cargo build
CMD /app/target/debug/feature-gate
