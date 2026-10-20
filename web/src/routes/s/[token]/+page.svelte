<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import { api, ApiError, urls } from '$lib/api';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import Thumb from '$lib/components/Thumb.svelte';
	import Viewer from '$lib/components/Viewer.svelte';
	import { formatBytes, formatDate, isPreviewable } from '$lib/format';
	import { openViewer } from '$lib/state/app.svelte';
	import type { Crumb, NodeItem } from '$lib/types';

	const token = $derived(page.params.token ?? '');
	let root = $state<NodeItem | null>(null);
	let items = $state<NodeItem[]>([]);
	let trail = $state<Crumb[]>([]);
	let cursor = $state<string | null>(null);
	let missing = $state(false);
	let loading = $state(true);

	onMount(async () => {
		try {
			root = (await api.publicRoot(token)).root;
			trail = [{ id: root.id, name: root.name }];
			if (root.kind === 'folder') await load(root.id);
			else items = [root];
		} catch (e) {
			missing = e instanceof ApiError;
		} finally {
			loading = false;
		}
	});

	async function load(id: string, more = false) {
		const p = await api.publicChildren(token, id, more ? cursor : null);
		items = more ? items.concat(p.items) : p.items;
		cursor = p.next_cursor;
	}

	async function enter(n: NodeItem) {
		if (n.kind === 'folder') {
			trail = [...trail, { id: n.id, name: n.name }];
			await load(n.id);
		} else if (isPreviewable(n)) {
			openViewer(items.filter((i) => i.kind === 'file' && isPreviewable(i)), items.filter((i) => i.kind === 'file' && isPreviewable(i)).findIndex((i) => i.id === n.id), token);
		} else {
			location.href = urls.publicDownload(token, n.id);
		}
	}

	async function jump(i: number) {
		trail = trail.slice(0, i + 1);
		await load(trail[i].id);
	}
</script>

<svelte:head><title>{root ? `${root.name} – ` : ''}Drivich</title></svelte:head>

<div class="pub">
	<header>
		<a href="/" class="brand"><img src="/logo.png" alt="" width="36" /><span>Drivich</span></a>
		<span class="tag">{t('s.partage_public_lecture_seule')}</span>
	</header>
	<main>
		{#if loading}
			<p class="muted c">{t('file_browser.chargement')}</p>
		{:else if missing || !root}
			<EmptyState title={t('s.ce_lien_nest_plus_disponible')} text="Il a peut-être été révoqué, il a expiré ou l’élément a été supprimé." art="share" />
		{:else}
			<nav class="crumbs" aria-label={t('file_browser.fil_dariane')}>
				{#each trail as c, i}
					{#if i > 0}<Icon name="chevron_right" size={20} />{/if}
					<button class:cur={i === trail.length - 1} onclick={() => jump(i)}>{c.name}</button>
				{/each}
				<a class="btn outlined zip" href={urls.publicZip(token, trail[trail.length - 1].id)}><Icon name="download" size={18} />{t('file_browser.telecharger_zip')}</a>
			</nav>
			<ul>
				{#each items as n (n.id)}
					<li>
						<button class="row" onclick={() => enter(n)}>
							<Thumb node={n} size={24} />
							<span class="n ellipsis">{n.name}</span>
							<span class="d">{formatDate(n.updated_at)}</span>
							<span class="s">{n.kind === 'folder' ? '—' : formatBytes(n.size)}</span>
						</button>
						{#if n.kind === 'file'}<a class="icon-btn small" href={urls.publicDownload(token, n.id)} download={n.name} aria-label={t('public.download_name', { name: n.name })}><Icon name="download" size={20} /></a>{/if}
					</li>
				{/each}
			</ul>
			{#if !items.length}<p class="muted c">{t('s.ce_dossier_est_vide')}</p>{/if}
			{#if cursor}<div class="c"><button class="btn outlined" onclick={() => load(trail[trail.length - 1].id, true)}>{t('s.afficher_plus')}</button></div>{/if}
		{/if}
	</main>
	<Viewer />
</div>

<style>
	.zip {
		margin-left: auto;
	}
	.pub {
		height: 100%;
		display: flex;
		flex-direction: column;
	}
	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		height: 64px;
		padding: 0 24px;
	}
	.brand {
		display: flex;
		align-items: center;
		gap: 10px;
		color: var(--text-2);
		font-family: var(--font);
		font-size: 22px;
		text-decoration: none;
	}
	.tag {
		color: var(--text-2);
	}
	main {
		flex: 1;
		overflow: auto;
		margin: 0 16px 16px;
		padding: 16px 24px;
		background: var(--surface);
		border-radius: var(--radius-panel);
	}
	.crumbs {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		margin-bottom: 8px;
		font-family: var(--font);
		font-size: 22px;
	}
	.crumbs button {
		border: 0;
		background: transparent;
		padding: 6px 10px;
		border-radius: 18px;
		color: var(--text-2);
		font: inherit;
	}
	.crumbs button:hover {
		background: var(--hover-bg);
	}
	.crumbs .cur {
		color: var(--text);
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	li {
		display: flex;
		align-items: center;
		border-bottom: 1px solid var(--line);
	}
	.row {
		flex: 1;
		min-width: 0;
		display: grid;
		grid-template-columns: 24px minmax(120px, 1fr) 100px 90px;
		align-items: center;
		gap: 16px;
		min-height: 48px;
		border: 0;
		background: transparent;
		text-align: left;
		padding: 0 8px;
	}
	.row:hover {
		background: var(--surface-2);
	}
	.n {
		font-weight: 500;
	}
	.d,
	.s {
		color: var(--text-2);
	}
	.c {
		text-align: center;
		padding: 32px;
	}
	@media (max-width: 600px) {
		.row {
			grid-template-columns: 24px 1fr;
		}
		.d,
		.s {
			display: none;
		}
	}
</style>
