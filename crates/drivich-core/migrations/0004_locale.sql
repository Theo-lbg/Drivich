-- SPDX-License-Identifier: AGPL-3.0-or-later
-- Copyright (c) 2026 Theo-lbg

-- Interface language chosen by the user (BCP 47 tag such as "fr" or "pt-BR");
-- NULL = follow the browser's language.
ALTER TABLE users ADD COLUMN locale text;
