<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { folderHref, openNode } from '$lib/actions';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import Thumb from '$lib/components/Thumb.svelte';
	import { formatBytes, formatDate, typeLabel } from '$lib/format';
	import { nav } from '$lib/state/app.svelte';
	import { session } from '$lib/state/session.svelte';
	import { errorToast } from '$lib/state/toast.svelte';
	import { enqueueWithPaths, filesFromDataTransfer } from '$lib/state/uploads.svelte';
	import type { NodeItem } from '$lib/types';

	let recent = $state<NodeItem[]>([]);
	let folders = $state<NodeItem[]>([]);
	let starred = $state<NodeItem[]>([]);
	let loaded = $state(false);

	async function load() {
		try {
			const [r, f, s] = await Promise.all([
				api.list({ view: 'recent', limit: 12 }),
				api.list({ view: 'drive', parent: null, type: 'folder', limit: 8, sort: 'modified', order: 'desc' }),
				api.list({ view: 'starred', limit: 8 })
			]);
			recent = r.items;
			folders = f.items;
			starred = s.items;
		} catch (e) {
			errorToast(e);
		}
		loaded = true;
	}

	$effect(() => {
		nav.tick;
		load();
	});

	onMount(() => {
		nav.folderId = null;
		nav.canWrite = true;
	});

	const suggestions = $derived(recent.slice(0, 4));
	const hour = new Date().getHours();
	const greeting = $derived(hour >= 18 || hour < 5 ? t('routes.bonsoir') : t('routes.bonjour'));

	async function onDrop(e: DragEvent) {
		if (!e.dataTransfer?.types.includes('Files')) return;
		e.preventDefault();
		const list = await filesFromDataTransfer(e.dataTransfer);
		if (list.length) enqueueWithPaths(list, null);
	}
</script>

