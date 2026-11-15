-- SPDX-License-Identifier: AGPL-3.0-or-later
-- Copyright (c) 2026 Theo-lbg

-- PDF thumbnails are now generated: retry the PDFs previously marked unsupported.
UPDATE blobs SET thumb_state = 'pending' WHERE mime = 'application/pdf' AND thumb_state = 'unsupported';
