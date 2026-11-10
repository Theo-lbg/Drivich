#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg
#
# PreToolUse hook for the Bash tool (see .claude/settings.json).
# Blocks the few commands that cannot be undone or that break the project's
# rules; everything else passes. Exit 2 = blocked, the message goes to Claude.
#
# Rules it enforces (see CLAUDE.md):
#   - `main` is protected: no commit and no push from it, never a force push;
#   - a Docker volume holds the database and the library: no `down -v`,
#     `volume rm` or `system prune --volumes`.
set -uo pipefail

cmd="$(jq -r '.tool_input.command // empty' 2>/dev/null)"
[[ -z "$cmd" ]] && exit 0

block() {
  echo "Blocked by .claude/hooks/validate-bash.sh: $1" >&2
  exit 2
}

# Force pushes, whatever the branch.
if grep -Eq 'git[[:space:]]+push([^;&|]*)[[:space:]](-f|--force|--force-with-lease)([[:space:]]|=|$)' <<<"$cmd"; then
  block "force push. Ask the user first."
fi

# Commits and pushes while on main (or pushing to it explicitly).
if grep -Eq 'git[[:space:]]+(commit|push)([[:space:]]|$)' <<<"$cmd"; then
  branch="$(git -C "${CLAUDE_PROJECT_DIR:-.}" branch --show-current 2>/dev/null || true)"
  if [[ "$branch" == "main" || "$branch" == "master" ]]; then
    block "'$branch' is protected (pull request + green CI). Create a branch first (feat/…, fix/…, chore/…)."
  fi
  if grep -Eq 'git[[:space:]]+push[^;&|]*[[:space:]](HEAD:)?(main|master)([[:space:]]|$)' <<<"$cmd"; then
    block "pushing to main is not allowed; open a pull request."
  fi
fi

# Destroying the data volumes of a Compose stack.
if grep -Eq 'docker[[:space:]]+compose[^;&|]*[[:space:]]down([^;&|]*)[[:space:]](-v|--volumes)([[:space:]]|$)' <<<"$cmd" \
  || grep -Eq 'docker[[:space:]]+volume[[:space:]]+(rm|prune)' <<<"$cmd" \
  || grep -Eq 'docker[[:space:]]+system[[:space:]]+prune[^;&|]*--volumes' <<<"$cmd"; then
  block "this would delete Docker volumes (database, library). Ask the user first."
fi

exit 0