<svelte:head><title>{t('routes.accueil_drivich')}</title></svelte:head>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<section class="home" ondragover={(e) => e.dataTransfer?.types.includes('Files') && e.preventDefault()} ondrop={onDrop}>
	<h1>{session.user ? t('home.welcome_named', { greeting, name: session.user.name.split(' ')[0] }) : t('home.welcome', { greeting })}</h1>

	{#if loaded && !recent.length && !folders.length && !starred.length}
		<EmptyState title={t('routes.un_emplacement_centralise_pour_tous')} text="Faites glisser vos fichiers et dossiers ici ou utilisez le bouton « Nouveau » pour les importer." />
	{:else}
		{#if suggestions.length}
			<h2 class="sugg">{t('routes.suggestions')}</h2>
			<div class="cards sugg">
				{#each suggestions as n (n.id)}
					<button class="card" onclick={() => openNode(n, recent)}>
						<span class="head"><Thumb node={n} size={20} /><span class="n ellipsis">{n.name}</span></span>
						<span class="prev"><Thumb node={n} size={64} cover /></span>
						<span class="meta">{typeLabel(n)} · {formatDate(n.last_opened_at ?? n.updated_at)}</span>
					</button>
				{/each}
			</div>
		{/if}

		{#if folders.length}
			<h2>{t('advanced_search.dossiers')}</h2>
			<div class="folders">
				{#each folders as f (f.id)}
					<a class="folder" href={folderHref(f.id)}><Icon name="folder" size={24} fill class="fi" /><span class="ellipsis">{f.name}</span></a>
				{/each}
			</div>
		{/if}

		{#if starred.length}
			<h2>{t('advanced_search.favoris')}</h2>
			<div class="folders">
				{#each starred as n (n.id)}
					<button class="folder" onclick={() => openNode(n, starred)}><Thumb node={n} size={24} /><span class="ellipsis">{n.name}</span></button>
				{/each}
			</div>
		{/if}

		{#if recent.length}
			<h2>{t('routes.fichiers_recents')}</h2>
			<ul class="list">
				{#each recent as n (n.id)}
					<li>
						<button onclick={() => openNode(n, recent)}>
							<Thumb node={n} size={24} />
							<span class="lname"><span class="ln ellipsis">{n.name}</span><span class="lsub ellipsis">{(n.owner_id === session.user?.id ? 'moi' : n.owner_name) + ' · ' + formatDate(n.last_opened_at ?? n.updated_at)}</span></span>
							<span class="lo ellipsis">{n.owner_id === session.user?.id ? 'moi' : n.owner_name}</span>
							<span class="ld">{formatDate(n.last_opened_at ?? n.updated_at)}</span>
							<span class="ls">{formatBytes(n.size)}</span>
						</button>
					</li>
				{/each}
			</ul>
		{/if}
	{/if}
</section>

<style>
	.home {
		flex: 1;
		min-width: 0;
		overflow: auto;
		background: var(--surface);
		border-radius: var(--radius-panel);
		padding: 24px 32px 40px;
	}
	h1 {
		font-size: 28px;
		line-height: 36px;
		margin-bottom: 8px;
	}
	h2 {
		margin: 28px 0 12px;
		font-size: 16px;
		font-weight: 500;
		color: var(--text-2);
	}
	.cards {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
		gap: 16px;
	}
	.card {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 0 0 12px;
		border: 1px solid var(--line);
		border-radius: 12px;
		background: var(--card);
		text-align: left;
		overflow: hidden;
	}
	.card:hover {
		background: var(--card-hover);
	}
	.head {
		display: flex;
		align-items: center;
		gap: 12px;
		height: 48px;
		padding: 0 16px;
	}
	.n {
		font-weight: 500;
	}
	.prev {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 150px;
		margin: 0 8px;
		border-radius: 8px;
		background: var(--media-bg);
		overflow: hidden;
	}
	.meta {
		padding: 0 16px;
		color: var(--text-2);
		font-size: 12px;
	}
	.folders {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
		gap: 12px;
	}
	.folder {
		display: flex;
		align-items: center;
		gap: 12px;
		height: 48px;
		padding: 0 16px;
		border: 1px solid var(--line);
		border-radius: 12px;
		background: var(--card);
		color: var(--text);
		text-decoration: none;
		text-align: left;
		font-weight: 500;
	}
	.folder:hover {
		background: var(--card-hover);
		text-decoration: none;
	}
	:global(.fi) {
		color: var(--icon-gray);
	}
	.list {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.list button {
		display: grid;
		grid-template-columns: 24px minmax(160px, 1fr) 140px 100px 90px;
		align-items: center;
		gap: 16px;
		width: 100%;
		min-height: 48px;
		padding: 0 16px;
		border: 0;
		border-bottom: 1px solid var(--line);
		background: transparent;
		text-align: left;
	}
	.list button:hover {
		background: var(--surface-2);
	}
	.lname {
		display: flex;
		min-width: 0;
	}
	.ln {
		font-weight: 500;
	}
	.lsub {
		display: none;
	}
	.lo,
	.ld,
	.ls {
		color: var(--text-2);
	}
	@media (max-width: 900px) {
		.home {
			padding: 8px 0 16px;
			border-radius: 0;
		}
		h1 {
			margin: 0 16px;
			font-size: 22px;
			line-height: 28px;
		}
		h2 {
			margin: 20px 16px 8px;
		}
		.sugg {
			display: none;
		}
		.folders {
			margin: 0 16px;
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
		.list button {
			grid-template-columns: 24px minmax(0, 1fr);
			gap: 16px;
			min-height: 72px;
			border-bottom: 0;
		}
		.lname {
			flex-direction: column;
			gap: 2px;
		}
		.ln {
			font-weight: 400;
			font-size: 16px;
			line-height: 24px;
		}
		.lsub {
			display: block;
			color: var(--text-2);
			font-size: 14px;
		}
		.lo,
		.ld,
		.ls {
			display: none;
		}
	}
</style>
