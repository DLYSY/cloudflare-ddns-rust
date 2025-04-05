FROM busybox:latest

ADD ./target/x86_64-unknown-linux-musl/release/ddns_rust /app/ddns_rust

RUN mkdir -p /var/spool/cron/crontabs && \
    echo "*/2 * * * * /app/ddns_rust" | crontab -

CMD tail