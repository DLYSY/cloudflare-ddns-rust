FROM mcr.microsoft.com/devcontainers/rust:latest

RUN apt update && \
    apt dist-upgrade -y && \
    apt install musl -y && \
    rustup target add x86_64-unknown-linux-musl