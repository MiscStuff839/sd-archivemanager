FROM rust:slim-bullseye

ENV CARGO_TARGET_DIR=/Dev/target

# Install dependencies
RUN apt update
RUN apt install -y pkg-config openssl libssl-dev git
RUN cargo install cargo-make