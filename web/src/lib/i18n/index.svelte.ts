// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

/**
 * Interface translations.
 *
 * To add a language: copy `locales/en.json` to `locales/<code>.json`, translate
 * the values (keys never change), and add one line to LANGUAGES and LOADERS
 * below. Nothing on the server needs to change: the user's choice is stored as
 * a BCP 47 tag. Missing keys fall back to English, then to the key itself.
 *
 * Plurals: give `key_one` and `key_other` (and other CLDR forms if the language
 * needs them) and call `t('key', { count })`. Placeholders look like `{name}`.
 */

type Dict = Record<string, string>;

/** Languages offered in the settings, each named in its own language. */
export const LANGUAGES: { code: string; name: string }[] = [
	{ code: 'fr', name: 'Français' },
	{ code: 'en', name: 'English' },
	{ code: 'es', name: 'Español' },
	{ code: 'de', name: 'Deutsch' }
];

const LOADERS: Record<string, () => Promise<{ default: Dict }>> = {
	fr: () => import('./locales/fr.json'),
	en: () => import('./locales/en.json'),
	es: () => import('./locales/es.json'),
	de: () => import('./locales/de.json')
};

const STORAGE_KEY = 'drivich.locale';
const FALLBACK = 'en';

export const i18n = $state<{ locale: string; ready: boolean }>({ locale: FALLBACK, ready: false });
const dicts = $state<{ current: Dict; base: Dict }>({ current: {}, base: {} });
let plural = new Intl.PluralRules(FALLBACK);

/** Map any tag (`fr-CA`, `de-AT`, `EN`) onto a language we ship. */
export function resolveLocale(tag: string | null | undefined): string | null {
	if (!tag) return null;
	const lower = tag.toLowerCase();
	const exact = LANGUAGES.find((l) => l.code.toLowerCase() === lower);
	if (exact) return exact.code;
	const base = lower.split('-')[0];
	return LANGUAGES.find((l) => l.code.toLowerCase() === base)?.code ?? null;
}

/** Saved choice, else the browser's preferred languages, else English. */
export function detectLocale(): string {
	try {
		const saved = resolveLocale(localStorage.getItem(STORAGE_KEY));
		if (saved) return saved;
	} catch {
		/* storage unavailable */
	}
	for (const tag of navigator.languages ?? [navigator.language]) {
		const hit = resolveLocale(tag);
		if (hit) return hit;
	}
	return FALLBACK;
}

async function load(code: string): Promise<Dict> {
	return (await (LOADERS[code] ?? LOADERS[FALLBACK])()).default;
}

/** Switch language. `remember` keeps it in this browser (used before sign-in). */
export async function setLocale(tag: string | null | undefined, remember = true): Promise<void> {
	const code = resolveLocale(tag) ?? detectLocale();
	if (!dicts.base || Object.keys(dicts.base).length === 0) dicts.base = await load(FALLBACK);
	dicts.current = code === FALLBACK ? dicts.base : await load(code);
	plural = new Intl.PluralRules(code);
	i18n.locale = code;
	i18n.ready = true;
	document.documentElement.lang = code;
	if (remember) {
		try {
			localStorage.setItem(STORAGE_KEY, code);
		} catch {
			/* storage unavailable */
		}
	}
}

/** The language the browser asks for, ignoring any saved choice. */
export function browserLocale(): string {
	for (const tag of navigator.languages ?? [navigator.language]) {
		const hit = resolveLocale(tag);
		if (hit) return hit;
	}
	return FALLBACK;
}

/** Forget the saved choice and follow the browser again. */
export async function setAutoLocale(): Promise<void> {
	try {
		localStorage.removeItem(STORAGE_KEY);
	} catch {
		/* storage unavailable */
	}
	await setLocale(browserLocale(), false);
}

export async function initI18n(): Promise<void> {
	await setLocale(detectLocale(), false);
}

function interpolate(text: string, params?: Record<string, string | number>): string {
	if (!params) return text;
	return text.replace(/\{(\w+)\}/g, (m, name) => (name in params ? String(params[name]) : m));
}

/** Translate `key`. Reads reactive state, so templates update when the language changes. */
export function t(key: string, params?: Record<string, string | number>): string {
	let raw: string | undefined;
	if (params && typeof params.count === 'number') {
		const form = plural.select(params.count);
		raw = dicts.current[`${key}_${form}`] ?? dicts.current[`${key}_other`] ?? dicts.base[`${key}_${form}`] ?? dicts.base[`${key}_other`];
	}
	raw ??= dicts.current[key] ?? dicts.base[key] ?? key;
	return interpolate(raw, params);
}

export function has(key: string): boolean {
	return key in dicts.current || key in dicts.base;
}

/** Message to show for an error: translated by its API code when we have one. */
export function errorMessage(e: unknown): string {
	const code = (e as { code?: string } | null)?.code;
	if (code && has(`error.${code}`)) return t(`error.${code}`);
	if (e instanceof Error && e.message) return e.message;
	return t('error.generic');
}

/** Current BCP 47 tag for `Intl` formatting (dates, sizes). */
export const locale = () => i18n.locale;
