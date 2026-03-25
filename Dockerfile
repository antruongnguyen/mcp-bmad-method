# Build from source
FROM rust:1.85 AS builder

WORKDIR /src
COPY Cargo.toml Cargo.lock* ./
COPY mcp-bmad-server/ mcp-bmad-server/

RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /src/target/release/mcp-bmad-server /usr/local/bin/mcp-bmad-server

EXPOSE 3000

ENTRYPOINT ["mcp-bmad-server"]
