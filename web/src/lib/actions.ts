// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

import { goto } from '$app/navigation';
import { api, urls } from './api';
import { dialogs, openViewer, refreshAll } from './state/app.svelte';
import { refreshStorage } from './state/session.svelte';
import { t } from './i18n/index.svelte';
import { errorToast, toast } from './state/toast.svelte';
import { isPreviewable } from './format';
import { enqueueVersion } from './state/uploads.svelte';
import type { NodeItem } from './types';

export function folderHref(id: string): string {
	return `/drive/folders/${id}`;
}

function clickLink(href: string, name?: string) {
	const a = document.createElement('a');
	a.href = href;
	if (name) a.download = name;
	document.body.appendChild(a);
	a.click();
	a.remove();
}

/** A file downloads as is; a folder streams as a .zip. */
export function downloadNode(n: NodeItem) {
	if (n.kind === 'folder') {
		toast(t('toast.zip_started'));
		clickLink(urls.zip(n.id));
		return;
	}
	clickLink(urls.download(n.id), n.name);
}

/** Download a selection: one file as is, anything else as one .zip. */
export function downloadMany(items: NodeItem[]) {
	if (items.length === 1) return downloadNode(items[0]);
	if (!items.length) return;
	toast(t('toast.zip_started'));
	clickLink(urls.zipSelection(items.map((i) => i.id)));
}

/** Pick a file and upload it as a new version of `n`. */
export function askNewVersion(n: NodeItem) {
	const input = document.createElement('input');
	input.type = 'file';
	input.onchange = () => {
		const f = input.files?.[0];
		if (f) {
			enqueueVersion(f, n);
			toast(t('versions.uploading'));
		}
	};
	input.click();
}

export function openNode(n: NodeItem, siblings: NodeItem[]) {
	if (n.trashed_at) {
		toast(t('toast.restore_to_open'));
		return;
	}
	if (n.kind === 'folder') {
		goto(folderHref(n.id));
		return;
	}
	const files = siblings.filter((s) => s.kind === 'file' && isPreviewable(s));
	const i = files.findIndex((f) => f.id === n.id);
	if (i >= 0) openViewer(files, i);
	else openViewer([n], 0);
}

export async function trashNodes(items: NodeItem[]) {
	try {
		const r = await api.batch(items.map((i) => i.id), 'trash');
		if (r.failed.length) toast(r.failed[0][1], { kind: 'error' });
		if (r.ok.length) {
			const label =
				r.ok.length === 1
					? t('toast.trashed_one', { name: items.find((i) => i.id === r.ok[0])?.name ?? '' })
					: t('toast.trashed_many', { count: r.ok.length });
			toast(label, { action: { label: t('toast.undo'), run: () => restoreNodes(items.filter((i) => r.ok.includes(i.id)), true) } });
		}
	} catch (e) {
		errorToast(e);
	}
	refreshAll();
}

export async function restoreNodes(items: NodeItem[], quiet = false) {
	try {
		const r = await api.batch(items.map((i) => i.id), 'restore');
		if (r.failed.length) toast(r.failed[0][1], { kind: 'error' });
		else if (!quiet) toast(r.ok.length === 1 ? t('toast.restored_one') : t('toast.restored_many', { count: r.ok.length }));
	} catch (e) {
		errorToast(e);
	}
	refreshAll();
}

export async function deleteForever(items: NodeItem[]) {
	try {
		const r = await api.batch(items.map((i) => i.id), 'delete');
		if (r.failed.length) toast(r.failed[0][1], { kind: 'error' });
		else toast(r.ok.length === 1 ? t('toast.deleted_forever_one') : t('toast.deleted_forever_many', { count: r.ok.length }));
	} catch (e) {
		errorToast(e);
	}
	refreshStorage();
	refreshAll();
}

export async function emptyTrash() {
	try {
		const r = await api.emptyTrash();
		toast(r.deleted ? t('toast.trash_emptied') : t('toast.trash_already_empty'));
	} catch (e) {
		errorToast(e);
	}
	refreshStorage();
	refreshAll();
}

export async function setStarred(items: NodeItem[], starred: boolean) {
	try {
		const r = await api.batch(items.map((i) => i.id), starred ? 'star' : 'unstar');
		if (r.failed.length) toast(r.failed[0][1], { kind: 'error' });
		else toast(starred ? t('toast.starred') : t('toast.unstarred'));
	} catch (e) {
		errorToast(e);
	}
	refreshAll();
}

export async function moveNodes(items: NodeItem[], parentId: string | null) {
	try {
		const r = await api.batch(items.map((i) => i.id), 'move', parentId);
		if (r.failed.length) toast(r.failed[0][1], { kind: 'error' });
		else toast(r.ok.length === 1 ? t('toast.moved_one') : t('toast.moved_many', { count: r.ok.length }));
	} catch (e) {
		errorToast(e);
	}
	refreshAll();
}

export const askRename = (n: NodeItem) => (dialogs.rename = n);
export const askShare = (n: NodeItem) => (dialogs.share = n);
export const askMove = (items: NodeItem[]) => (dialogs.move = items);
export const askDelete = (items: NodeItem[]) => (dialogs.confirmDelete = items);

/** Copy text to the clipboard (with a fallback for non-secure contexts). */
export async function copyText(text: string): Promise<boolean> {
	try {
		await navigator.clipboard.writeText(text);
		return true;
	} catch {
		const ta = document.createElement('textarea');
		ta.value = text;
		ta.style.position = 'fixed';
		ta.style.opacity = '0';
		document.body.appendChild(ta);
		ta.select();
		const ok = document.execCommand('copy');
		ta.remove();
		return ok;
	}
}
