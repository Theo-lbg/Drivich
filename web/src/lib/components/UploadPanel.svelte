<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import { formatBytes } from '$lib/format';
	import { activeCount, cancel, clearFinished, retry, uploads } from '$lib/state/uploads.svelte';
	import Icon from './Icon.svelte';

	const active = $derived(uploads.jobs.filter((j) => j.status === 'queued' || j.status === 'uploading').length);
	const done = $derived(uploads.jobs.filter((j) => j.status === 'done').length);
	const failed = $derived(uploads.jobs.filter((j) => j.status === 'error').length);
	const title = $derived(
		active > 0 ? t('upload_panel.uploading', { count: active }) : failed > 0 ? t('upload_panel.failures', { count: failed }) : t('upload_panel.done', { count: done })
	);
	const total = $derived(uploads.jobs.reduce((a, j) => a + j.size, 0));
	const sent = $derived(uploads.jobs.reduce((a, j) => a + (j.status === 'done' ? j.size : j.sent), 0));
</script>

{#if uploads.jobs.length}
	<section class="panel" aria-label={t('upload_panel.importations_en_cours')}>
		<header>
			<h2>{title}</h2>
			<button class="icon-btn small" aria-label={uploads.collapsed ? t('folder_tree.developper') : t('folder_tree.reduire')} onclick={() => (uploads.collapsed = !uploads.collapsed)}>
				<Icon name={uploads.collapsed ? 'keyboard_arrow_up' : 'keyboard_arrow_down'} size={24} />
			</button>
			<button class="icon-btn small" aria-label={t('toasts.fermer')} onclick={() => (activeCount() ? (uploads.collapsed = true) : clearFinished())}><Icon name="close" size={22} /></button>
		</header>
		{#if active > 0}
			<div class="overall" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={total ? Math.round((sent / total) * 100) : 0}><div style="width:{total ? (sent / total) * 100 : 0}%"></div></div>
		{/if}
		{#if !uploads.collapsed}
			<ul>
				{#each uploads.jobs as j (j.id)}
					<li class:err={j.status === 'error'}>
						<Icon name="description" size={22} />
						<div class="info">
							<span class="n ellipsis" title={j.name}>{j.name}</span>
							{#if j.status === 'uploading'}
								<div class="bar"><div style="width:{j.size ? (j.sent / j.size) * 100 : 0}%"></div></div>
								<span class="s">{t('upload_panel.progress', { sent: formatBytes(j.sent), total: formatBytes(j.size) })}</span>
							{:else if j.status === 'error'}
								<span class="s e">{j.error}</span>
							{:else if j.status === 'queued'}
								<span class="s">{t('upload_panel.en_attente')}</span>
							{:else if j.status === 'canceled'}
								<span class="s">{t('upload_panel.annule')}</span>
							{/if}
						</div>
						{#if j.status === 'done'}
							<span class="ok" title={t('upload_panel.termine')}><Icon name="check_circle" size={24} fill /></span>
						{:else if j.status === 'error'}
							<button class="icon-btn small" title={t('upload_panel.reessayer')} aria-label={t('upload_panel.reessayer')} onclick={() => retry(j)}><Icon name="refresh" size={20} /></button>
						{:else if j.status !== 'canceled'}
							<button class="icon-btn small" title={t('dialogs.annuler')} aria-label="Annuler l’importation de {j.name}" onclick={() => cancel(j)}><Icon name="close" size={20} /></button>
						{/if}
					</li>
				{/each}
			</ul>
		{/if}
	</section>
{/if}

<style>
	.panel {
		position: fixed;
		right: 24px;
		bottom: 0;
		z-index: 800;
		width: min(360px, calc(100vw - 16px));
		background: var(--elevated);
		border-radius: 8px 8px 0 0;
		box-shadow: var(--shadow-2);
		overflow: hidden;
	}
	header {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 8px 8px 8px 24px;
		background: var(--surface-2);
		height: 56px;
	}
	h2 {
		flex: 1;
		font-family: var(--font);
		font-size: 16px;
		font-weight: 500;
	}
	.overall {
		height: 3px;
		background: var(--accent-soft);
	}
	.overall div {
		height: 100%;
		background: var(--accent);
		transition: width 0.2s;
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
		max-height: min(300px, 40vh);
		overflow: auto;
	}
	li {
		display: flex;
		align-items: center;
		gap: 16px;
		padding: 8px 12px 8px 24px;
		min-height: 56px;
		border-bottom: 1px solid var(--line);
		color: var(--text-2);
	}
	.info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.n {
		color: var(--text);
	}
	.s {
		font-size: 12px;
		color: var(--text-2);
	}
	.s.e {
		color: var(--danger);
	}
	.bar {
		height: 4px;
		border-radius: 2px;
		background: var(--accent-soft);
		overflow: hidden;
	}
	.bar div {
		height: 100%;
		background: var(--accent);
	}
	.ok {
		color: var(--ok);
		display: inline-flex;
	}
</style>
