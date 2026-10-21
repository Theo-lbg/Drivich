<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import { page } from '$app/state';
	import Icon from './Icon.svelte';

	const path = $derived(page.url.pathname);
	const tabs = $derived([
		{ href: '/', label: t('sidebar.accueil'), icon: 'home', on: path === '/' },
		{ href: '/starred', label: t('advanced_search.favoris'), icon: 'star', on: path.startsWith('/starred') },
		{ href: '/shared', label: t('bottom_nav.partages'), icon: 'group', on: path.startsWith('/shared') },
		{ href: '/drive', label: t('bottom_nav.fichiers'), icon: 'folder', on: path.startsWith('/drive') }
	]);
</script>

<!-- Phone navigation, like Drive's bottom bar; the desktop keeps its sidebar. -->
<nav class="bnav" aria-label={t('sidebar.navigation_principale')}>
	{#each tabs as tab}
		<a href={tab.href} class:on={tab.on} aria-current={tab.on ? 'page' : undefined}>
			<span class="pill"><Icon name={tab.icon} size={24} fill={tab.on} /></span>
			<span class="lbl">{tab.label}</span>
		</a>
	{/each}
</nav>

<style>
	.bnav {
		display: none;
	}
	@media (max-width: 900px) {
		.bnav {
			position: fixed;
			z-index: 40;
			left: 0;
			right: 0;
			bottom: 0;
			display: flex;
			padding: 12px 8px calc(16px + env(safe-area-inset-bottom));
			background: var(--surface-2);
		}
		a {
			flex: 1;
			min-width: 0;
			display: flex;
			flex-direction: column;
			align-items: center;
			gap: 4px;
			color: var(--text-2);
			font-size: 12px;
			line-height: 16px;
			text-decoration: none;
		}
		.pill {
			display: flex;
			align-items: center;
			justify-content: center;
			width: 64px;
			height: 32px;
			border-radius: 16px;
		}
		a.on {
			color: var(--text);
			font-weight: 500;
		}
		a.on .pill {
			background: var(--accent-soft);
			color: var(--accent-soft-text);
		}
		.lbl {
			max-width: 100%;
			overflow: hidden;
			text-overflow: ellipsis;
			white-space: nowrap;
		}
	}
</style>
