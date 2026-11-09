// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

// Builds the static, API-less demo for GitHub Pages (see src/lib/demo/mock-api.ts).
//   DEMO_BASE=/Drivich npm run build:demo      (output in web/build)
// SvelteKit's hash router refuses the page options in src/routes/+layout.ts
// (they only matter for the real app), so that file is set aside during the build.
// The demo images and PDFs (demo-assets/) are copied into static/demo for the build only,
// so they never end up in the normal app or its Docker image.
import { spawnSync } from 'node:child_process';
import { cpSync, existsSync, renameSync, rmSync } from 'node:fs';

const layout = 'src/routes/+layout.ts';
const aside = 'src/routes/+layout.ts.demo-bak';
if (existsSync(aside)) renameSync(aside, layout);
renameSync(layout, aside);
cpSync('demo-assets', 'static/demo', { recursive: true });
let status = 1;
try {
	status = spawnSync('npx', ['vite', 'build'], { stdio: 'inherit', env: { ...process.env, VITE_DEMO: 'true' } }).status ?? 1;
} finally {
	renameSync(aside, layout);
	rmSync('static/demo', { recursive: true, force: true });
}
process.exit(status);
