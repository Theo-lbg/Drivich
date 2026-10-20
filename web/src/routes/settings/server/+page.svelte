<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { formatBytes, formatDate, pct } from '$lib/format';
	import { errorToast, toast } from '$lib/state/toast.svelte';
	import type { AdminOverview, BackupItem, JobControls } from '$lib/types';
	import { urls } from '$lib/api';
	import Icon from '$lib/components/Icon.svelte';

	let o = $state<AdminOverview | null>(null);
	let controls = $state<Required<JobControls> | null>(null);
	let backups = $state<{ items: BackupItem[]; enabled: boolean; keep: number } | null>(null);
	let timer: ReturnType<typeof setInterval> | undefined;

	async function load() {
		try {
			[o, controls, backups] = await Promise.all([api.adminOverview(), api.adminJobControls(), api.adminBackups()]);
		} catch (e) {
			errorToast(e);
		}
	}

	onMount(() => {
		load();
		timer = setInterval(load, 5000);
		return () => clearInterval(timer);
	});

	async function saveControls(next: Required<JobControls>) {
		try {
			controls = await api.adminSaveJobControls(next);
		} catch (e) {
			errorToast(e);
			load();
		}
	}

	function togglePause(kind: string) {
		if (!controls) return;
		const paused = controls.paused.includes(kind) ? controls.paused.filter((k) => k !== kind) : [...controls.paused, kind];
		saveControls({ ...controls, paused });
	}

	function setLimit(kind: string, raw: string) {
		if (!controls) return;
		const limits = { ...controls.limits };
		if (raw.trim() === '') delete limits[kind];
		else limits[kind] = Math.min(32, Math.max(0, Math.round(Number(raw))));
		saveControls({ ...controls, limits });
	}

	async function createBackup() {
		try {
			await api.adminCreateBackup();
			toast(t('backups.queued'));
			setTimeout(load, 4000);
		} catch (e) {
			errorToast(e);
		}
	}

	async function retry() {
		try {
			const r = await api.adminRetryJobs();
			toast(t('server.requeued', { count: r.requeued }));
			load();
		} catch (e) {
			errorToast(e);
		}
	}

	const STATUS_LABEL: Record<string, string> = $derived({ queued: t('server.en_attente'), running: t('server.en_cours'), done: t('server.terminees'), failed: t('server.en_echec') });
	const KIND_LABEL: Record<string, string> = $derived({
		thumbnail: t('server.miniatures'),
		checksum: t('server.empreintes_sha_256'),
		index_text: t('server.indexation_du_texte'),
		purge_trash: t('server.purge_de_la_corbeille'),
		cleanup_uploads: t('server.nettoyage_des_televersements'),
		cleanup_sessions: t('server.nettoyage_des_sessions'),
		gc_blobs: t('server.fichiers_orphelins'),
		db_dump: t('server.kind_db_dump'),
		office_preview: t('server.kind_office_preview')
	});

	const grouped = $derived.by(() => {
		const m = new Map<string, Record<string, number>>();
		for (const k of controls?.kinds ?? []) m.set(k, {});
		for (const s of o?.jobs.stats ?? []) {
			const r = m.get(s.kind) ?? {};
			r[s.status] = s.count;
			m.set(s.kind, r);
		}
		return [...m.entries()];
	});
</script>

