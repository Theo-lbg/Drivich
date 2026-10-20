-- SPDX-License-Identifier: AGPL-3.0-or-later
-- Copyright (c) 2026 Theo-lbg

-- Profile picture: 0 = none; incremented on every change (cache-busting for the browser).
ALTER TABLE users ADD COLUMN avatar_version bigint NOT NULL DEFAULT 0;
