FROM rust:slim-bullseye

# Install dependencies
RUN apt update
RUN apt install -y pkg-config openssl libssl-dev git
RUN cargo install cargo-make