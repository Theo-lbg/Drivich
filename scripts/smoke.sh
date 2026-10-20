#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg
#
# End-to-end smoke test against a running Drivich API.
#   BASE=http://127.0.0.1:2283 scripts/smoke.sh
# Needs: curl, jq, head, dd. Creates throwaway users with random emails, so it
# can run against a fresh or an existing instance (the first run on an empty
# instance also performs the initial admin setup).
set -euo pipefail

BASE="${BASE:-http://127.0.0.1:2283}/api/v1"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
PW="smoke-test-password-$(date +%s)"
RUN="$(date +%s)$RANDOM"
pass=0
fail() { echo "FAIL: $*" >&2; exit 1; }
ok()   { pass=$((pass + 1)); echo "ok   $*"; }
expect_status() { # expected description curl-args...
  local want="$1" desc="$2"; shift 2
  local got
  got="$(curl -s -o /dev/null -w '%{http_code}' "$@")"
  [[ "$got" == "$want" ]] && ok "$desc ($got)" || fail "$desc: expected $want, got $got"
}
A=(-H 'X-Drivich-CSRF: 1' -b "$TMP/a.jar" -c "$TMP/a.jar")
B=(-H 'X-Drivich-CSRF: 1' -b "$TMP/b.jar" -c "$TMP/b.jar")
json() { jq -er "$1"; }

# --- health, setup/login ------------------------------------------------------
curl -sf "$BASE/health" | json '.status' >/dev/null && ok "health"
setup_required="$(curl -sf "$BASE/auth/status" | jq -r '.setup_required')"  # jq -e fails on false
EMAIL_A="alice-$RUN@example.test"; EMAIL_B="bob-$RUN@example.test"
ADM=(-H 'X-Drivich-CSRF: 1' -b "$TMP/adm.jar" -c "$TMP/adm.jar")
if [[ "$setup_required" == "true" ]]; then
  # Fresh instance: the first account becomes the administrator.
  curl -sf "${ADM[@]}" -H 'Content-Type: application/json' -d "{\"email\":\"admin-$RUN@example.test\",\"name\":\"Admin\",\"password\":\"$PW\"}" "$BASE/auth/setup" >/dev/null && ok "first-run setup creates the admin"
  expect_status 409 "setup cannot run twice" "${A[@]}" -H 'Content-Type: application/json' -d "{\"email\":\"x-$RUN@example.test\",\"name\":\"X\",\"password\":\"$PW\"}" "$BASE/auth/setup"
else
  : "${ADMIN_EMAIL:?instance already set up: set ADMIN_EMAIL and ADMIN_PASSWORD}"
  : "${ADMIN_PASSWORD:?set ADMIN_PASSWORD}"
  curl -sf "${ADM[@]}" -H 'Content-Type: application/json' -d "{\"email\":\"$ADMIN_EMAIL\",\"password\":\"$ADMIN_PASSWORD\"}" "$BASE/auth/login" >/dev/null && ok "admin login"
fi
for who in "Alice:$EMAIL_A" "Bob:$EMAIL_B"; do
  curl -sf "${ADM[@]}" -H 'Content-Type: application/json' -d "{\"email\":\"${who#*:}\",\"name\":\"${who%%:*}\",\"password\":\"$PW\"}" "$BASE/admin/users" >/dev/null || fail "admin creates ${who%%:*}"
