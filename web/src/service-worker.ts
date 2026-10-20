// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

/// <reference lib="webworker" />
// Makes Drivich installable and lets the app shell open offline. Only the
// static build output is cached: API calls (`/api/…`), files and anything
// that carries a session are never stored by this worker.
import { version } from '$app/env';
import { assets, immutable } from '$app/manifest';

declare const self: ServiceWorkerGlobalScope;

const CACHE = `drivich-shell-${version}`;
const SHELL: string[] = [...immutable.map((f) => f.path), ...assets.map((f) => f.path), '/'];

self.addEventListener('install', (event) => {
	event.waitUntil(
		caches
			.open(CACHE)
			.then((c) => c.addAll(SHELL))
			.then(() => self.skipWaiting())
	);
});

self.addEventListener('activate', (event) => {
	event.waitUntil(
		caches
			.keys()
			.then((keys) => Promise.all(keys.filter((k) => k !== CACHE).map((k) => caches.delete(k))))
			.then(() => self.clients.claim())
	);
});

self.addEventListener('fetch', (event) => {
	const req = event.request;
	if (req.method !== 'GET') return;
	const url = new URL(req.url);
	if (url.origin !== self.location.origin || url.pathname.startsWith('/api/')) return;

	if (req.mode === 'navigate') {
		// Network first so a new deployment is seen at once; the cached shell is the offline fallback.
		event.respondWith(fetch(req).catch(async () => (await caches.match('/')) ?? Response.error()));
		return;
	}
	if (SHELL.includes(url.pathname)) {
		event.respondWith(caches.match(req).then((hit) => hit ?? fetch(req)));
	}
});
