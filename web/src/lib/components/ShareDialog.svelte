<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import { api } from '$lib/api';
	import { copyText } from '$lib/actions';
	import { formatDate } from '$lib/format';
	import { dialogs, refreshAll } from '$lib/state/app.svelte';
	import { errorToast, toast } from '$lib/state/toast.svelte';
	import type { ShareInfo, UserBrief } from '$lib/types';
	import Avatar from './Avatar.svelte';
	import Dialog from './Dialog.svelte';
	import Icon from './Icon.svelte';

	const node = $derived(dialogs.share);

	let info = $state<ShareInfo | null>(null);
	let loading = $state(true);
	let q = $state('');
	let role = $state<'viewer' | 'commenter' | 'editor'>('viewer');
	let found = $state<UserBrief[]>([]);
	let busy = $state(false);
	let expiry = $state('');
	let freshLink = $state<string | null>(null);
	let timer: ReturnType<typeof setTimeout> | undefined;

	$effect(() => {
		const n = node;
		info = null;
		freshLink = null;
		q = '';
		found = [];
		if (!n) return;
		loading = true;
		api.shares(n.id).then((i) => (info = i)).catch(errorToast).finally(() => (loading = false));
	});

	function onInput() {
		clearTimeout(timer);
		const v = q.trim();
		if (v.length < 2) return (found = []);
		timer = setTimeout(async () => {
			try {
				const list = (await api.lookupUsers(v)).items;
				const already = new Set(info?.users.map((u) => u.grantee_id) ?? []);
				found = list.filter((u) => !already.has(u.id));
			} catch {
				found = [];
			}
		}, 150);
	}

	async function reload() {
		if (node) info = await api.shares(node.id);
		refreshAll();
	}

	async function add(email: string) {
		if (!node || busy) return;
		busy = true;
		try {
			await api.addShare(node.id, email, role);
			q = '';
			found = [];
			await reload();
			toast(t('share.shared_with', { email }));
		} catch (e) {
			errorToast(e);
		} finally {
			busy = false;
		}
	}

	async function changeRole(shareId: string, r: string) {
		try {
			await api.updateShare(shareId, r);
			await reload();
		} catch (e) {
			errorToast(e);
		}
	}

	async function removeShare(shareId: string) {
		try {
			await api.removeShare(shareId);
			await reload();
		} catch (e) {
			errorToast(e);
		}
	}

	function expiresAt(): string | null {
		const days = { '1d': 1, '7d': 7, '30d': 30 }[expiry];
		return days ? new Date(Date.now() + days * 86_400_000).toISOString() : null;
	}

	async function createLink() {
		if (!node || busy) return;
		busy = true;
		try {
			const r = await api.createLink(node.id, expiresAt());
			freshLink = `${location.origin}/s/${r.token}`;
			await copyText(freshLink);
			toast(t('share_dialog.lien_cree_et_copie_dans'));
			await reload();
		} catch (e) {
			errorToast(e);
		} finally {
			busy = false;
		}
	}

	async function revoke(id: string) {
		try {
			await api.revokeLink(id);
			freshLink = null;
			await reload();
			toast(t('share_dialog.lien_revoque_il_ne_fonctionne'));
		} catch (e) {
			errorToast(e);
		}
	}

	function onEnter(e: KeyboardEvent) {
		if (e.key !== 'Enter') return;
		e.preventDefault();
		if (found.length) add(found[0].email);
		else if (q.includes('@')) add(q.trim());
	}
</script>

