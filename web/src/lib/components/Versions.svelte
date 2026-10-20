<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t, errorMessage } from '$lib/i18n/index.svelte';
	import { api, urls } from '$lib/api';
	import { askNewVersion } from '$lib/actions';
	import { formatBytes, formatDateLong } from '$lib/format';
	import type { NodeItem, VersionItem } from '$lib/types';
	import { refreshStorage } from '$lib/state/session.svelte';
	import { uploads } from '$lib/state/uploads.svelte';
	import { errorToast, toast } from '$lib/state/toast.svelte';
	import Icon from './Icon.svelte';

	let { node, onchanged }: { node: NodeItem; onchanged: (n: NodeItem) => void } = $props();

	let items = $state<VersionItem[]>([]);
	let keep = $state(0);
	let loading = $state(true);
	let error = $state('');

	const canEdit = $derived(node.role === 'owner' || node.role === 'editor');

	async function load() {
		const id = node.id;
		try {
			const r = await api.versions(id);
			if (node.id === id) {
				items = r.items;
				keep = r.max_versions;
				error = '';
			}
		} catch (e) {
			error = errorMessage(e);
		} finally {
			loading = false;
		}
	}

	// Reload when another file is selected, when its content changes, or when a version upload ends.
	$effect(() => {
		void node.id;
		void node.updated_at;
		void uploads.doneVersion;
		load();
	});

	async function restore(v: VersionItem) {
		try {
			const r = await api.restoreVersion(node.id, v.id);
			toast(t('versions.restored'));
			onchanged(r.node);
			refreshStorage();
			await load();
		} catch (e) {
			errorToast(e);
		}
	}

	async function remove(v: VersionItem) {
		try {
			await api.deleteVersion(node.id, v.id);
			items = items.filter((x) => x.id !== v.id);
			toast(t('versions.deleted'));
			refreshStorage();
		} catch (e) {
			errorToast(e);
		}
	}
</script>

<section class="versions" aria-label={t('versions.title')}>
	<h3>{t('versions.title')}</h3>
	<ul>
		<li class="cur">
			<div><strong>{t('versions.current')}</strong><span>{formatDateLong(node.updated_at)} · {formatBytes(node.size)}</span></div>
		</li>
		{#each items as v (v.id)}
			<li>
				<div><span class="when">{formatDateLong(v.saved_at)}</span><span>{formatBytes(v.size)}</span></div>
				<div class="acts">
					<a class="icon-btn small" href={urls.versionDownload(node.id, v.id)} title={t('versions.download')} aria-label={t('versions.download')}><Icon name="download" size={18} /></a>
					{#if canEdit}
						<button class="icon-btn small" title={t('versions.restore')} aria-label={t('versions.restore')} onclick={() => restore(v)}><Icon name="history" size={18} /></button>
						<button class="icon-btn small" title={t('versions.delete')} aria-label={t('versions.delete')} onclick={() => remove(v)}><Icon name="delete" size={18} /></button>
					{/if}
				</div>
			</li>
		{/each}
	</ul>
	{#if loading}<p class="hint">{t('file_browser.chargement')}</p>{:else if error}<p class="hint">{error}</p>{:else if !items.length}<p class="hint">{t('versions.empty')}</p>{/if}
	{#if canEdit}
		<button class="btn" onclick={() => askNewVersion(node)}><Icon name="upload_file" size={18} />{t('versions.upload')}</button>
		{#if keep}<p class="hint">{t('versions.keep_note', { count: keep })}</p>{/if}
	{/if}
</section>

<style>
	.versions {
		margin-top: 24px;
		border-top: 1px solid var(--line);
		padding-top: 16px;
	}
	h3 {
		font-size: 14px;
		margin: 0 0 8px;
	}
	ul {
		list-style: none;
		margin: 0 0 8px;
		padding: 0;
	}
	li {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		padding: 6px 0;
		font-size: 13px;
	}
	li div:first-child {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	li span {
		color: var(--text-2);
		font-size: 12px;
	}
	li .when {
		color: var(--text);
		font-size: 13px;
	}
	.acts {
		display: flex;
		flex: none;
	}
	.hint {
		color: var(--text-2);
		font-size: 12px;
		margin: 8px 0;
	}
</style>
