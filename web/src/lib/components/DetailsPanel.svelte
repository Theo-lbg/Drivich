<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import { api } from '$lib/api';
	import { formatBytes, formatDateLong, typeLabel } from '$lib/format';
	import type { NodeItem } from '$lib/types';
	import { errorToast } from '$lib/state/toast.svelte';
	import Avatar from './Avatar.svelte';
	import Comments from './Comments.svelte';
	import Versions from './Versions.svelte';
	import Icon from './Icon.svelte';
	import Thumb from './Thumb.svelte';

	let { node, count = 0, onclose, onchanged }: { node: NodeItem | null; count?: number; onclose: () => void; onchanged: (n: NodeItem) => void } = $props();

	let desc = $state('');
	let editing = $state(false);

	$effect(() => {
		desc = node?.description ?? '';
		editing = false;
	});

	async function saveDesc() {
		if (!node) return;
		editing = false;
		if ((node.description ?? '') === desc.trim()) return;
		try {
			onchanged(await api.describe(node.id, desc.trim() || null));
		} catch (e) {
			errorToast(e);
		}
	}

	const canEdit = $derived(node?.role === 'owner' || node?.role === 'editor');
</script>

<aside class="details" aria-label={t('details_panel.details')}>
	<header>
		<h2 class="ellipsis">{node ? node.name : count > 1 ? t('details.selected_many', { count }) : t('details_panel.details')}</h2>
		<button class="icon-btn small" onclick={onclose} aria-label={t('details_panel.fermer_le_panneau_des_details')}><Icon name="close" size={20} /></button>
	</header>
	{#if node}
		<div class="preview"><Thumb {node} size={72} cover /></div>
		<dl>
			<dt>{t('advanced_search.type')}</dt>
			<dd>{typeLabel(node)}{node.mime && node.kind === 'file' ? ` (${node.mime})` : ''}</dd>
			{#if node.kind === 'file'}
				<dt>{t('details_panel.taille')}</dt>
				<dd>{formatBytes(node.size)}</dd>
			{/if}
			<dt>{t('details_panel.proprietaire')}</dt>
			<dd class="who"><Avatar name={node.owner_name} size={20} userId={node.owner_id} version={node.owner_avatar_version} /> {node.owner_name}</dd>
			<dt>{t('details_panel.modifie')}</dt>
			<dd>{formatDateLong(node.updated_at)}</dd>
			{#if node.last_opened_at}
				<dt>{t('details_panel.ouvert')}</dt>
				<dd>{formatDateLong(node.last_opened_at)}</dd>
			{/if}
			<dt>{t('details_panel.cree')}</dt>
			<dd>{formatDateLong(node.created_at)}</dd>
			<dt>{t('details_panel.description')}</dt>
			<dd>
				{#if editing && canEdit}
					<textarea class="field area" bind:value={desc} rows="3" maxlength="2000" onblur={saveDesc} aria-label={t('details_panel.description')}></textarea>
				{:else if canEdit}
					<button class="linkish" onclick={() => (editing = true)}>{node.description || t('details_panel.ajouter_une_description')}</button>
				{:else}
					{node.description || '—'}
				{/if}
			</dd>
		</dl>
		{#if node.kind === 'file' && !node.trashed_at}<Versions {node} {onchanged} />{/if}
		{#if !node.trashed_at}<Comments {node} />{/if}
	{:else}
		<p class="hint">{count > 1 ? t('details_panel.selectionnez_un_seul_element_pour') : t('details_panel.selectionnez_un_fichier_ou_un')}</p>
	{/if}
</aside>

<style>
	.details {
		width: 360px;
		flex: none;
		border-left: 1px solid var(--line);
		padding: 16px 20px;
		overflow: auto;
	}
	header {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-bottom: 16px;
	}
	h2 {
		flex: 1;
		font-size: 18px;
		line-height: 24px;
	}
	.preview {
		height: 160px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 12px;
		background: var(--surface-2);
		overflow: hidden;
		margin-bottom: 16px;
	}
	dl {
		margin: 0;
	}
	dt {
		margin-top: 16px;
		font-size: 12px;
		color: var(--text-2);
	}
	dd {
		margin: 2px 0 0;
		font-size: 14px;
		overflow-wrap: anywhere;
	}
	.hint {
		color: var(--text-2);
		margin: 24px 0;
	}
	.who {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.linkish {
		border: 0;
		background: transparent;
		padding: 0;
		color: var(--accent);
		text-align: left;
	}
	.area {
		height: auto;
		padding: 8px 12px;
		font-size: 14px;
	}
	@media (max-width: 1100px) {
		.details {
			position: absolute;
			right: 0;
			top: 0;
			bottom: 0;
			background: var(--surface);
			box-shadow: var(--shadow-2);
			z-index: 20;
			width: min(360px, 100%);
		}
	}
</style>
