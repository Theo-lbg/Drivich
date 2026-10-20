<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import { page } from '$app/state';
	import { formatBytes, pct } from '$lib/format';
	import { dialogs, nav, pickers } from '$lib/state/app.svelte';
	import { session } from '$lib/state/session.svelte';
	import { ui } from '$lib/state/ui.svelte';
	import FolderTree from './FolderTree.svelte';
	import Icon from './Icon.svelte';
	import Menu from './Menu.svelte';

	let menu = $state<{ x: number; y: number } | null>(null);
	let driveOpen = $state(false);

	const path = $derived(page.url.pathname);
	const activeFolder = $derived(path.startsWith('/drive/folders/') ? path.split('/')[3] : null);
	const items = $derived([
		{ href: '/', label: t('sidebar.accueil'), icon: 'home', match: (p: string) => p === '/' },
		{ href: '/shared', label: t('advanced_search.partages_avec_moi'), icon: 'group', match: (p: string) => p.startsWith('/shared') },
		{ href: '/recent', label: t('recent.recents'), icon: 'schedule', match: (p: string) => p.startsWith('/recent') },
		{ href: '/starred', label: t('advanced_search.favoris'), icon: 'star', match: (p: string) => p.startsWith('/starred') },
		{ href: '/trash', label: t('advanced_search.corbeille'), icon: 'delete', match: (p: string) => p.startsWith('/trash') }
	]);
	const blocked = $derived(nav.folderId !== null && !nav.canWrite);

	function openNew(e: MouseEvent) {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		menu = { x: r.left, y: r.bottom + 4 };
	}

	const total = $derived(session.quota ?? session.disk?.total_bytes ?? null);
	const limitLabel = $derived(session.quota ? formatBytes(session.quota) : session.disk ? t('sidebar.disk_total', { size: formatBytes(session.disk.total_bytes) }) : '');
</script>