<h2>{t('server.serveur')}</h2>
{#if o}
	<div class="cards">
		<div class="c"><small>{t('server.version')}</small><b>Drivich {o.version}</b></div>
		<div class="c"><small>{t('server.base_de_donnees')}</small><b class:bad={!o.database.ok}>{o.database.ok ? t('server.connectee') : t('server.injoignable')}</b><span>PostgreSQL {o.database.version}</span></div>
		<div class="c"><small>{t('server.utilisateurs')}</small><b>{o.counts.users}</b></div>
		<div class="c"><small>{t('server.fichiers_dossiers')}</small><b>{o.counts.files} / {o.counts.folders}</b><span>{t('server.stored', { size: formatBytes(o.counts.stored_bytes) })}</span></div>
	</div>

	{#if o.disk}
		<h3>{t('server.disque_du_serveur')}</h3>
		<div class="bar"><div style="width:{pct(o.disk.total_bytes - o.disk.free_bytes, o.disk.total_bytes)}%"></div></div>
		<p class="muted">{t('server.disk_usage', { used: formatBytes(o.disk.total_bytes - o.disk.free_bytes), total: formatBytes(o.disk.total_bytes), free: formatBytes(o.disk.free_bytes) })}</p>
	{/if}

	<h3>{t('server.parametres_du_serveur')}</h3>
	<dl>
		<dt>{t('server.dossier_de_donnees')}</dt><dd><code>{o.data_dir}</code></dd>
		<dt>{t('server.taille_maximale_dun_fichier')}</dt><dd>{o.limits.max_file_bytes ? formatBytes(o.limits.max_file_bytes) : t('server.illimitee')}</dd>
		<dt>{t('server.retention_de_la_corbeille')}</dt><dd>{t('server.days', { count: o.limits.trash_days })}</dd>
		<dt>{t('server.expiration_des_televersements_inacheves')}</dt><dd>{o.limits.upload_ttl_hours} h</dd>
	</dl>
	<p class="note">{t('server.ces_valeurs_se_reglent_par')} <code>.env.example</code>).</p>

	<h3>{t('server.traitements_en_arriere_plan')}</h3>
	{#if grouped.length && controls}
		<p class="note">{t('server.job_controls_hint')}</p>
		<div class="jobs" role="table" aria-label={t('server.taches')}>
			<div class="jr jh" role="row"><span>{t('server.tache')}</span><span>{t('server.en_attente')}</span><span>{t('server.en_cours')}</span><span>{t('server.terminees')}</span><span>{t('server.en_echec')}</span><span>{t('server.concurrency')}</span><span></span></div>
			{#each grouped as [kind, st]}
				{@const paused = controls.paused.includes(kind)}
				<div class="jr" role="row" class:off={paused}>
					<span>{KIND_LABEL[kind] ?? kind}{#if paused} <small class="pill">{t('server.paused')}</small>{/if}</span>
					{#each ['queued', 'running', 'done', 'failed'] as s}<span class:bad={s === 'failed' && st[s]}>{st[s] ?? 0}</span>{/each}
					<span><input class="num" type="number" min="0" max="32" placeholder="∞" title={t('server.limit_hint')} aria-label="{t('server.concurrency')} – {KIND_LABEL[kind] ?? kind}" value={controls.limits[kind] ?? ''} onchange={(e) => setLimit(kind, e.currentTarget.value)} /></span>
					<span><button class="icon-btn small" title={paused ? t('server.resume') : t('server.pause')} aria-label="{paused ? t('server.resume') : t('server.pause')} – {KIND_LABEL[kind] ?? kind}" onclick={() => togglePause(kind)}><Icon name={paused ? 'play_arrow' : 'pause'} size={20} /></button></span>
				</div>
			{/each}
		</div>
	{:else}
		<p class="muted">{t('server.aucune_tache_enregistree_pour_linstant')}</p>
	{/if}

	{#if o.jobs.recent_failures.length}
		<h3>{t('server.echecs_recents')}</h3>
		<ul class="fails">
			{#each o.jobs.recent_failures as f (f.id)}
				<li><b>{KIND_LABEL[f.kind] ?? f.kind}</b> · {formatDate(f.updated_at)} · {t('server.attempts', { count: f.attempts })}<br /><code>{f.last_error ?? t('server.unknown_error')}</code></li>
			{/each}
		</ul>
		<button class="btn outlined" onclick={retry}>{t('server.relancer_les_taches_en_echec')}</button>
	{/if}
	<h3>{t('backups.title')}</h3>
	{#if backups}
		<p class="note">{backups.enabled ? t('backups.intro', { keep: backups.keep }) : t('backups.disabled')}</p>
		{#if backups.items.length}
			<ul class="backups">
				{#each backups.items as b (b.name)}
					<li><span class="bn">{b.name}</span><span class="muted">{formatDate(b.created_at)}</span><span class="muted">{formatBytes(b.size)}</span><a class="btn sm" href={urls.adminBackup(b.name)} download={b.name}><Icon name="download" size={18} />{t('backups.download')}</a></li>
				{/each}
			</ul>
		{:else}
			<p class="muted">{t('backups.none')}</p>
		{/if}
		<p class="note">{t('backups.sensitive')} {t('backups.restore_hint')}</p>
		<button class="btn outlined" onclick={createBackup}>{t('backups.create')}</button>
	{/if}

{:else}
	<p class="muted">{t('file_browser.chargement')}</p>
{/if}

<style>
	h2 {
		font-size: 24px;
		margin-bottom: 16px;
	}
	h3 {
		margin: 28px 0 10px;
		font-size: 18px;
	}
	.cards {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
		gap: 12px;
	}
	.c {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 16px;
		border: 1px solid var(--line);
		border-radius: 12px;
	}
	.c small {
		color: var(--text-2);
	}
	.c b {
		font-size: 20px;
		font-family: var(--font);
		font-weight: 500;
	}
	.c span {
		color: var(--text-2);
		font-size: 12px;
	}
	.bad {
		color: var(--danger);
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
		display: grid;
		grid-template-columns: max-content 1fr;
		gap: 8px 24px;
		margin: 0;
	}
	dt {
		color: var(--text-2);
	}
	dd {
		margin: 0;
	}
	code {
		font-family: ui-monospace, Menlo, monospace;
		font-size: 12px;
		background: var(--surface-2);
		padding: 1px 6px;
		border-radius: 4px;
		overflow-wrap: anywhere;
	}
	.note {
		color: var(--text-2);
		font-size: 13px;
	}
	.jobs {
		border: 1px solid var(--line);
		border-radius: 12px;
		overflow: auto;
		max-width: 860px;
	}
	.jr {
		display: grid;
		grid-template-columns: 1fr repeat(4, 84px) 96px 48px;
		align-items: center;
		min-height: 44px;
		padding: 0 16px;
		border-bottom: 1px solid var(--line);
	}
	.jr:last-child {
		border-bottom: 0;
	}
	.jh {
		background: var(--surface-2);
		color: var(--text-2);
		font-weight: 500;
	}
	.num {
		width: 72px;
		height: 32px;
		padding: 0 8px;
		border: 1px solid var(--line-strong);
		border-radius: 8px;
		background: transparent;
		color: var(--text);
	}
	.off {
		opacity: 0.65;
	}
	.pill {
		margin-left: 6px;
		padding: 2px 8px;
		border-radius: 10px;
		background: var(--accent-soft);
		color: var(--accent-soft-text);
		font-size: 11px;
	}
	.backups {
		list-style: none;
		margin: 0 0 12px;
		padding: 0;
		max-width: 760px;
	}
	.backups li {
		display: grid;
		grid-template-columns: 1fr 140px 90px auto;
		align-items: center;
		gap: 12px;
		min-height: 44px;
		border-bottom: 1px solid var(--line);
	}
	.bn {
		font-family: ui-monospace, Menlo, monospace;
		font-size: 12px;
		overflow-wrap: anywhere;
	}
	.sm {
		height: 32px;
		padding: 0 12px;
		text-decoration: none;
	}
	.fails {
		padding-left: 18px;
		color: var(--text-2);
	}
</style>