{#if node}
	<Dialog title={t('share.title', { name: node.name })} onclose={() => (dialogs.share = null)} width={560}>
		<div class="add">
			<input class="field" placeholder={t('share_dialog.ajouter_des_personnes_e_mail')} aria-label={t('share_dialog.ajouter_des_personnes')} bind:value={q} oninput={onInput} onkeydown={onEnter} autocomplete="off" />
			<select class="field role" bind:value={role} aria-label={t('share_dialog.role')}>
				<option value="viewer">{t('share_dialog.lecteur')}</option>
				<option value="commenter">{t('share_dialog.commentateur')}</option>
				<option value="editor">{t('share_dialog.editeur')}</option>
			</select>
		</div>
		{#if found.length}
			<ul class="found" role="listbox">
				{#each found as u (u.id)}
					<li><button onclick={() => add(u.email)}><Avatar name={u.name} size={32} userId={u.id} version={u.avatar_version} /><span><b>{u.name}</b><small>{u.email}</small></span></button></li>
				{/each}
			</ul>
		{:else if q.trim().length >= 2}
			<p class="hint">{t('share_dialog.aucun_utilisateur_trouve_saisissez_une')}</p>
		{/if}

		<h3>{t('share_dialog.personnes_ayant_acces')}</h3>
		{#if loading}
			<p class="hint">{t('file_browser.chargement')}</p>
		{:else if info}
			<ul class="people">
				<li>
					<Avatar name={info.owner.name} size={36} userId={info.owner.id} version={info.owner.avatar_version} />
					<span class="who"><b>{t('share.you', { name: info.owner.name })}</b><small>{info.owner.email}</small></span>
					<span class="own">{t('details_panel.proprietaire')}</span>
				</li>
				{#each info.users as s (s.id)}
					<li>
						<Avatar name={s.grantee_name} size={36} userId={s.grantee_id} version={s.grantee_avatar_version} />
						<span class="who"><b>{s.grantee_name}</b><small>{s.grantee_email}</small></span>
						<select class="plain" value={s.role} aria-label={t('share.role_of', { name: s.grantee_name })} onchange={(e) => changeRole(s.id, e.currentTarget.value)}>
							<option value="viewer">{t('share_dialog.lecteur')}</option>
							<option value="commenter">{t('share_dialog.commentateur')}</option>
							<option value="editor">{t('share_dialog.editeur')}</option>
						</select>
						<button class="icon-btn small" title={t('share_dialog.retirer_lacces')} aria-label="Retirer l’accès de {s.grantee_name}" onclick={() => removeShare(s.id)}><Icon name="close" size={20} /></button>
					</li>
				{/each}
			</ul>

			<h3>{t('share_dialog.acces_general')}</h3>
			<div class="general">
				<span class="lock"><Icon name={info.links.length ? 'public' : 'lock'} size={24} /></span>
				<div class="who">
					<b>{info.links.length ? t('share_dialog.toute_personne_disposant_du_lien') : t('share_dialog.restreint')}</b>
					<small>{info.links.length ? t('share_dialog.peut_consulter_et_telecharger_sans') : t('share_dialog.seules_les_personnes_ajoutees_peuvent')}</small>
				</div>
			</div>
			{#each info.links as l (l.id)}
				<div class="link">
					<Icon name="link" size={20} />
					<span class="grow">{l.expires_at ? t('share.link_active_expires', { created: formatDate(l.created_at), expires: formatDate(l.expires_at) }) : t('share.link_active_forever', { created: formatDate(l.created_at) })}</span>
					<button class="btn danger sm" onclick={() => revoke(l.id)}>{t('share_dialog.revoquer')}</button>
				</div>
			{/each}
			{#if freshLink}
				<div class="fresh">
					<input class="field" readonly value={freshLink} aria-label={t('share_dialog.lien_de_partage')} onfocus={(e) => e.currentTarget.select()} />
					<button class="btn outlined" onclick={async () => { await copyText(freshLink!); toast(t('share_dialog.lien_copie')); }}><Icon name="content_copy" size={18} />{t('share_dialog.copier')}</button>
				</div>
				<p class="hint">{t('share_dialog.cette_adresse_nest_affichee_quune')}</p>
			{/if}
			<div class="newlink">
				<select class="plain" bind:value={expiry} aria-label={t('share_dialog.expiration_du_lien')}>
					<option value="">{t('share_dialog.sans_expiration')}</option>
					<option value="1d">{t('share_dialog.expire_dans_1_jour')}</option>
					<option value="7d">{t('share_dialog.expire_dans_7_jours')}</option>
					<option value="30d">{t('share_dialog.expire_dans_30_jours')}</option>
				</select>
				<button class="btn outlined" onclick={createLink} disabled={busy}><Icon name="link" size={18} />{info.links.length ? t('share_dialog.creer_un_autre_lien') : t('share_dialog.creer_un_lien')}</button>
			</div>
		{/if}
		{#snippet actions()}
			<button class="btn filled" onclick={() => (dialogs.share = null)}>{t('dialogs.ok')}</button>
		{/snippet}
	</Dialog>
{/if}

<style>
	.add {
		display: grid;
		grid-template-columns: 1fr 150px;
		gap: 8px;
	}
	.add .field {
		height: 48px;
		font-size: 15px;
	}
	.found {
		list-style: none;
		margin: 4px 0 0;
		padding: 4px 0;
		border: 1px solid var(--line);
		border-radius: 12px;
	}
	.found button {
		display: flex;
		align-items: center;
		gap: 12px;
		width: 100%;
		padding: 8px 16px;
		border: 0;
		background: transparent;
		text-align: left;
	}
	.found button:hover {
		background: var(--surface-2);
	}
	.found span {
		display: flex;
		flex-direction: column;
	}
	small {
		color: var(--text-2);
		font-size: 12px;
	}
	h3 {
		margin: 24px 0 8px;
		font-size: 16px;
		font-weight: 500;
	}
	.people {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.people li,
	.general,
	.link {
		display: flex;
		align-items: center;
		gap: 12px;
		min-height: 52px;
	}
	.who {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}
	.own {
		color: var(--text-2);
		padding-right: 8px;
	}
	.plain {
		border: 0;
		background: transparent;
		padding: 8px;
		border-radius: 8px;
		color: var(--text-2);
	}
	.plain:hover {
		background: var(--hover-bg);
	}
	.lock {
		display: inline-flex;
		width: 36px;
		height: 36px;
		align-items: center;
		justify-content: center;
		border-radius: 50%;
		background: var(--surface-2);
		color: var(--text-2);
	}
	.link {
		margin-left: 48px;
		min-height: 40px;
		color: var(--text-2);
	}
	.grow {
		flex: 1;
		min-width: 0;
	}
	.sm {
		height: 32px;
		padding: 0 12px;
	}
	.fresh {
		display: flex;
		gap: 8px;
		margin: 8px 0 0 48px;
	}
	.fresh .field {
		height: 40px;
		font-size: 13px;
	}
	.newlink {
		display: flex;
		align-items: center;
		gap: 8px;
		margin: 12px 0 0 48px;
		flex-wrap: wrap;
	}
	.hint {
		margin: 8px 0 0 0;
		color: var(--text-2);
		font-size: 13px;
	}
	.fresh + .hint {
		margin-left: 48px;
	}
</style>
