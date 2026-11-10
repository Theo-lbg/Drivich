<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { api } from '$lib/api';
	import { folderHref } from '$lib/actions';
	import { t } from '$lib/i18n/index.svelte';
	import { goto } from '$lib/nav';
	import { openViewer } from '$lib/state/app.svelte';
	import { toast } from '$lib/state/toast.svelte';

	// Target of "Copy link": opens the file in the viewer on top of its folder.
	// Access is checked by the server like everywhere else, so a link only works
	// for people who can already see the file.
	onMount(async () => {
		try {
			const n = await api.node(page.params.id ?? '');
			if (n.kind === 'folder') return goto(folderHref(n.id), { replaceState: true });
			if (n.trashed_at) {
				toast(t('toast.restore_to_open'));
				return goto('/trash', { replaceState: true });
			}
			const behind = n.role !== 'owner' ? '/shared' : n.parent_id ? folderHref(n.parent_id) : '/drive';
			await goto(behind, { replaceState: true });
			openViewer([n], 0);
			api.opened(n.id).catch(() => {});
		} catch {
			toast(t('file_page.introuvable'), { kind: 'error' });
			goto('/drive', { replaceState: true });
		}
	});
</script>

<svelte:head><title>Drivich</title></svelte:head>
<div class="boot" role="status"><span class="spinner"></span><span class="sr-only">{t('file_browser.chargement')}</span></div>

<style>
	.boot {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.spinner {
		width: 24px;
		height: 24px;
		border: 3px solid var(--accent-soft);
		border-top-color: var(--accent);
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
</style>
