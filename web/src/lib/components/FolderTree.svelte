<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import { api } from '$lib/api';
	import { folderHref } from '$lib/actions';
	import { nav } from '$lib/state/app.svelte';
	import type { NodeItem } from '$lib/types';
	import FolderTree from './FolderTree.svelte';
	import Icon from './Icon.svelte';

	let {
		parentId = null,
		depth = 0,
		activeId = null,
		onselect = null,
		exclude = []
	}: {
		parentId?: string | null;
		depth?: number;
		activeId?: string | null;
		/** Picker mode: choose instead of navigating. */
		onselect?: ((id: string) => void) | null;
		exclude?: string[];
	} = $props();

	let folders = $state<NodeItem[]>([]);
	let open = $state<Record<string, boolean>>({});
	let ready = $state(false);

	async function load() {
		try {
			const page = await api.list({ view: 'drive', parent: parentId, type: 'folder', limit: 200, sort: 'name', order: 'asc' });
			folders = page.items.filter((f) => !exclude.includes(f.id));
		} catch {
			folders = [];
		}
		ready = true;
	}

	$effect(() => {
		nav.tick;
		parentId;
		load();
	});
</script>

{#each folders as f (f.id)}
	<div class="node" style="padding-left:{depth * 16}px">
		<button class="tw" aria-label={open[f.id] ? t('folder_tree.reduire') : t('folder_tree.developper')} aria-expanded={!!open[f.id]} onclick={() => (open[f.id] = !open[f.id])}>
			<Icon name={open[f.id] ? 'keyboard_arrow_down' : 'chevron_right'} size={18} />
		</button>
		{#if onselect}
			<button class="lbl" class:act={activeId === f.id} onclick={() => onselect(f.id)}>
				<Icon name="folder" size={20} fill /><span class="ellipsis">{f.name}</span>
			</button>
		{:else}
			<a class="lbl" class:act={activeId === f.id} href={folderHref(f.id)}>
				<Icon name="folder" size={20} fill /><span class="ellipsis">{f.name}</span>
			</a>
		{/if}
	</div>
	{#if open[f.id]}
		<FolderTree parentId={f.id} depth={depth + 1} {activeId} {onselect} {exclude} />
	{/if}
{/each}

<style>
	.node {
		display: flex;
		align-items: center;
		min-width: 0;
	}
	.tw {
		width: 24px;
		height: 32px;
		border: 0;
		background: transparent;
		color: var(--text-2);
		padding: 0;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		flex: none;
	}
	.lbl {
		display: flex;
		align-items: center;
		gap: 12px;
		flex: 1;
		min-width: 0;
		height: 32px;
		padding: 0 12px 0 4px;
		border: 0;
		border-radius: 16px;
		background: transparent;
		color: var(--text);
		text-align: left;
		text-decoration: none;
		color: var(--icon-gray);
	}
	.lbl span {
		color: var(--text);
	}
	.lbl:hover {
		background: var(--hover-bg);
		text-decoration: none;
	}
	.lbl.act {
		background: var(--accent-soft);
	}
</style>
