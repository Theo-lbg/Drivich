// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

import type { SortKey } from '$lib/types';

function read<T>(key: string, fallback: T): T {
	try {
		const v = localStorage.getItem(key);
		return v === null ? fallback : (JSON.parse(v) as T);
	} catch {
		return fallback;
	}
}

function write(key: string, value: unknown) {
	try {
		localStorage.setItem(key, JSON.stringify(value));
	} catch {
		/* storage unavailable */
	}
}

export type Theme = 'system' | 'light' | 'dark';

export const ui = $state({
	theme: 'system' as Theme,
	layout: 'list' as 'list' | 'grid',
	sort: 'name' as SortKey,
	order: 'asc' as 'asc' | 'desc',
	details: false,
	drawer: false,
	loaded: false
});

export function loadUi() {
	if (ui.loaded) return;
	ui.theme = read<Theme>('drivich.theme', 'system');
	applyTheme();
	ui.layout = read('drivich.layout', 'list');
	ui.sort = read('drivich.sort', 'name');
	ui.order = read('drivich.order', 'asc');
	ui.loaded = true;
}

export function setLayout(l: 'list' | 'grid') {
	ui.layout = l;
	write('drivich.layout', l);
}

export function setSort(sort: SortKey, order?: 'asc' | 'desc') {
	if (order) ui.order = order;
	else if (ui.sort === sort) ui.order = ui.order === 'asc' ? 'desc' : 'asc';
	else ui.order = sort === 'name' ? 'asc' : 'desc';
	ui.sort = sort;
	write('drivich.sort', ui.sort);
	write('drivich.order', ui.order);
}

/** Colours of the browser UI chrome (address bar on mobile), kept in sync with the theme. */
const CHROME = { light: '#f8fafd', dark: '#131314' };

export function applyTheme() {
	if (typeof document === 'undefined') return;
	const root = document.documentElement;
	if (ui.theme === 'system') root.removeAttribute('data-theme');
	else root.setAttribute('data-theme', ui.theme);
	const dark = ui.theme === 'dark' || (ui.theme === 'system' && matchMedia('(prefers-color-scheme: dark)').matches);
	document.querySelector('meta[name="theme-color"]')?.setAttribute('content', dark ? CHROME.dark : CHROME.light);
}

export function setTheme(theme: Theme) {
	ui.theme = theme;
	write('drivich.theme', theme);
	applyTheme();
}

if (typeof matchMedia !== 'undefined') {
	// Follow the device when the user left the choice on "Automatic".
	matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => ui.theme === 'system' && applyTheme());
}