done
curl -sf "${A[@]}" -H 'Content-Type: application/json' -d "{\"email\":\"$EMAIL_A\",\"password\":\"$PW\"}" "$BASE/auth/login" >/dev/null && ok "alice login"
curl -sf "${B[@]}" -H 'Content-Type: application/json' -d "{\"email\":\"$EMAIL_B\",\"password\":\"$PW\"}" "$BASE/auth/login" >/dev/null && ok "bob login"
expect_status 401 "anonymous request is refused" "$BASE/nodes"
expect_status 403 "state-changing request without CSRF header is refused" -b "$TMP/a.jar" -X POST -H 'Content-Type: application/json' -d '{}' "$BASE/nodes/folder"
expect_status 401 "wrong password is refused" "${B[@]}" -H 'Content-Type: application/json' -d "{\"email\":\"$EMAIL_A\",\"password\":\"nope-nope-nope\"}" "$BASE/auth/login"
expect_status 403 "a normal user cannot reach admin routes" "${B[@]}" "$BASE/admin/overview"
curl -sf "${ADM[@]}" "$BASE/admin/overview" | jq -e '.database.ok == true' >/dev/null && ok "admin overview"
curl -sf "$BASE/openapi.json" | jq -e '.openapi and (.paths | length > 40)' >/dev/null && ok "openapi.json is served"
expect_status 403 "a normal user cannot read job controls" "${B[@]}" "$BASE/admin/jobs/config"
expect_status 403 "a normal user cannot list backups" "${B[@]}" "$BASE/admin/backups"
curl -sf "${ADM[@]}" "$BASE/admin/backups" | jq -e '.items | type == "array"' >/dev/null && ok "admin backups listing"
curl -sf "${ADM[@]}" -X PUT -H 'Content-Type: application/json' -d '{"paused":["thumbnail"],"limits":{"thumbnail":1}}' "$BASE/admin/jobs/config" | jq -e '.paused == ["thumbnail"]' >/dev/null && ok "pause a job kind"
curl -sf "${ADM[@]}" -X PUT -H 'Content-Type: application/json' -d '{"paused":[],"limits":{}}' "$BASE/admin/jobs/config" | jq -e '.paused == []' >/dev/null && ok "resume job kinds"

# --- folders, rename, move --------------------------------------------------------
F1="$(curl -sf "${A[@]}" -H 'Content-Type: application/json' -d '{"parent_id":null,"name":"Projet"}' "$BASE/nodes/folder" | json .id)"
F2="$(curl -sf "${A[@]}" -H 'Content-Type: application/json' -d "{\"parent_id\":\"$F1\",\"name\":\"Archives\"}" "$BASE/nodes/folder" | json .id)"
ok "folders created"
curl -sf "${A[@]}" -X PATCH -H 'Content-Type: application/json' -d '{"name":"Projet 2026"}' "$BASE/nodes/$F1" | json '.name' | grep -qx 'Projet 2026' && ok "rename"
expect_status 400 "a name with / is refused" "${A[@]}" -X PATCH -H 'Content-Type: application/json' -d '{"name":"a/b"}' "$BASE/nodes/$F1"
expect_status 400 "a folder cannot be moved into itself" "${A[@]}" -X PATCH -H 'Content-Type: application/json' -d "{\"parent_id\":\"$F2\"}" "$BASE/nodes/$F1"
expect_status 404 "bob cannot see alice's folder" "${B[@]}" "$BASE/nodes/$F1"

# --- resumable upload with interruption -----------------------------------------------
dd if=/dev/urandom of="$TMP/big.bin" bs=1024 count=3000 2>/dev/null   # ~3 MB
SIZE="$(wc -c < "$TMP/big.bin" | tr -d ' ')"
UP="$(curl -sf "${A[@]}" -H 'Content-Type: application/json' -d "{\"parent_id\":\"$F1\",\"name\":\"big.bin\",\"size\":$SIZE}" "$BASE/uploads" | json '.upload.id')"
head -c 1000000 "$TMP/big.bin" | curl -sf "${A[@]}" -X PATCH -H 'Upload-Offset: 0' -H 'Content-Type: application/offset+octet-stream' --data-binary @- "$BASE/uploads/$UP" | json '.upload.received' | grep -qx 1000000 && ok "first chunk stored"
off="$(curl -sI "${A[@]}" "$BASE/uploads/$UP" | tr -d '\r' | awk -F': ' 'tolower($1)=="upload-offset"{print $2}')"
[[ "$off" == "1000000" ]] && ok "HEAD reports the resume offset ($off)" || fail "offset was '$off'"
expect_status 409 "wrong offset is refused" "${A[@]}" -X PATCH -H 'Upload-Offset: 5' --data-binary 'zz' "$BASE/uploads/$UP"
expect_status 404 "bob cannot touch alice's upload" "${B[@]}" -X PATCH -H 'Upload-Offset: 1000000' --data-binary 'zz' "$BASE/uploads/$UP"
NODE="$(tail -c +1000001 "$TMP/big.bin" | curl -sf "${A[@]}" -X PATCH -H 'Upload-Offset: 1000000' -H 'Content-Type: application/offset+octet-stream' --data-binary @- "$BASE/uploads/$UP" | json '.node.id')"
ok "upload resumed and completed"

