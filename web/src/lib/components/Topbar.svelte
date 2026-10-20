<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import { goto } from '$app/navigation';
	import { session, signOut } from '$lib/state/session.svelte';
	import { setTheme, ui } from '$lib/state/ui.svelte';
	import Avatar from './Avatar.svelte';
	import Icon from './Icon.svelte';
	import Menu from './Menu.svelte';
	import SearchBar from './SearchBar.svelte';
	import AppsMenu from './AppsMenu.svelte';

	let menu = $state<{ x: number; y: number } | null>(null);
	let apps = $state(false);

	function openAccount(e: MouseEvent) {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		menu = { x: r.right - 280, y: r.bottom + 8 };
	}

	async function logout() {
		await signOut();
		goto('/login');
	}
</script>

<header class="top">
	<a class="brand" href="/" aria-label={t('topbar.drivich_accueil')}>
		<img src="/logo.png" alt="" width="40" height="37" />
		<span>Drivich</span>
	</a>
	<SearchBar onmenu={() => (ui.drawer = !ui.drawer)} />
	<div class="right">
		<a class="icon-btn" href="https://github.com/Theo-lbg" target="_blank" rel="noreferrer" title={t('topbar.aide_et_projet')} aria-label={t('topbar.aide')}><Icon name="help" size={24} /></a>
		<a class="icon-btn" href="/settings" title={t('topbar.parametres')} aria-label={t('topbar.parametres')}><Icon name="settings" size={24} /></a>
		<button class="icon-btn" data-apps-button title={t('apps_menu.applications_drivich')} aria-label={t('app_rail.applications')} aria-haspopup="menu" aria-expanded={apps} onclick={() => (apps = !apps)}><Icon name="apps" size={24} /></button>
		{#if session.user}
			<button class="acct" onclick={openAccount} aria-label="Compte {session.user.name}" aria-haspopup="menu"><Avatar name={session.user.name} size={32} userId={session.user.id} version={session.user.avatar_version} /></button>
		{/if}
	</div>
</header>

{#if apps}<AppsMenu onclose={() => (apps = false)} />{/if}

{#if menu && session.user}
	<div class="card-wrap">
		<Menu
			x={menu.x}
			y={menu.y}
			minWidth={280}
			onclose={() => (menu = null)}
			items={[
				{ label: session.user.name, icon: 'person', disabled: true, hint: session.user.is_admin ? t('users.admin') : '' },
				{ label: session.user.email, disabled: true },
				{ divider: true },
				{ label: t('topbar.parametres_et_compte'), icon: 'manage_accounts', run: () => goto('/settings') },
				{ divider: true },
				{ label: t('topbar.theme_automatique_appareil'), icon: 'contrast', checked: ui.theme === 'system', run: () => setTheme('system') },
				{ label: t('topbar.theme_clair'), icon: 'light_mode', checked: ui.theme === 'light', run: () => setTheme('light') },
				{ label: t('topbar.theme_sombre'), icon: 'dark_mode', checked: ui.theme === 'dark', run: () => setTheme('dark') },
				{ divider: true },
				{ label: t('settings.se_deconnecter'), icon: 'logout', run: logout }
			]}
		/>
	</div>
{/if}

<style>
	.top {
		display: flex;
		align-items: center;
		gap: 8px;
		height: var(--topbar-h);
		padding: 0 12px 0 16px;
		flex: none;
	}
	.brand {
		display: flex;
		align-items: center;
		gap: 10px;
		width: calc(var(--sidebar-w) - 16px);
		color: var(--text-2);
		font-family: var(--font);
		font-size: 22px;
		letter-spacing: 0;
		text-decoration: none;
		flex: none;
	}
	.brand:hover {
		text-decoration: none;
	}
	.right {
		display: flex;
		align-items: center;
		gap: 4px;
		margin-left: auto;
	}
	.acct {
		border: 0;
		background: transparent;
		padding: 4px;
		margin-left: 8px;
		border-radius: 50%;
	}
	.acct:hover {
		box-shadow: 0 0 0 4px var(--hover-bg);
	}
	@media (max-width: 900px) {
		.brand {
			display: none;
		}
		.top {
			padding-left: 8px;
		}
	}
</style>
