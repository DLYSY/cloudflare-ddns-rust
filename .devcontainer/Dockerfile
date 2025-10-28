FROM mcr.microsoft.com/devcontainers/rust:latest

RUN sh -c "$(curl -fsLS get.chezmoi.io)" && \
    apt update && \
    apt dist-upgrade -y && \
    apt install musl fish -y && \
    rustup target add x86_64-unknown-linux-musl