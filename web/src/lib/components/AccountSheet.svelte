<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import { goto } from '$app/navigation';
	import { formatBytes } from '$lib/format';
	import { session, signOut, storageUsage } from '$lib/state/session.svelte';
	import { setTheme, ui } from '$lib/state/ui.svelte';
	import Avatar from './Avatar.svelte';
	import Icon from './Icon.svelte';

	const usage = $derived(storageUsage());
	const full = $derived(usage.pct >= 90);

	function go(href: string) {
		ui.account = false;
		goto(href);
	}

	async function logout() {
		ui.account = false;
		await signOut();
		goto('/login');
	}

	const themes = $derived([
		{ id: 'system', label: t('topbar.theme_automatique_appareil'), icon: 'contrast' },
		{ id: 'light', label: t('topbar.theme_clair'), icon: 'light_mode' },
		{ id: 'dark', label: t('topbar.theme_sombre'), icon: 'dark_mode' }
	] as const);
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && (ui.account = false)} />

{#if ui.account && session.user}
	<div class="sheet" role="dialog" aria-modal="true" aria-label={t('account.compte')}>
		<div class="bar"><button class="ok" onclick={() => (ui.account = false)}>{t('account.ok')}</button></div>

		<div class="card who">
			<Avatar name={session.user.name} size={72} userId={session.user.id} version={session.user.avatar_version} />
			<div class="id">
				<div class="name ellipsis">{session.user.name}</div>
				<div class="mail ellipsis">{session.user.email}</div>
			</div>
		</div>

		<div class="card">
			<button class="row" onclick={() => go('/settings')}><Icon name="manage_accounts" size={24} />{t('topbar.parametres_et_compte')}</button>
		</div>

		{#if usage.total}
			<div class="card stor" class:full>
				<div class="head"><Icon name={full ? 'error' : 'cloud'} size={24} fill={full} />{t('account.storage_pct', { pct: Math.round(usage.pct), total: formatBytes(usage.total) })}</div>
				<div class="meter" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(usage.pct)}><div style="width:{Math.max(usage.pct, usage.used > 0 ? 1 : 0)}%"></div></div>
				<div class="sub">{t('account.storage_used', { used: formatBytes(usage.used), total: formatBytes(usage.total) })}</div>
				<button class="link" onclick={() => go('/settings/storage')}>{t('sidebar.gerer_le_stockage')}</button>
			</div>
		{:else}
			<div class="card stor">
				<div class="head"><Icon name="cloud" size={24} />{t('sidebar.used', { used: formatBytes(usage.used) })}</div>
				<button class="link" onclick={() => go('/settings/storage')}>{t('sidebar.gerer_le_stockage')}</button>
			</div>
		{/if}

		<div class="card">
			{#each themes as th}
				<button class="row" onclick={() => setTheme(th.id)}>
					<Icon name={th.icon} size={24} />
					<span class="grow">{th.label}</span>
					{#if ui.theme === th.id}<Icon name="check" size={24} />{/if}
				</button>
			{/each}
		</div>

		<div class="card">
			<button class="row" onclick={logout}><Icon name="logout" size={24} />{t('settings.se_deconnecter')}</button>
		</div>
	</div>
{/if}

<style>
	.sheet {
		position: fixed;
		inset: 0;
		z-index: 90;
		overflow: auto;
		padding: 0 12px calc(16px + env(safe-area-inset-bottom));
		background: var(--bg);
		display: flex;
		flex-direction: column;
		gap: 4px;
	}
	.bar {
		display: flex;
		justify-content: flex-end;
		padding: 12px 12px 8px;
	}
	.ok {
		border: 0;
		background: transparent;
		color: var(--accent);
		font-size: 18px;
		font-weight: 500;
		padding: 8px 12px;
	}
	.card {
		background: var(--card);
		border-radius: 28px;
		padding: 8px 0;
		overflow: hidden;
	}
	.who {
		display: flex;
		align-items: center;
		gap: 16px;
		padding: 20px 24px;
	}
	.id {
		min-width: 0;
	}
	.name {
		font-family: var(--font);
		font-size: 22px;
		line-height: 28px;
	}
	.mail {
		color: var(--text-2);
		font-size: 16px;
	}
	.row {
		display: flex;
		align-items: center;
		gap: 16px;
		width: 100%;
		height: 56px;
		padding: 0 24px;
		border: 0;
		background: transparent;
		font-size: 16px;
		text-align: left;
	}
	.row:active {
		background: var(--hover-bg);
	}
	.grow {
		flex: 1;
	}
	.stor {
		padding: 20px 24px 12px;
	}
	.head {
		display: flex;
		align-items: center;
		gap: 16px;
		font-size: 16px;
	}
	.stor.full .head :global(svg) {
		color: var(--danger);
	}
	.meter {
		height: 6px;
		margin: 12px 0 12px 40px;
		border-radius: 3px;
		background: var(--bar-bg);
		overflow: hidden;
	}
	.meter div {
		height: 100%;
		background: var(--accent);
	}
	.full .meter div {
		background: var(--danger);
	}
	.sub {
		margin-left: 40px;
		color: var(--text-2);
		font-size: 14px;
	}
	.link {
		display: block;
		margin: 12px 0 0 28px;
		padding: 8px 12px;
		border: 0;
		background: transparent;
		color: var(--accent);
		font-size: 16px;
		font-weight: 500;
	}
</style>
