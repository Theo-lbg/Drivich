# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg
#
# Drivich background worker (thumbnails, checksums, indexing, housekeeping).
# Build context: the repository root.
#   docker build -f worker/Dockerfile -t drivich-worker .
# Video thumbnails need ffmpeg; build with --build-arg WITH_FFMPEG=false for a
# smaller image if you do not need them. The image also contains the PostgreSQL 17
# client (pg_dump) used for the automatic database backups.

FROM rust:1-slim-bookworm@sha256:a24e292b4af7bdee351a31d3a6379972a5652f0ea22a1b6d6883324ac4f5097c AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p drivich-worker \
 && cp target/release/drivich-worker /drivich-worker

FROM debian:bookworm-slim@sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587
ARG WITH_FFMPEG=true
# pg_dump must be at least as recent as the server (17): Debian bookworm only ships 15,
# so the PostgreSQL project's own apt repository is added (key and setup script come from
# the signed `postgresql-common` package, nothing is downloaded with curl).
RUN apt-get update \
 && apt-get upgrade -y \
 && apt-get install -y --no-install-recommends ca-certificates tini postgresql-common \
 && /usr/share/postgresql-common/pgdg/apt.postgresql.org.sh -y \
 && apt-get install -y --no-install-recommends postgresql-client-17 \
 && if [ "$WITH_FFMPEG" = "true" ]; then apt-get install -y --no-install-recommends ffmpeg; fi \
 && apt-get purge -y postgresql-common \
 && apt-get autoremove -y \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 --home-dir /data --shell /usr/sbin/nologin drivich \
 && mkdir -p /data && chown drivich:drivich /data
COPY --from=build /drivich-worker /usr/local/bin/drivich-worker
USER drivich
ENV DRIVICH_DATA_DIR=/data \
    RUST_LOG=info
VOLUME ["/data"]
ENTRYPOINT ["tini", "--", "drivich-worker"]
