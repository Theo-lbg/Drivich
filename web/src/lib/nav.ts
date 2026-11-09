// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

import { goto as kitGoto } from '$app/navigation';

/**
 * `goto` for root-relative routes ("/drive"). The static demo routes with the URL
 * hash (so it can live under /<repository>/ on GitHub Pages), where the same
 * route is written "#/drive"; in the real app this is plain `goto`.
 */
export const goto: typeof kitGoto = (url, opts) =>
	kitGoto(import.meta.env.VITE_DEMO === 'true' && typeof url === 'string' && url.startsWith('/') ? '#' + url : url, opts);
