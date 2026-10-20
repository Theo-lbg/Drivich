// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

import { errorMessage } from '$lib/i18n/index.svelte';

export interface Toast {
	id: number;
	text: string;
	kind: 'info' | 'error';
	action?: { label: string; run: () => void };
}

export const toasts = $state<{ list: Toast[] }>({ list: [] });
let seq = 0;

export function toast(text: string, opts: { kind?: 'info' | 'error'; action?: Toast['action']; ms?: number } = {}) {
	const t: Toast = { id: ++seq, text, kind: opts.kind ?? 'info', action: opts.action };
	toasts.list.push(t);
	setTimeout(() => dismiss(t.id), opts.ms ?? (opts.kind === 'error' ? 7000 : 4500));
}

export function dismiss(id: number) {
	const i = toasts.list.findIndex((t) => t.id === id);
	if (i >= 0) toasts.list.splice(i, 1);
}

export function errorToast(e: unknown) {
	toast(errorMessage(e), { kind: 'error' });
}
