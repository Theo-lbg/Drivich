-- SPDX-License-Identifier: AGPL-3.0-or-later
-- Copyright (c) 2026 Theo-lbg

-- File versions: the node always points at the current blob; older contents are
-- kept here (newest first) until they fall out of the retention window.
CREATE TABLE node_versions (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    node_id    uuid NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    blob_id    uuid NOT NULL REFERENCES blobs(id),
    size       bigint NOT NULL,
    mime       text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()  -- when this content was replaced
);
CREATE INDEX node_versions_node_idx ON node_versions (node_id, created_at DESC);
CREATE INDEX node_versions_blob_idx ON node_versions (blob_id);

-- An upload session can target an existing file (new version) instead of a folder.
ALTER TABLE upload_sessions ADD COLUMN replace_node_id uuid REFERENCES nodes(id) ON DELETE CASCADE;

-- Comments on files and folders.
CREATE TABLE comments (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    node_id    uuid NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    author_id  uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    body       text NOT NULL CHECK (length(body) BETWEEN 1 AND 4000),
    created_at timestamptz NOT NULL DEFAULT now(),
    edited_at  timestamptz
);
CREATE INDEX comments_node_idx ON comments (node_id, created_at);

-- Office documents are rendered to PDF by the worker (LibreOffice) for the viewer.
ALTER TABLE blobs ADD COLUMN preview_state text NOT NULL DEFAULT 'none'
    CHECK (preview_state IN ('none','pending','ready','failed','unsupported'));
