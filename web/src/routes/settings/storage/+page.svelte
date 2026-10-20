<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import { onMount } from 'svelte';
	import { formatBytes, pct } from '$lib/format';
	import { refreshStorage, session } from '$lib/state/session.svelte';

	onMount(refreshStorage);

	const total = $derived(session.quota ?? session.disk?.total_bytes ?? null);
	const free = $derived(session.quota !== null ? Math.max(0, session.quota - session.used) : (session.disk?.free_bytes ?? null));
</script>

<h2>{t('sidebar.espace_de_stockage')}</h2>
<p class="lead">
	{session.quota !== null ? t('storage.used_of_quota', { used: formatBytes(session.used), quota: formatBytes(session.quota) }) : t('storage.used_no_quota', { used: formatBytes(session.used) })}
</p>
<div class="bar" role="progressbar" aria-label={t('storage.espace_utilise')} aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(pct(session.used, total))}>
	<div style="width:{Math.max(pct(session.used, total), session.used > 0 ? 1 : 0)}%"></div>
</div>

<dl>
	<dt>{t('storage.utilise')}</dt><dd>{formatBytes(session.used)}</dd>
	<dt>{session.quota !== null ? t('storage.disponible_dans_votre_quota') : t('storage.libre_sur_le_disque_du')}</dt><dd>{free === null ? '—' : formatBytes(free)}</dd>
	{#if session.disk}<dt>{t('storage.capacite_du_disque_du_serveur')}</dt><dd>{formatBytes(session.disk.total_bytes)}</dd>{/if}
</dl>

<p class="note">
	{t('storage.le_calcul_inclut_les_fichiers')}
</p>

<style>
	h2 {
		font-size: 24px;
		margin-bottom: 8px;
	}
	.lead {
		margin: 0 0 16px;
		font-size: 16px;
		color: var(--text-2);
	}
	.bar {
		max-width: 560px;
		height: 8px;
		border-radius: 4px;
		background: var(--bar-bg);
		overflow: hidden;
	}
	.bar div {
		height: 100%;
		background: var(--accent);
	}
	dl {
		max-width: 560px;
		margin: 24px 0;
		display: grid;
		grid-template-columns: 1fr auto;
		gap: 8px 24px;
	}
	dt {
		color: var(--text-2);
	}
	dd {
		margin: 0;
		font-weight: 500;
	}
	.note {
		max-width: 560px;
		color: var(--text-2);
		font-size: 13px;
	}
</style>