# --- download, Range, integrity -------------------------------------------------------------
curl -sf "${A[@]}" -o "$TMP/dl.bin" "$BASE/nodes/$NODE/download"
cmp "$TMP/big.bin" "$TMP/dl.bin" && ok "downloaded bytes are identical"
code="$(curl -s "${A[@]}" -H 'Range: bytes=100-199' -o "$TMP/part.bin" -w '%{http_code}' "$BASE/nodes/$NODE/download")"
[[ "$code" == 206 ]] && cmp <(dd if="$TMP/big.bin" bs=1 skip=100 count=100 2>/dev/null) "$TMP/part.bin" && ok "Range request returns 206 with the right bytes"
expect_status 416 "unsatisfiable range" "${A[@]}" -H "Range: bytes=$((SIZE + 10))-" "$BASE/nodes/$NODE/download"
etag="$(curl -sI "${A[@]}" "$BASE/nodes/$NODE/download" | tr -d '\r' | awk -F': ' 'tolower($1)=="etag"{print $2}')"
expect_status 304 "ETag revalidation" "${A[@]}" -H "If-None-Match: $etag" "$BASE/nodes/$NODE/download"
expect_status 404 "bob cannot download alice's file" "${B[@]}" "$BASE/nodes/$NODE/download"

# --- text file: inline safety + content search -------------------------------------------------
printf '<script>alert(1)</script> mot-unique-%s' "$RUN" > "$TMP/page.html"
HTML="$(curl -sf "${A[@]}" -H 'Content-Type: application/json' -d "{\"parent_id\":\"$F1\",\"name\":\"page.html\",\"size\":$(wc -c < "$TMP/page.html" | tr -d ' ')}" "$BASE/uploads" | json '.upload.id')"
HNODE="$(curl -sf "${A[@]}" -X PATCH -H 'Upload-Offset: 0' --data-binary @"$TMP/page.html" "$BASE/uploads/$HTML" | json '.node.id')"
ctype="$(curl -sI "${A[@]}" "$BASE/nodes/$HNODE/content" | tr -d '\r' | awk -F': ' 'tolower($1)=="content-type"{print $2}')"
[[ "$ctype" == text/plain* ]] && ok "HTML is served as text/plain, never executed ($ctype)" || fail "HTML content-type was $ctype"

# --- search (name, suggestions) ----------------------------------------------------------------------------
curl -sf "${A[@]}" -G --data-urlencode "q=big" "$BASE/search" | jq -e '.items | length >= 1' >/dev/null && ok "search finds by name"
curl -sf "${A[@]}" -G --data-urlencode "q=big" "$BASE/search/suggest" | jq -e '.items | length >= 1' >/dev/null && ok "suggestions"
curl -sf "${B[@]}" -G --data-urlencode "q=big" "$BASE/search" | jq -e '.items | length == 0' >/dev/null && ok "bob's search never leaks alice's files"

