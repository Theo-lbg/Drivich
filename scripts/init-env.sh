#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg
#
# Create .env from the example with a random database password.
# The password is written to .env only; it is never printed.
# Works in a clone (scripts/init-env.sh, template .env.example at the root) and
# next to the files of a release (init-env.sh, template example.env).
set -euo pipefail
cd "$(dirname "$0")"
if [[ -f example.env ]]; then
  template=example.env
  next="docker compose up -d"
else
  cd ..
  template=.env.example
  next="docker compose up -d --build"
fi
if [[ -f .env ]]; then
  echo ".env already exists, leaving it untouched."
  exit 0
fi
pw="$(head -c 48 /dev/urandom | base64 | tr -dc 'A-Za-z0-9' | head -c 32)"
umask 077
sed -e "s|^DB_PASSWORD=.*|DB_PASSWORD=${pw}|" \
    -e "s|^DRIVICH_UID=.*|DRIVICH_UID=$(id -u)|" \
    -e "s|^DRIVICH_GID=.*|DRIVICH_GID=$(id -g)|" "$template" > .env
# Folders for the data, owned by you, so the containers can write to them.
mkdir -p library postgres
echo "Created .env (database password generated) and the library/ and postgres/ folders."
echo "Review .env, then: ${next}"
