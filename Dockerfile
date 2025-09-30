FROM scratch

ADD ./target/x86_64-unknown-linux-musl/release/ddns_rust /app/ddns_rust

ENTRYPOINT /app/ddns_rust --loops