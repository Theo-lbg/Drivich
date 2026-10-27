# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg
#
# Drivich API server. Build context: the repository root.
#   docker build -f server/Dockerfile -t drivich-server .

FROM rust:1-slim-bookworm@sha256:86234e857a413004348b471d11a4ebe1aa92d2eae2b6b143cac68fd1ac3c48be AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN --mount=type=cache,id=drivich-cargo-registry,sharing=locked,target=/usr/local/cargo/registry \
    --mount=type=cache,id=drivich-server-target,target=/src/target \
    cargo build --release --locked -p drivich-server \
 && cp target/release/drivich-server /drivich-server

FROM debian:bookworm-slim@sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587
RUN apt-get update \
 && apt-get upgrade -y \
 && apt-get install -y --no-install-recommends ca-certificates tini \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 --home-dir /data --shell /usr/sbin/nologin drivich \
 && mkdir -p /data && chown drivich:drivich /data
COPY --from=build /drivich-server /usr/local/bin/drivich-server
USER drivich
ENV DRIVICH_BIND=0.0.0.0:2283 \
    DRIVICH_DATA_DIR=/data \
    RUST_LOG=info
VOLUME ["/data"]
EXPOSE 2283
HEALTHCHECK --interval=15s --timeout=5s --start-period=20s --retries=3 \
    CMD ["drivich-server", "--healthcheck"]
ENTRYPOINT ["tini", "--", "drivich-server"]
