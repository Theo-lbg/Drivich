<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import { onMount } from 'svelte';
	import { formatBytes, pct } from '$lib/format';
	import { refreshStorage, session } from '$lib/state/session.svelte';

	onMount(refreshStorage);

	const disk = $derived(session.disk);
	const diskUsed = $derived(disk ? Math.max(0, disk.total_bytes - disk.free_bytes) : null);
	/** What is on the disk but is not Drivich (system, other apps, other users' files are Drivich's own). */
	const others = $derived(disk && diskUsed !== null ? Math.max(0, diskUsed - session.used) : null);
	const total = $derived(disk?.total_bytes ?? session.quota ?? null);
	const used = $derived(diskUsed ?? session.used);
</script>

<h2>{t('sidebar.espace_de_stockage')}</h2>
<p class="lead">
	{total ? t('storage.used_of_quota', { used: formatBytes(used), quota: formatBytes(total) }) : t('storage.used_no_quota', { used: formatBytes(used) })}
</p>
<div class="bar" role="progressbar" aria-label={t('storage.espace_utilise')} aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(pct(used, total))}>
	<div class="drv" style="width:{Math.max(pct(session.used, total), session.used > 0 ? 1 : 0)}%"></div>
	{#if others !== null}<div class="oth" style="width:{pct(others, total)}%"></div>{/if}
</div>
<ul class="legend">
	<li><i class="drv"></i>{t('storage.drivich')}</li>
	{#if others !== null}<li><i class="oth"></i>{t('storage.autres_donnees')}</li>{/if}
</ul>

<dl>
	<dt>{t('storage.total_utilise')}</dt><dd>{formatBytes(used)}</dd>
	<dt>{t('storage.drivich')}</dt><dd>{formatBytes(session.used)}</dd>
	{#if others !== null}<dt>{t('storage.autres_donnees')}</dt><dd>{formatBytes(others)}</dd>{/if}
	{#if disk}<dt>{t('storage.libre_sur_le_disque_du')}</dt><dd>{formatBytes(disk.free_bytes)}</dd>{/if}
	{#if disk}<dt>{t('storage.capacite_du_disque_du_serveur')}</dt><dd>{formatBytes(disk.total_bytes)}</dd>{/if}
	{#if session.quota !== null}<dt>{t('storage.disponible_dans_votre_quota')}</dt><dd>{formatBytes(Math.max(0, session.quota - session.used))} / {formatBytes(session.quota)}</dd>{/if}
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
	.bar {
		display: flex;
	}
	.bar div {
		height: 100%;
	}
	.drv {
		background: var(--accent);
	}
	.oth {
		background: var(--text-2);
		opacity: 0.45;
	}
	.legend {
		list-style: none;
		margin: 10px 0 0;
		padding: 0;
		display: flex;
		gap: 20px;
		font-size: 13px;
		color: var(--text-2);
	}
	.legend i {
		display: inline-block;
		width: 10px;
		height: 10px;
		border-radius: 50%;
		margin-right: 6px;
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
