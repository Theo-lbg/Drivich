// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

import { api } from '$lib/api';
import type { MenuItem } from '$lib/components/Menu.svelte';
import { t } from '$lib/i18n/index.svelte';
import { dialogs, nav, openViewer, pickers, refreshAll } from '$lib/state/app.svelte';
import { session } from '$lib/state/session.svelte';
import { errorToast } from '$lib/state/toast.svelte';

type BlankKind = 'docx' | 'xlsx' | 'pptx' | 'md' | 'txt';

const DEFAULT_NAME: Record<BlankKind, string> = {
	docx: 'new.document_sans_titre',
	xlsx: 'new.feuille_sans_titre',
	pptx: 'new.presentation_sans_titre',
	md: 'new.markdown_sans_titre',
	txt: 'new.texte_sans_titre'
};

/** Create an empty document in the current folder and open it. */
async function createBlank(kind: BlankKind) {
	try {
		const node = await api.createBlank(nav.canWrite ? nav.folderId : null, kind, t(DEFAULT_NAME[kind]));
		refreshAll();
		openViewer([node], 0);
	} catch (err) {
		errorToast(err);
	}
}

/** Shortcut hint in the Drive style: "^C, then F". */
const chord = (key: string) => t('new.raccourci', { key });

/** Entries of the "New" menu, shared by the sidebar button, the mobile button and the context menus. */
export function newMenuItems(parent: string | null = nav.canWrite ? nav.folderId : null): MenuItem[] {
	const items: MenuItem[] = [
		{ label: t('dialogs.nouveau_dossier'), icon: 'create_new_folder', hint: chord('F'), run: () => (dialogs.newFolder = { parent }) },
		{ divider: true },
		{ label: t('file_browser.importer_un_fichier'), icon: 'upload_file', hint: chord('U'), run: () => pickers.files?.() },
		{ label: t('file_browser.importer_un_dossier'), icon: 'drive_folder_upload', hint: chord('I'), run: () => pickers.folder?.() },
		{ divider: true }
	];
	if (session.office) {
		items.push(
			{ label: t('new.documents'), icon: 'description', color: 'var(--file-blue)', submenu: [{ label: t('new.document_vierge'), icon: 'description', color: 'var(--file-blue)', run: () => createBlank('docx') }] },
			{ label: t('new.feuilles'), icon: 'table_chart', color: 'var(--file-green)', submenu: [{ label: t('new.feuille_vierge'), icon: 'table_chart', color: 'var(--file-green)', run: () => createBlank('xlsx') }] },
			{ label: t('new.presentations'), icon: 'slideshow', color: 'var(--file-orange)', submenu: [{ label: t('new.presentation_vierge'), icon: 'slideshow', color: 'var(--file-orange)', run: () => createBlank('pptx') }] }
		);
	}
	items.push({
		label: t('new.plus'),
		submenu: [
			{ label: t('new.markdown'), icon: 'markdown', color: 'var(--file-blue)', run: () => createBlank('md') },
			{ label: t('new.fichier_texte'), icon: 'text_snippet', color: 'var(--file-gray)', run: () => createBlank('txt') }
		]
	});
	return items;
}

/** Drive-style "C then F/U/I" keyboard chords. Returns a cleanup function. */
export function installNewChords(): () => void {
	let armed = 0;
	const onkey = (e: KeyboardEvent) => {
		const el = e.target as HTMLElement | null;
		if (el && (el.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName))) return;
		if (e.altKey || e.metaKey) return;
		const k = e.key.toLowerCase();
		if (e.ctrlKey && k === 'c' && !window.getSelection()?.toString()) {
			armed = Date.now();
			return;
		}
		if (!armed) return;
		const fresh = Date.now() - armed < 1500;
		armed = 0;
		if (!fresh || e.ctrlKey || !nav.canWrite) return;
		if (k === 'f') dialogs.newFolder = { parent: nav.folderId };
		else if (k === 'u') pickers.files?.();
		else if (k === 'i') pickers.folder?.();
		else return;
		e.preventDefault();
	};
	window.addEventListener('keydown', onkey);
	return () => window.removeEventListener('keydown', onkey);
}