# --- sharing -------------------------------------------------------------------------------------------------------------
curl -sf "${A[@]}" -H 'Content-Type: application/json' -d "{\"email\":\"$EMAIL_B\",\"role\":\"viewer\"}" "$BASE/nodes/$F1/shares" >/dev/null && ok "folder shared with bob (viewer)"
curl -sf "${B[@]}" "$BASE/nodes/$NODE" | jq -e '.role == "viewer"' >/dev/null && ok "bob sees the file through the folder share"
curl -sf "${B[@]}" "$BASE/nodes?view=shared" | jq -e '.items | map(.name) | index("Projet 2026") != null' >/dev/null && ok "listed in 'Shared with me'"
expect_status 403 "viewer cannot rename" "${B[@]}" -X PATCH -H 'Content-Type: application/json' -d '{"name":"x"}' "$BASE/nodes/$NODE"
expect_status 403 "viewer cannot create inside" "${B[@]}" -H 'Content-Type: application/json' -d "{\"parent_id\":\"$F1\",\"name\":\"x\"}" "$BASE/nodes/folder"
expect_status 403 "viewer cannot manage sharing" "${B[@]}" "$BASE/nodes/$F1/shares"
curl -sf "${B[@]}" "$BASE/nodes/$NODE/download" -o "$TMP/dl2.bin" && cmp "$TMP/big.bin" "$TMP/dl2.bin" && ok "bob downloads the shared file"
SHARE="$(curl -sf "${A[@]}" "$BASE/nodes/$F1/shares" | json '.users[0].id')"
curl -sf "${A[@]}" -X DELETE "$BASE/shares/$SHARE" >/dev/null
expect_status 404 "revoking the share removes access immediately" "${B[@]}" "$BASE/nodes/$NODE"

# --- public link ----------------------------------------------------------------------------------------------------------------
LINK="$(curl -sf "${A[@]}" -X POST -H 'Content-Type: application/json' -d '{}' "$BASE/nodes/$NODE/links")"
TOKEN="$(echo "$LINK" | json .token)"; LINKID="$(echo "$LINK" | json .link.id)"
curl -sf "$BASE/public/$TOKEN" | jq -e '.root.name == "big.bin"' >/dev/null && ok "public link resolves without login"
curl -sf "$BASE/public/$TOKEN/nodes/$NODE/download" -o "$TMP/dl3.bin" && cmp "$TMP/big.bin" "$TMP/dl3.bin" && ok "public download"
expect_status 404 "public link cannot reach other files" "$BASE/public/$TOKEN/nodes/$HNODE/download"
curl -sf "${A[@]}" -X DELETE "$BASE/links/$LINKID" >/dev/null
expect_status 404 "revoked link stops working" "$BASE/public/$TOKEN"

# --- trash, restore, delete -------------------------------------------------------------------------------------------------
curl -sf "${A[@]}" -X POST "$BASE/nodes/$F1/trash" >/dev/null
curl -sf "${A[@]}" "$BASE/nodes?view=trash" | jq -e '.items | map(.name) | index("Projet 2026") != null' >/dev/null && ok "folder is in the trash"
curl -sf "${A[@]}" -G --data-urlencode "q=big" "$BASE/search" | jq -e '.items | length == 0' >/dev/null && ok "trashed files leave search results"
curl -sf "${A[@]}" -X POST "$BASE/nodes/$F1/restore" >/dev/null && ok "restore"
curl -sf "${A[@]}" -X POST "$BASE/nodes/$F1/trash" >/dev/null
curl -sf "${A[@]}" -X DELETE "$BASE/nodes/$F1" >/dev/null && ok "permanent delete"
expect_status 404 "files inside are gone" "${A[@]}" "$BASE/nodes/$NODE"

