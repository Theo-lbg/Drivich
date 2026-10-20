-- SPDX-License-Identifier: AGPL-3.0-or-later
-- Copyright (c) 2026 Theo-lbg

-- Optional link to the user's Immich instance (shown in the apps launcher).
ALTER TABLE users ADD COLUMN immich_url text;
