# Build async-opcua-demo-server
FROM rust:latest AS builder
# RUN apt-get update &&
RUN apt-get install -y libssl-dev
WORKDIR /build
COPY . .
WORKDIR samples/demo-server
RUN cargo install --path .

# Repackage the binary in a standalone container
FROM debian:bookworm-slim AS dist
RUN rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /usr/local/cargo/bin/async-opcua-demo-server ./
COPY --from=builder /build/samples/server.conf ./
COPY --from=builder /build/samples/demo-server/log4rs.yaml ./
COPY --from=builder /build/lib/src/server/html/index.html ./
EXPOSE 4855
EXPOSE 8585
CMD ["/app/async-opcua-demo-server --raise-events --config /app/server.conf"]
