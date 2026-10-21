<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import Avatar from '$lib/components/Avatar.svelte';
	import Dialog from '$lib/components/Dialog.svelte';
	import { formatBytes } from '$lib/format';
	import { session } from '$lib/state/session.svelte';
	import { errorToast, toast } from '$lib/state/toast.svelte';
	import type { AdminUserRow } from '$lib/types';

	let rows = $state<AdminUserRow[]>([]);
	let allowSignup = $state(false);
	let defaultQuotaGb = $state('');
	let immichDefault = $state('');
	let creating = $state(false);
	let resetFor = $state<AdminUserRow | null>(null);
	let nu = $state({ email: '', name: '', password: '', is_admin: false, quotaGb: '' });
	let newPassword = $state('');

	const GB = 1024 ** 3;

	async function load() {
		try {
			rows = (await api.adminUsers()).items;
			const s = await api.adminSettings();
			allowSignup = s.allow_signup;
			defaultQuotaGb = s.default_quota_bytes ? String(+(s.default_quota_bytes / GB).toFixed(2)) : '';
			immichDefault = s.immich_url ?? '';
		} catch (e) {
			errorToast(e);
		}
	}
	onMount(load);

	async function create(e: SubmitEvent) {
		e.preventDefault();
		try {
			await api.adminCreateUser({ email: nu.email, name: nu.name, password: nu.password, is_admin: nu.is_admin, quota_bytes: nu.quotaGb ? Math.round(+nu.quotaGb * GB) : null });
			creating = false;
			nu = { email: '', name: '', password: '', is_admin: false, quotaGb: '' };
			toast(t('users.utilisateur_cree'));
			await load();
		} catch (err) {
			errorToast(err);
		}
	}

	async function patch(r: AdminUserRow, p: { is_admin?: boolean; disabled?: boolean; quota_bytes?: number | null }) {
		try {
			await api.adminUpdateUser(r.user.id, p);
			await load();
		} catch (err) {
			errorToast(err);
			await load();
		}
	}

	async function setQuota(r: AdminUserRow, v: string) {
		await patch(r, { quota_bytes: v.trim() ? Math.round(+v * GB) : null });
	}

	async function resetPw(e: SubmitEvent) {
		e.preventDefault();
		if (!resetFor) return;
		try {
			await api.adminResetPassword(resetFor.user.id, newPassword);
			toast(t('users.password_reset_done', { name: resetFor.user.name }));
			resetFor = null;
			newPassword = '';
		} catch (err) {
			errorToast(err);
		}
	}

	async function saveSettings() {
		try {
			await api.adminSaveSettings({ allow_signup: allowSignup, default_quota_bytes: defaultQuotaGb.trim() ? Math.round(+defaultQuotaGb * GB) : null, immich_url: immichDefault.trim() || null });
			toast(t('users.parametres_enregistres'));
		} catch (err) {
			errorToast(err);
		}
	}
</script>

<div class="top"><h2>{t('server.utilisateurs')}</h2><button class="btn filled" onclick={() => (creating = true)}>{t('users.ajouter_un_utilisateur')}</button></div>

