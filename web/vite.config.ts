// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';

// Drivich web is a single-page app: every route is served by index.html and
// talks to the API under /api (proxied to the server in dev, by nginx in Docker).
const apiTarget = process.env.DRIVICH_API_URL ?? 'http://127.0.0.1:2283';

export default defineConfig({
	resolve: { alias: { $lib: fileURLToPath(new URL('./src/lib', import.meta.url)) } },
	plugins: [
		sveltekit({
			compilerOptions: {
				// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},
			adapter: adapter({ fallback: 'index.html', precompress: false, strict: false })
		})
	],
	server: {
		proxy: { '/api': { target: apiTarget, changeOrigin: false } }
	}
});