{#if ui.drawer}<button class="scrim" aria-label={t('sidebar.fermer_le_menu')} onclick={() => (ui.drawer = false)}></button>{/if}
<nav class="side" class:open={ui.drawer} aria-label={t('sidebar.navigation_principale')}>
	<button class="new" onclick={openNew} disabled={blocked} title={blocked ? t('sidebar.vous_ne_pouvez_pas_modifier') : ''} aria-haspopup="menu">
		<Icon name="add" size={28} /> <span>{t('sidebar.nouveau')}</span>
	</button>

	<ul onclick={() => (ui.drawer = false)} onkeydown={() => {}} role="presentation">
		<li>
			<a class="item" class:act={path === '/' } href="/"><Icon name="home" size={24} fill={path === '/'} />{t('sidebar.accueil')}</a>
		</li>
		<li>
			<div class="item drive" class:act={path === '/drive' || path.startsWith('/drive/')}>
				<button class="tw" aria-label={driveOpen ? t('sidebar.reduire_mon_drive') : t('sidebar.developper_mon_drive')} aria-expanded={driveOpen} onclick={() => (driveOpen = !driveOpen)}>
					<Icon name={driveOpen ? 'keyboard_arrow_down' : 'chevron_right'} size={18} />
				</button>
				<a href="/drive" class="lnk"><Icon name="folder" size={24} fill={path === '/drive'} />{t('advanced_search.mon_drive')}</a>
			</div>
			{#if driveOpen}
				<div class="tree"><FolderTree parentId={null} depth={1} activeId={activeFolder} /></div>
			{/if}
		</li>
		<li class="gap"></li>
		{#each items.slice(1, 4) as it}
			<li><a class="item" class:act={it.match(path)} href={it.href}><Icon name={it.icon} size={24} fill={it.match(path) && it.icon === 'star'} />{it.label}</a></li>
		{/each}
		<li class="gap"></li>
		<li><a class="item" class:act={path.startsWith('/trash')} href="/trash"><Icon name="delete" size={24} />{t('advanced_search.corbeille')}</a></li>
		<li><a class="item" class:act={path.startsWith('/settings/storage')} href="/settings/storage"><Icon name="cloud" size={24} />{t('sidebar.espace_de_stockage')}</a></li>
	</ul>

	<div class="storage">
		<div class="bar" role="progressbar" aria-label={t('sidebar.espace_de_stockage_utilise')} aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(pct(session.used, total))}>
			<div style="width:{Math.max(pct(session.used, total), session.used > 0 ? 1 : 0)}%"></div>
		</div>
		<p>{limitLabel ? t('sidebar.used_of', { used: formatBytes(session.used), total: limitLabel }) : t('sidebar.used', { used: formatBytes(session.used) })}</p>
		<a class="btn outlined manage" href="/settings/storage">{t('sidebar.gerer_le_stockage')}</a>
	</div>
</nav>

{#if menu}
	<Menu
		x={menu.x}
		y={menu.y}
		minWidth={260}
		onclose={() => (menu = null)}
		items={[
			{ label: t('dialogs.nouveau_dossier'), icon: 'create_new_folder', run: () => (dialogs.newFolder = { parent: nav.canWrite ? nav.folderId : null }) },
			{ divider: true },
			{ label: t('file_browser.importer_un_fichier'), icon: 'upload_file', run: () => pickers.files?.() },
			{ label: t('file_browser.importer_un_dossier'), icon: 'drive_folder_upload', run: () => pickers.folder?.() }
		]}
	/>
{/if}

<style>
	.side {
		display: flex;
		flex-direction: column;
		width: var(--sidebar-w);
		flex: none;
		padding: 0 8px 8px 12px;
		overflow: auto;
	}
	.new {
		display: inline-flex;
		align-items: center;
		gap: 12px;
		width: 128px;
		height: 56px;
		margin: 8px 0 16px 4px;
		padding: 0 20px 0 16px;
		border: 0;
		border-radius: 16px;
		background: var(--card);
		box-shadow: var(--shadow-1);
		color: var(--text);
		font-family: var(--font);
		font-weight: 500;
		font-size: 14px;
	}
	.new:hover:not(:disabled) {
		box-shadow: var(--shadow-2);
		background: var(--new-hover);
	}
	.new:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.gap {
		height: 16px;
	}
	.item {
		display: flex;
		align-items: center;
		gap: 12px;
		height: 32px;
		padding: 0 12px 0 12px;
		border-radius: 16px;
		color: var(--text);
		text-decoration: none;
		font-size: 14px;
		height: 40px;
	}
	.item:hover {
		background: var(--hover-bg);
		text-decoration: none;
	}
	.item.act {
		background: var(--accent-soft);
		color: var(--accent-soft-text);
		font-weight: 500;
	}
	.item.drive {
		padding: 0;
		gap: 0;
	}
	.item.drive:hover {
		background: transparent;
	}
	.item.drive.act {
		background: var(--accent-soft);
	}
	.tw {
		width: 24px;
		height: 40px;
		border: 0;
		background: transparent;
		padding: 0;
		margin-left: -4px;
		color: var(--text-2);
		display: inline-flex;
		align-items: center;
		justify-content: center;
	}
	.lnk {
		display: flex;
		align-items: center;
		gap: 12px;
		flex: 1;
		height: 40px;
		padding-right: 12px;
		color: inherit;
		text-decoration: none;
		border-radius: 16px;
	}
	.item.drive:not(.act) .lnk:hover {
		background: var(--hover-bg);
	}
	.tree {
		margin: 2px 0 4px 8px;
	}
	.storage {
		margin-top: auto;
		padding: 16px 12px 0;
	}
	.bar {
		height: 4px;
		border-radius: 2px;
		background: var(--bar-bg);
		overflow: hidden;
	}
	.bar div {
		height: 100%;
		background: var(--accent);
		border-radius: 2px;
	}
	.storage p {
		margin: 12px 0;
		color: var(--text-2);
	}
	.manage {
		height: 40px;
		width: 100%;
		padding: 0 12px;
		text-decoration: none;
	}
	.scrim {
		display: none;
	}
	@media (max-width: 900px) {
		.side {
			position: fixed;
			z-index: 70;
			top: 0;
			bottom: 0;
			left: 0;
			padding-top: 8px;
			background: var(--bg);
			box-shadow: var(--shadow-2);
			transform: translateX(-105%);
			transition: transform 0.18s ease-out;
		}
		.side.open {
			transform: none;
		}
		.scrim {
			display: block;
			position: fixed;
			inset: 0;
			z-index: 65;
			border: 0;
			background: var(--scrim);
		}
	}
</style>