<div class="table" role="table" aria-label={t('server.utilisateurs')}>
	<div class="tr th" role="row"><span>{t('users.utilisateur')}</span><span>{t('storage.utilise')}</span><span>{t('users.quota_go')}</span><span>{t('users.admin')}</span><span>{t('users.actif')}</span><span></span></div>
	{#each rows as r (r.user.id)}
		<div class="tr" role="row" class:off={r.user.disabled}>
			<span class="u"><Avatar name={r.user.name} size={36} userId={r.user.id} version={r.user.avatar_version} /><span><b>{r.user.name}</b>{r.user.id === session.user?.id ? ' (vous)' : ''}<small>{r.user.email}</small></span></span>
			<span data-label={t('storage.utilise')}>{formatBytes(r.used_bytes)}</span>
			<span data-label={t('users.quota_go')}><input class="field q" type="number" min="0" step="0.1" placeholder="∞" value={r.user.quota_bytes ? +(r.user.quota_bytes / GB).toFixed(2) : ''} aria-label="Quota de {r.user.name} en Go" onchange={(e) => setQuota(r, e.currentTarget.value)} /></span>
			<span data-label={t('users.admin')}><input type="checkbox" checked={r.user.is_admin} disabled={r.user.id === session.user?.id} aria-label={t('users.administrateur')} onchange={(e) => patch(r, { is_admin: e.currentTarget.checked })} /></span>
			<span data-label={t('users.actif')}><input type="checkbox" checked={!r.user.disabled} disabled={r.user.id === session.user?.id} aria-label={t('users.compte_actif')} onchange={(e) => patch(r, { disabled: !e.currentTarget.checked })} /></span>
			<span><button class="btn sm" onclick={() => (resetFor = r)}>{t('login.mot_de_passe')}</button></span>
		</div>
	{/each}
</div>

<h3>{t('users.inscription_et_quotas')}</h3>
<div class="opts">
	<label class="chk"><input type="checkbox" bind:checked={allowSignup} /> {t('users.autoriser_les_inscriptions_depuis_la')}</label>
	<label class="dq">{t('users.quota_par_defaut_go_vide')}
		<input class="field" type="number" min="0" step="0.1" bind:value={defaultQuotaGb} placeholder={t('users.illimite')} />
	</label>
	<label class="dq wide">{t('users.adresse_dimmich_par_defaut_proposee')}
		<input class="field" type="url" bind:value={immichDefault} placeholder={t('users.https_photos_mon_serveur_fr')} autocomplete="off" spellcheck="false" />
	</label>
	<button class="btn filled" onclick={saveSettings}>{t('settings.enregistrer')}</button>
</div>

{#if creating}
	<Dialog title={t('users.ajouter_un_utilisateur')} onclose={() => (creating = false)}>
		<form id="nu" onsubmit={create}>
			<label class="field-label" for="nu-n">{t('dialogs.nom')}</label><input id="nu-n" class="field" bind:value={nu.name} required />
			<label class="field-label" for="nu-e">{t('login.adresse_e_mail')}</label><input id="nu-e" class="field" type="email" bind:value={nu.email} required />
			<label class="field-label" for="nu-p">{t('users.mot_de_passe_initial_10')}</label><input id="nu-p" class="field" type="password" minlength="10" bind:value={nu.password} required />
			<label class="field-label" for="nu-q">{t('users.quota_en_go_vide_par')}</label><input id="nu-q" class="field" type="number" min="0" step="0.1" bind:value={nu.quotaGb} />
			<label class="chk"><input type="checkbox" bind:checked={nu.is_admin} /> {t('users.administrateur')}</label>
		</form>
		{#snippet actions()}
			<button class="btn" onclick={() => (creating = false)}>{t('dialogs.annuler')}</button>
			<button class="btn filled" type="submit" form="nu">{t('dialogs.creer')}</button>
		{/snippet}
	</Dialog>
{/if}

{#if resetFor}
	<Dialog title={t('users.new_password_for', { name: resetFor.user.name })} onclose={() => (resetFor = null)} width={440}>
		<form id="rp" onsubmit={resetPw}>
			<input class="field" type="password" minlength="10" bind:value={newPassword} required aria-label={t('users.nouveau_mot_de_passe')} autocomplete="new-password" />
			<p class="note">{t('users.toutes_les_sessions_de_cet')}</p>
		</form>
		{#snippet actions()}
			<button class="btn" onclick={() => (resetFor = null)}>{t('dialogs.annuler')}</button>
			<button class="btn filled" type="submit" form="rp">{t('advanced_search.reinitialiser')}</button>
		{/snippet}
	</Dialog>
{/if}

<style>
	.top {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 16px;
	}
	h2 {
		font-size: 24px;
	}
	h3 {
		margin: 32px 0 12px;
		font-size: 18px;
	}
	.table {
		border: 1px solid var(--line);
		border-radius: 12px;
		overflow: auto;
	}
	.tr {
		display: grid;
		grid-template-columns: minmax(220px, 1fr) 100px 110px 70px 70px 130px;
		align-items: center;
		gap: 12px;
		min-height: 56px;
		padding: 0 16px;
		border-bottom: 1px solid var(--line);
	}
	.tr:last-child {
		border-bottom: 0;
	}
	.th {
		min-height: 44px;
		color: var(--text-2);
		font-weight: 500;
		background: var(--surface-2);
	}
	.off {
		opacity: 0.55;
	}
	.u {
		display: flex;
		align-items: center;
		gap: 12px;
		min-width: 0;
	}
	.u span {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	small {
		color: var(--text-2);
	}
	.q {
		height: 36px;
		font-size: 14px;
		padding: 0 8px;
	}
	.sm {
		height: 32px;
		padding: 0 12px;
	}
	.opts {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-end;
		gap: 24px;
	}
	.chk {
		display: flex;
		align-items: center;
		gap: 8px;
		margin: 8px 0;
	}
	.dq {
		display: flex;
		flex-direction: column;
		gap: 6px;
		color: var(--text-2);
		font-size: 12px;
	}
	.dq.wide .field {
		width: min(360px, 100%);
	}
	.dq .field {
		width: 200px;
		height: 40px;
		font-size: 14px;
	}
	form .field {
		margin-bottom: 12px;
	}
	.note {
		color: var(--text-2);
		margin: 0;
	}
	@media (max-width: 900px) {
		/* Phones: one card per user instead of a table that scrolls sideways. */
		.table {
			border: 0;
			display: flex;
			flex-direction: column;
			gap: 12px;
			overflow: visible;
		}
		.th {
			display: none;
		}
		.tr {
			grid-template-columns: 1fr 1fr;
			grid-auto-rows: auto;
			align-items: end;
			gap: 12px 16px;
			padding: 16px;
			border: 1px solid var(--line);
			border-radius: 16px;
		}
		.tr:last-child {
			border-bottom: 1px solid var(--line);
		}
		.u {
			grid-column: 1 / -1;
		}
		[data-label]::before {
			content: attr(data-label);
			display: block;
			margin-bottom: 4px;
			color: var(--text-2);
			font-size: 12px;
		}
		.q {
			width: 100%;
		}
		.tr > span:last-child {
			grid-column: 1 / -1;
		}
		.tr > span:last-child .btn {
			width: 100%;
		}
		.top {
			flex-wrap: wrap;
			gap: 12px;
		}
	}
</style>
