FROM rust:1.95-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    build-essential \
	libcurl4-openssl-dev \
    cmake \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

RUN cargo fetch
RUN cargo build --release --bin api

FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates \
    wget \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/api /usr/local/bin/blazingrail

USER nobody
ENTRYPOINT ["/usr/local/bin/blazingrail"]
