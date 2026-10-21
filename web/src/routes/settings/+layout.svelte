<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import { page } from '$app/state';
	import type { Snippet } from 'svelte';
	import Icon from '$lib/components/Icon.svelte';
	import { session } from '$lib/state/session.svelte';

	let { children }: { children: Snippet } = $props();
	const path = $derived(page.url.pathname);
	const tabs = $derived([
		{ href: '/settings', label: t('layout_settings.compte'), icon: 'manage_accounts', on: path === '/settings' },
		{ href: '/settings/storage', label: t('layout_settings.stockage'), icon: 'cloud', on: path.startsWith('/settings/storage') },
		...(session.user?.is_admin
			? [
					{ href: '/settings/users', label: t('server.utilisateurs'), icon: 'group', on: path.startsWith('/settings/users') },
					{ href: '/settings/server', label: t('server.serveur'), icon: 'dns', on: path.startsWith('/settings/server') }
				]
			: [])
	]);
</script>

<svelte:head><title>{t('layout_settings.parametres_drivich')}</title></svelte:head>

<section class="settings">
	<aside>
		<h1>{t('topbar.parametres')}</h1>
		<nav aria-label={t('topbar.parametres')}>
			{#each tabs as tab}
				<a href={tab.href} class:on={tab.on}><Icon name={tab.icon} size={22} />{tab.label}</a>
			{/each}
		</nav>
	</aside>
	<div class="content">{@render children()}</div>
</section>

<style>
	.settings {
		flex: 1;
		min-width: 0;
		display: flex;
		background: var(--surface);
		border-radius: var(--radius-panel);
		overflow: hidden;
	}
	aside {
		width: 260px;
		flex: none;
		padding: 24px 12px;
		border-right: 1px solid var(--line);
	}
	h1 {
		font-size: 22px;
		margin: 0 12px 16px;
	}
	nav {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	a {
		display: flex;
		align-items: center;
		gap: 12px;
		height: 40px;
		padding: 0 16px;
		border-radius: 20px;
		color: var(--text);
		text-decoration: none;
	}
	a:hover {
		background: var(--hover-bg);
		text-decoration: none;
	}
	a.on {
		background: var(--accent-soft);
		color: var(--accent-soft-text);
		font-weight: 500;
	}
	.content {
		flex: 1;
		min-width: 0;
		overflow: auto;
		padding: 24px 32px 48px;
	}
	@media (max-width: 900px) {
		.settings {
			flex-direction: column;
			border-radius: 0;
		}
		aside {
			width: auto;
			border-right: 0;
			border-bottom: 1px solid var(--line);
			padding: 12px 8px 4px;
		}
		h1 {
			margin: 0 8px 8px;
		}
		/* Tabs share the width instead of scrolling sideways. */
		nav {
			flex-direction: row;
			gap: 4px;
		}
		a {
			flex: 1;
			min-width: 0;
			flex-direction: column;
			justify-content: center;
			gap: 2px;
			height: 56px;
			padding: 0 4px;
			border-radius: 16px;
			font-size: 12px;
			text-align: center;
		}
		.content {
			padding: 16px;
		}
	}
</style>
