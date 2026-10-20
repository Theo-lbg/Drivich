-- SPDX-License-Identifier: AGPL-3.0-or-later
-- Copyright (c) 2026 Theo-lbg

CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Users ---------------------------------------------------------------
CREATE TABLE users (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    email         text NOT NULL,
    name          text NOT NULL,
    password_hash text NOT NULL,
    is_admin      boolean NOT NULL DEFAULT false,
    disabled      boolean NOT NULL DEFAULT false,
    quota_bytes   bigint,
    created_at    timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX users_email_lower_idx ON users (lower(email));

CREATE TABLE sessions (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash   bytea NOT NULL UNIQUE,
    user_agent   text,
    created_at   timestamptz NOT NULL DEFAULT now(),
    last_seen_at timestamptz NOT NULL DEFAULT now(),
    expires_at   timestamptz NOT NULL
);
CREATE INDEX sessions_user_idx ON sessions (user_id);
CREATE INDEX sessions_expires_idx ON sessions (expires_at);

-- Blobs: original bytes live on disk, only their description lives here ---
CREATE TABLE blobs (
    id          uuid PRIMARY KEY,
    size        bigint NOT NULL,
    sha256      text,
    mime        text NOT NULL,
    thumb_state text NOT NULL DEFAULT 'none'
                CHECK (thumb_state IN ('none','pending','ready','failed','unsupported')),
    indexed     boolean NOT NULL DEFAULT false,
    created_at  timestamptz NOT NULL DEFAULT now()
);

-- Nodes: folders and files, one tree per owner (parent_id NULL = "Mon Drive") ---
CREATE TABLE nodes (
    id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id       uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    parent_id      uuid REFERENCES nodes(id) ON DELETE CASCADE,
    kind           text NOT NULL CHECK (kind IN ('folder','file')),
    name           text NOT NULL CHECK (length(name) BETWEEN 1 AND 255),
    mime           text,
    size           bigint NOT NULL DEFAULT 0,
    blob_id        uuid REFERENCES blobs(id),
    description    text,
    starred        boolean NOT NULL DEFAULT false,
    trashed_at     timestamptz,
    trash_root_id  uuid,
    created_at     timestamptz NOT NULL DEFAULT now(),
    updated_at     timestamptz NOT NULL DEFAULT now(),
    last_opened_at timestamptz,
    CHECK ((kind = 'file') = (blob_id IS NOT NULL)),
    CHECK (id IS DISTINCT FROM parent_id)
);
CREATE INDEX nodes_parent_idx   ON nodes (parent_id, trashed_at);
CREATE INDEX nodes_owner_idx    ON nodes (owner_id, parent_id, trashed_at);
CREATE INDEX nodes_recent_idx   ON nodes (owner_id, last_opened_at DESC NULLS LAST) WHERE kind = 'file';
CREATE INDEX nodes_updated_idx  ON nodes (owner_id, updated_at DESC) WHERE kind = 'file';
CREATE INDEX nodes_starred_idx  ON nodes (owner_id) WHERE starred AND trashed_at IS NULL;
CREATE INDEX nodes_trash_idx    ON nodes (owner_id, trash_root_id) WHERE trashed_at IS NOT NULL;
CREATE INDEX nodes_blob_idx     ON nodes (blob_id) WHERE blob_id IS NOT NULL;
CREATE INDEX nodes_name_trgm_idx ON nodes USING gin (lower(name) gin_trgm_ops);

-- Extracted text for content search (filled by the worker) ---------------
CREATE TABLE blob_text (
    blob_id uuid PRIMARY KEY REFERENCES blobs(id) ON DELETE CASCADE,
    tsv     tsvector NOT NULL
);
CREATE INDEX blob_text_tsv_idx ON blob_text USING gin (tsv);

-- Sharing ----------------------------------------------------------------
CREATE TABLE shares (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    node_id    uuid NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    grantee_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role       text NOT NULL CHECK (role IN ('viewer','commenter','editor')),
    created_by uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (node_id, grantee_id)
);
CREATE INDEX shares_grantee_idx ON shares (grantee_id);

CREATE TABLE share_links (
    id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    node_id    uuid NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    token_hash bytea NOT NULL UNIQUE,
    role       text NOT NULL DEFAULT 'viewer' CHECK (role IN ('viewer')),
    expires_at timestamptz,
    revoked_at timestamptz,
    created_by uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX share_links_node_idx ON share_links (node_id);

-- Resumable uploads ------------------------------------------------------
CREATE TABLE upload_sessions (
    id          uuid PRIMARY KEY,
    owner_id    uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    parent_id   uuid REFERENCES nodes(id) ON DELETE CASCADE,
    name        text NOT NULL,
    mime        text NOT NULL,
    total_size  bigint NOT NULL CHECK (total_size >= 0),
    received    bigint NOT NULL DEFAULT 0 CHECK (received >= 0),
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    expires_at  timestamptz NOT NULL
);
CREATE INDEX upload_sessions_owner_idx ON upload_sessions (owner_id);
CREATE INDEX upload_sessions_expires_idx ON upload_sessions (expires_at);

-- Background jobs (queue in PostgreSQL: SKIP LOCKED + LISTEN/NOTIFY) -----
CREATE TABLE jobs (
    id           bigserial PRIMARY KEY,
    kind         text NOT NULL,
    payload      jsonb NOT NULL DEFAULT '{}'::jsonb,
    dedupe_key   text,
    status       text NOT NULL DEFAULT 'queued' CHECK (status IN ('queued','running','done','failed')),
    attempts     integer NOT NULL DEFAULT 0,
    max_attempts integer NOT NULL DEFAULT 3,
    run_at       timestamptz NOT NULL DEFAULT now(),
    locked_until timestamptz,
    last_error   text,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now(),
    finished_at  timestamptz
);
CREATE INDEX jobs_ready_idx ON jobs (run_at) WHERE status = 'queued';
CREATE INDEX jobs_status_idx ON jobs (status, kind);
CREATE UNIQUE INDEX jobs_dedupe_idx ON jobs (dedupe_key) WHERE dedupe_key IS NOT NULL AND status IN ('queued','running');

-- Instance settings ------------------------------------------------------
CREATE TABLE settings (
    key        text PRIMARY KEY,
    value      jsonb NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now()
);