# --- Profile picture -------------------------------------------------------------------------------------
# 1x1 PNG, enough to exercise the upload path (the server re-encodes it to a 256x256 JPEG).
PNG_B64='iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg=='
printf '%s' "$PNG_B64" | base64 -d > "$TMP/avatar.png"
ALICE_ID="$(curl -sf "${A[@]}" "$BASE/me" | json .user.id)"
expect_status 401 "avatars require a session" "$BASE/users/$ALICE_ID/avatar"
expect_status 404 "no picture yet" "${B[@]}" "$BASE/users/$ALICE_ID/avatar"
expect_status 400 "a non-image is refused" "${A[@]}" -X PUT --data-binary 'this is not an image' "$BASE/me/avatar"
curl -sf "${A[@]}" -X PUT -H 'Content-Type: image/png' --data-binary @"$TMP/avatar.png" "$BASE/me/avatar" | jq -e '.user.avatar_version == 1' >/dev/null && ok "profile picture uploaded (version 1)"
ctype="$(curl -sI "${B[@]}" "$BASE/users/$ALICE_ID/avatar" | tr -d '\r' | awk -F': ' 'tolower($1)=="content-type"{print $2}')"
[[ "$ctype" == image/jpeg* ]] && ok "other users can see it, re-encoded as JPEG" || fail "avatar content-type was $ctype"
aet="$(curl -sI "${B[@]}" "$BASE/users/$ALICE_ID/avatar" | tr -d '\r' | awk -F': ' 'tolower($1)=="etag"{print $2}')"
expect_status 304 "avatar ETag revalidation" "${B[@]}" -H "If-None-Match: $aet" "$BASE/users/$ALICE_ID/avatar"
curl -sf "${B[@]}" "$BASE/search/suggest?q=x" >/dev/null
curl -sf "${A[@]}" -X DELETE "$BASE/me/avatar" | jq -e '.user.avatar_version == 0' >/dev/null && ok "profile picture removed"
expect_status 404 "removed picture is gone" "${B[@]}" "$BASE/users/$ALICE_ID/avatar"

# --- Interface language -----------------------------------------------------------------------------------
expect_status 400 "an invalid language tag is refused" "${A[@]}" -X PATCH -H 'Content-Type: application/json' -d '{"locale":"fr_FR"}' "$BASE/me"
curl -sf "${A[@]}" -X PATCH -H 'Content-Type: application/json' -d '{"locale":"pt-BR"}' "$BASE/me" | jq -e '.user.locale == "pt-BR"' >/dev/null && ok "language saved on the account"
curl -sf "${A[@]}" "$BASE/me" | jq -e '.user.locale == "pt-BR"' >/dev/null && ok "/me returns the language"
curl -sf "${A[@]}" -X PATCH -H 'Content-Type: application/json' -d '{"locale":null}' "$BASE/me" | jq -e '.user.locale == null' >/dev/null && ok "language cleared (follows the browser)"

# --- Immich shortcut (apps launcher) ----------------------------------------------------------------
expect_status 400 "a javascript: address is refused" "${A[@]}" -X PATCH -H 'Content-Type: application/json' -d '{"immich_url":"javascript:alert(1)"}' "$BASE/me"
curl -sf "${A[@]}" -X PATCH -H 'Content-Type: application/json' -d '{"immich_url":"https://photos.example.test/"}' "$BASE/me" | jq -e '.user.immich_url == "https://photos.example.test"' >/dev/null && ok "personal Immich address saved (normalised)"
curl -sf "${A[@]}" "$BASE/me" | jq -e '.immich_url == "https://photos.example.test"' >/dev/null && ok "/me exposes the effective Immich address"
curl -sf "${ADM[@]}" -X PUT -H 'Content-Type: application/json' -d '{"immich_url":"https://default.example.test"}' "$BASE/admin/settings" | jq -e '.immich_url == "https://default.example.test"' >/dev/null && ok "admin sets the server default"
curl -sf "${B[@]}" "$BASE/me" | jq -e '.immich_url == "https://default.example.test"' >/dev/null && ok "other users get the server default"
curl -sf "${A[@]}" -X PATCH -H 'Content-Type: application/json' -d '{"immich_url":null}' "$BASE/me" >/dev/null
curl -sf "${A[@]}" "$BASE/me" | jq -e '.immich_url == "https://default.example.test"' >/dev/null && ok "clearing the personal address falls back to the default"
curl -sf "${ADM[@]}" -X PUT -H 'Content-Type: application/json' -d '{"immich_url":null}' "$BASE/admin/settings" >/dev/null

