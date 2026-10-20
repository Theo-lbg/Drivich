// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg
//
// Checks the translations: every language has exactly the keys of English with
// the same {placeholders}, and every t('key') used in the source exists.
//   npm run check:i18n
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';

const root = new URL('..', import.meta.url).pathname;
const dir = join(root, 'src/lib/i18n/locales');
const langs = readdirSync(dir).filter((f) => f.endsWith('.json')).map((f) => f.replace('.json', ''));
const dicts = Object.fromEntries(langs.map((l) => [l, JSON.parse(readFileSync(join(dir, `${l}.json`), 'utf8'))]));
const base = dicts.en;
const ph = (s) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort().join(',');
let errors = 0;
const fail = (msg) => { console.error('✗', msg); errors++; };

for (const l of langs) {
	for (const k of Object.keys(base)) {
		if (!(k in dicts[l])) fail(`${l}: missing key ${k}`);
		else if (ph(dicts[l][k]) !== ph(base[k])) fail(`${l}: placeholders differ for ${k} ({${ph(dicts[l][k])}} vs {${ph(base[k])}})`);
		else if (!dicts[l][k].trim()) fail(`${l}: empty value for ${k}`);
	}
	for (const k of Object.keys(dicts[l])) if (!(k in base)) fail(`${l}: unknown key ${k} (not in en.json)`);
}

// Keys used in the source (static keys only; dynamic ones are listed here).
const dynamic = ['file.', 'error.', 'unit.'];
const files = [];
(function walk(d) {
	for (const f of readdirSync(d)) {
		const p = join(d, f);
		if (statSync(p).isDirectory()) { if (f !== 'locales') walk(p); }
		else if (/\.(svelte|ts)$/.test(f)) files.push(p);
	}
})(join(root, 'src'));
const used = new Set();
for (const f of files) {
	if (f.endsWith('i18n/index.svelte.ts')) continue; // documentation examples, not real keys
	for (const m of readFileSync(f, 'utf8').matchAll(/\bt\(\s*'([\w.]+)'/g)) used.add(m[1]);
}
const has = (k) => k in base || `${k}_other` in base;
for (const k of used) if (!has(k) && !dynamic.some((d) => k.startsWith(d))) fail(`used but not defined: ${k}`);
const unused = Object.keys(base).filter((k) => {
	const root_ = k.replace(/_(one|other|few|many|zero|two)$/, '');
	return !used.has(k) && !used.has(root_) && !dynamic.some((d) => k.startsWith(d));
});
if (unused.length) console.warn(`! ${unused.length} keys look unused (kept): ${unused.slice(0, 8).join(', ')}${unused.length > 8 ? '…' : ''}`);

if (errors) { console.error(`\n${errors} problem(s)`); process.exit(1); }
console.log(`✓ ${langs.length} languages (${langs.join(', ')}), ${Object.keys(base).length} keys, ${used.size} keys used in the code`);
