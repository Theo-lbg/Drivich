// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

import type { NodeItem } from '$lib/types';

/** Where the user currently is; drives the "New" menu and drag-and-drop uploads. */
export const nav = $state<{ folderId: string | null; canWrite: boolean; tick: number }>({
	folderId: null,
	canWrite: true,
	tick: 0
});

/** Ask the visible list (and the sidebar tree) to reload. */
export function refreshAll() {
	nav.tick++;
}

export const dialogs = $state<{
	rename: NodeItem | null;
	share: NodeItem | null;
	move: NodeItem[] | null;
	confirmDelete: NodeItem[] | null;
	newFolder: { parent: string | null } | null;
	emptyTrash: boolean;
}>({ rename: null, share: null, move: null, confirmDelete: null, newFolder: null, emptyTrash: false });

export const viewer = $state<{ open: boolean; items: NodeItem[]; index: number; token: string | null }>({
	open: false,
	items: [],
	index: 0,
	token: null
});

export function openViewer(items: NodeItem[], index: number, token: string | null = null) {
	viewer.items = items;
	viewer.index = index;
	viewer.token = token;
	viewer.open = true;
}

/** Hidden file inputs live in the layout; the New menu calls these. */
export const pickers = $state<{ files: (() => void) | null; folder: (() => void) | null }>({ files: null, folder: null });