# --- zip download of folders -------------------------------------------------------------------------------------
mkfolder() { curl -sf "${A[@]}" -H 'Content-Type: application/json' -d "{\"parent_id\":$2,\"name\":\"$1\"}" "$BASE/nodes/folder" | json .id; }
upfile() { # parent name content
  local id; id="$(curl -sf "${A[@]}" -H 'Content-Type: application/json' -d "{\"parent_id\":\"$1\",\"name\":\"$2\",\"size\":${#3}}" "$BASE/uploads" | json .upload.id)"
  curl -sf "${A[@]}" -X PATCH -H 'Upload-Offset: 0' --data-binary "$3" "$BASE/uploads/$id" | json .node.id
}
ZR="$(mkfolder "Zip $RUN" null)"; ZS="$(mkfolder "sous" "\"$ZR\"")"; mkfolder "vide" "\"$ZR\"" >/dev/null
ZA="$(upfile "$ZR" "a.txt" "alpha")"; upfile "$ZS" "b.txt" "bravo bravo" >/dev/null
curl -sf "${A[@]}" -o "$TMP/f.zip" "$BASE/nodes/$ZR/zip"
unzip -tq "$TMP/f.zip" >/dev/null && ok "folder downloads as a valid .zip"
unzip -Z1 "$TMP/f.zip" | sort | tr '\n' '|' | grep -qx "Zip $RUN/|Zip $RUN/a.txt|Zip $RUN/sous/|Zip $RUN/sous/b.txt|Zip $RUN/vide/|" && ok "zip keeps the tree, empty folders included"
[[ "$(unzip -p "$TMP/f.zip" "Zip $RUN/sous/b.txt")" == "bravo bravo" ]] && ok "zip content is intact"
cd_head="$(curl -sI "${A[@]}" "$BASE/nodes/$ZR/zip" | tr -d '\r' | awk -F': ' 'tolower($1)=="content-disposition"{print $2}')"
[[ "$cd_head" == attachment* ]] && ok "zip is an attachment"
expect_status 404 "bob cannot zip alice's folder" "${B[@]}" "$BASE/nodes/$ZR/zip"
expect_status 400 "a file is not zipped on its own route" "${A[@]}" "$BASE/nodes/$ZA/zip"
curl -sf "${A[@]}" -o "$TMP/sel.zip" "$BASE/nodes/zip?ids=$ZA,$ZR" && unzip -tq "$TMP/sel.zip" >/dev/null && ok "a selection downloads as one .zip"
expect_status 400 "an invalid selection is refused" "${A[@]}" "$BASE/nodes/zip?ids=nope"
ZL="$(curl -sf "${A[@]}" -X POST -H 'Content-Type: application/json' -d '{}' "$BASE/nodes/$ZR/links")"; ZT="$(echo "$ZL" | json .token)"
curl -sf -o "$TMP/pub.zip" "$BASE/public/$ZT/nodes/$ZR/zip" && unzip -tq "$TMP/pub.zip" >/dev/null && ok "shared folder zip through a public link"
expect_status 404 "public zip outside the link is refused" "$BASE/public/$ZT/nodes/$F1/zip"

# --- versions -----------------------------------------------------------------------------------------------------------
newver() { # node content -> prints new size
  local id; id="$(curl -sf "${A[@]}" -H 'Content-Type: application/json' -d "{\"replace_node_id\":\"$1\",\"size\":${#2}}" "$BASE/uploads" | json .upload.id)"
  curl -sf "${A[@]}" -X PATCH -H 'Upload-Offset: 0' --data-binary "$2" "$BASE/uploads/$id" | json .node.size
}
[[ "$(newver "$ZA" "alpha two")" == 9 ]] && ok "uploading over a file creates a new version"
newver "$ZA" "alpha three!" >/dev/null
curl -sf "${A[@]}" "$BASE/nodes/$ZA/versions" | jq -e '(.items | length == 2) and .max_versions >= 1' >/dev/null && ok "versions are listed"
V1="$(curl -sf "${A[@]}" "$BASE/nodes/$ZA/versions" | json '.items[-1].id')"
[[ "$(curl -sf "${A[@]}" "$BASE/nodes/$ZA/versions/$V1/download")" == "alpha" ]] && ok "an old version downloads with its original bytes"
curl -sf "${A[@]}" -X POST "$BASE/nodes/$ZA/versions/$V1/restore" | jq -e '.node.size == 5' >/dev/null && ok "restoring an old version works"
[[ "$(curl -sf "${A[@]}" "$BASE/nodes/$ZA/download")" == "alpha" ]] && ok "restored content is the current content"
curl -sf "${A[@]}" -H 'Content-Type: application/json' -d "{\"email\":\"$EMAIL_B\",\"role\":\"viewer\"}" "$BASE/nodes/$ZR/shares" >/dev/null
expect_status 200 "a viewer can read versions" "${B[@]}" "$BASE/nodes/$ZA/versions"
expect_status 403 "a viewer cannot upload a version" "${B[@]}" -H 'Content-Type: application/json' -d "{\"replace_node_id\":\"$ZA\",\"size\":3}" "$BASE/uploads"
expect_status 403 "a viewer cannot restore a version" "${B[@]}" -X POST "$BASE/nodes/$ZA/versions/$V1/restore"
curl -sf "${A[@]}" -X DELETE "$BASE/nodes/$ZA/versions/$V1" >/dev/null && ok "an old version can be deleted"

