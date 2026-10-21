// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

import type { MenuItem } from '$lib/components/Menu.svelte';
import { t } from '$lib/i18n/index.svelte';
import { dialogs, nav, pickers } from '$lib/state/app.svelte';

/** Entries of the "New" menu, shared by the sidebar button and the mobile floating button. */
export function newMenuItems(): MenuItem[] {
	return [
		{ label: t('dialogs.nouveau_dossier'), icon: 'create_new_folder', run: () => (dialogs.newFolder = { parent: nav.canWrite ? nav.folderId : null }) },
		{ divider: true },
		{ label: t('file_browser.importer_un_fichier'), icon: 'upload_file', run: () => pickers.files?.() },
		{ label: t('file_browser.importer_un_dossier'), icon: 'drive_folder_upload', run: () => pickers.folder?.() }
	];
}
