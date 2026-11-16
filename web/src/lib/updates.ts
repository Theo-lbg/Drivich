// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

// New-release check, run from the administrator's browser: the server stays free of outbound
// HTTPS and no one else's address is sent to GitHub.
const LATEST_URL = 'https://api.github.com/repos/Theo-lbg/Drivich/releases/latest';
const CACHE_KEY = 'drivich.latest-release';
const TTL_MS = 12 * 3_600_000;

export type Release = { version: string; url: string };

/** "v1.2.3" → [1, 2, 3]; anything non-numeric after the first dash is ignored. */
function parts(v: string): number[] {
	return v.replace(/^v/i, '').split('-')[0].split('.').map((n) => Number.parseInt(n, 10) || 0);
}

export function isNewer(latest: string, current: string): boolean {
	const a = parts(latest);
	const b = parts(current);
	for (let i = 0; i < Math.max(a.length, b.length); i++) {
		const d = (a[i] ?? 0) - (b[i] ?? 0);
		if (d !== 0) return d > 0;
	}
	return false;
}

function readCache(): { at: number; release: Release | null } | null {
	try {
		const raw = localStorage.getItem(CACHE_KEY);
		return raw ? JSON.parse(raw) : null;
	} catch {
		return null;
	}
}

/** Latest published release, or null when unreachable (offline server, rate limit…). Cached 12 h. */
export async function latestRelease(): Promise<Release | null> {
	const cached = readCache();
	if (cached && Date.now() - cached.at < TTL_MS) return cached.release;
	let release: Release | null = null;
	try {
		const res = await fetch(LATEST_URL, { headers: { Accept: 'application/vnd.github+json' }, referrerPolicy: 'no-referrer' });
		if (res.ok) {
			const j = await res.json();
			if (typeof j.tag_name === 'string' && typeof j.html_url === 'string' && j.html_url.startsWith('https://github.com/')) {
				release = { version: j.tag_name.replace(/^v/i, ''), url: j.html_url };
			}
		}
	} catch {
		/* ignore: the badge simply does not show */
	}
	try {
		localStorage.setItem(CACHE_KEY, JSON.stringify({ at: Date.now(), release }));
	} catch {
		/* storage unavailable */
	}
	return release;
}