# --- comments ----------------------------------------------------------------------------------------------------------
expect_status 403 "a viewer cannot comment" "${B[@]}" -H 'Content-Type: application/json' -d '{"body":"hi"}' "$BASE/nodes/$ZA/comments"
SHARE_B="$(curl -sf "${A[@]}" "$BASE/nodes/$ZR/shares" | json '.users[0].id')"
curl -sf "${A[@]}" -X PATCH -H 'Content-Type: application/json' -d '{"role":"commenter"}' "$BASE/shares/$SHARE_B" >/dev/null
CID="$(curl -sf "${B[@]}" -H 'Content-Type: application/json' -d '{"body":"  <b>bonjour</b>  "}' "$BASE/nodes/$ZA/comments" | json .comment.id)"
curl -sf "${A[@]}" "$BASE/nodes/$ZA/comments" | jq -e '.items[0].body == "<b>bonjour</b>" and .items[0].can_delete == true and .items[0].can_edit == false' >/dev/null && ok "a commenter comments; the owner sees it and may delete it"
expect_status 403 "only the author edits a comment" "${A[@]}" -X PATCH -H 'Content-Type: application/json' -d '{"body":"x"}' "$BASE/comments/$CID"
curl -sf "${B[@]}" -X PATCH -H 'Content-Type: application/json' -d '{"body":"corrigé"}' "$BASE/comments/$CID" | jq -e '.comment.edited_at != null' >/dev/null && ok "the author edits their comment"
expect_status 400 "an empty comment is refused" "${B[@]}" -H 'Content-Type: application/json' -d '{"body":"   "}' "$BASE/nodes/$ZA/comments"
curl -sf "${A[@]}" -X DELETE "$BASE/comments/$CID" >/dev/null && ok "the owner deletes a comment"
expect_status 404 "a stranger cannot even list comments" "${ADM[@]}" "$BASE/nodes/$ZA/comments"

# --- Office previews (the worker renders them; without it the request is just queued) ---------------------
DOCX="$(upfile "$ZR" "Rapport.docx" "PK-not-a-real-docx")"
st="$(curl -sf "${A[@]}" -X POST "$BASE/nodes/$DOCX/preview" | json .state)"
[[ "$st" == pending || "$st" == unsupported || "$st" == failed || "$st" == ready ]] && ok "Office preview is requested ($st)"
curl -sf "${A[@]}" -X POST "$BASE/nodes/$ZA/preview" | jq -e '.state == "unsupported"' >/dev/null && ok "a text file has no Office preview"
expect_status 404 "no preview file before the worker made one" "${B[@]}" "$BASE/nodes/$DOCX/preview"

# --- logout ----------------------------------------------------------------------------------------------------------------------------
curl -sf "${A[@]}" -X POST "$BASE/auth/logout" >/dev/null
expect_status 401 "session is invalid after logout" "${A[@]}" "$BASE/nodes"

echo; echo "All $pass checks passed."
