<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { browserLocale, i18n, LANGUAGES, setAutoLocale, setLocale, t } from '$lib/i18n/index.svelte';
	import { goto } from '$app/navigation';
	import { api } from '$lib/api';
	import Avatar from '$lib/components/Avatar.svelte';
	import { refreshStorage, session, signOut } from '$lib/state/session.svelte';
	import { errorToast, toast } from '$lib/state/toast.svelte';
	import { setTheme, ui, type Theme } from '$lib/state/ui.svelte';
	import Icon from '$lib/components/Icon.svelte';

	const THEMES: { id: Theme; label: string; hint: string; icon: string }[] = $derived([
		{ id: 'system', label: t('settings.automatique'), hint: t('settings.suit_les_reglages_de_votre'), icon: 'contrast' },
		{ id: 'light', label: t('settings.clair'), hint: t('settings.toujours_le_theme_clair'), icon: 'light_mode' },
		{ id: 'dark', label: t('settings.sombre'), hint: t('settings.toujours_le_theme_sombre'), icon: 'dark_mode' }
	]);

	let name = $state(session.user?.name ?? '');
	let immich = $state(session.user?.immich_url ?? '');
	let savingImmich = $state(false);
	let avatarInput: HTMLInputElement | undefined = $state();
	let avatarBusy = $state(false);
	const MAX_AVATAR = 5 * 1024 * 1024;

	async function onAvatar(e: Event) {
		const input = e.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		input.value = '';
		if (!file) return;
		if (!file.type.startsWith('image/')) return toast(t('settings.choisissez_une_image_jpeg_png'), { kind: 'error' });
		if (file.size > MAX_AVATAR) return toast(t('settings.image_trop_volumineuse_5_mo'), { kind: 'error' });
		avatarBusy = true;
		try {
			session.user = (await api.uploadAvatar(file)).user;
			toast(t('settings.photo_de_profil_mise_a'));
		} catch (err) {
			errorToast(err);
		} finally {
			avatarBusy = false;
		}
	}

	async function removeAvatar() {
		avatarBusy = true;
		try {
			session.user = (await api.removeAvatar()).user;
			toast(t('settings.photo_de_profil_supprimee'));
		} catch (err) {
			errorToast(err);
		} finally {
			avatarBusy = false;
		}
	}
	let current = $state('');
	let next = $state('');
	let again = $state('');
	let busy = $state(false);

	/** `auto` follows the browser; any other value is a language code. */
	async function changeLanguage(value: string) {
		try {
			const code = value === 'auto' ? null : value;
			const { user } = await api.updateMe({ locale: code });
			session.user = user;
			if (code === null) await setAutoLocale();
			else await setLocale(code);
			toast(t('settings.language_saved'));
		} catch (err) {
			errorToast(err);
		}
	}

	async function saveImmich(e: SubmitEvent) {
		e.preventDefault();
		savingImmich = true;
		try {
			const { user } = await api.updateMe({ immich_url: immich.trim() || null });
			session.user = user;
			await refreshStorage(); // refreshes the effective address (personal or server default)
			immich = user.immich_url ?? '';
			toast(user.immich_url ? t('settings.adresse_dimmich_enregistree') : session.immichDefault ? t('settings.adresse_personnelle_retiree_celle_du') : t('settings.adresse_dimmich_retiree'));
		} catch (err) {
			errorToast(err);
		} finally {
			savingImmich = false;
		}
	}

	async function saveName(e: SubmitEvent) {
		e.preventDefault();
		try {
			session.user = (await api.updateMe({ name })).user;
			toast(t('settings.profil_mis_a_jour'));
		} catch (err) {
			errorToast(err);
		}
	}

	async function savePassword(e: SubmitEvent) {
		e.preventDefault();
		if (next !== again) return toast(t('settings.les_mots_de_passe_ne'), { kind: 'error' });
		busy = true;
		try {
			await api.changePassword(current, next);
			current = next = again = '';
			toast(t('settings.mot_de_passe_modifie_les'));
		} catch (err) {
			errorToast(err);
		} finally {
			busy = false;
		}
	}

	async function logout() {
		await signOut();
		goto('/login');
	}
</script>

{#if session.user}
	<div class="who">
		<button class="avatar-btn" onclick={() => avatarInput?.click()} disabled={avatarBusy} title={t('settings.changer_la_photo_de_profil')} aria-label={t('settings.changer_la_photo_de_profil')}>
			<Avatar name={session.user.name} size={96} userId={session.user.id} version={session.user.avatar_version} />
			<span class="cam"><Icon name="photo_camera" size={20} /></span>
		</button>
		<div>
			<h2>{session.user.name}</h2>
			<p>{session.user.email}{session.user.is_admin ? ' · ' + t('settings.administrator') : ''}</p>
			<div class="avatar-actions">
				<button class="btn outlined sm" onclick={() => avatarInput?.click()} disabled={avatarBusy}><Icon name="photo_camera" size={18} />{session.user.avatar_version ? t('settings.changer_la_photo') : t('settings.ajouter_une_photo')}</button>
				{#if session.user.avatar_version}<button class="btn danger sm" onclick={removeAvatar} disabled={avatarBusy}>{t('settings.supprimer')}</button>{/if}
			</div>
		</div>
		<input bind:this={avatarInput} type="file" accept="image/jpeg,image/png,image/gif,image/webp" hidden onchange={onAvatar} />
	</div>

	<section class="appearance" aria-labelledby="language-h">
		<h3 id="language-h">{t('settings.language')}</h3>
		<p class="help">{t('settings.language_hint')}</p>
		<select class="field langsel" aria-label={t('settings.language')} value={session.user.locale ?? 'auto'} onchange={(e) => changeLanguage(e.currentTarget.value)}>
			<option value="auto">{t('settings.language_auto')} — {LANGUAGES.find((l) => l.code === browserLocale())?.name}</option>
			{#each LANGUAGES as l}<option value={l.code}>{l.name}</option>{/each}
		</select>
	</section>

	<section class="appearance" aria-labelledby="appearance-h">
		<h3 id="appearance-h">{t('settings.apparence')}</h3>
		<div class="themes" role="radiogroup" aria-label={t('settings.theme')}>
			{#each THEMES as th}
				<button class="theme" class:on={ui.theme === th.id} role="radio" aria-checked={ui.theme === th.id} onclick={() => setTheme(th.id)}>
					<span class="ti"><Icon name={th.icon} size={24} /></span>
					<span class="tl">{th.label}</span>
					<span class="th">{th.hint}</span>
				</button>
			{/each}
		</div>
	</section>

	<form id="immich" class="immich" onsubmit={saveImmich}>
		<h3>Immich</h3>
		<p class="help">{t('settings.ajoutez_ladresse_de_votre_serveur')} <b>Immich</b> {t('settings.dans_le_menu_des_applications')}</p>
		<label class="field-label" for="immich-url">{t('settings.adresse_dimmich')}</label>
		<input id="immich-url" class="field" type="url" inputmode="url" bind:value={immich} placeholder={session.immichDefault ?? 'https://photos.mon-serveur.fr'} autocomplete="off" spellcheck="false" />
		{#if !immich.trim() && session.immichDefault}<p class="help">{t('settings.adresse_definie_par_ladministrateur')} <code>{session.immichDefault}</code> {t('settings.utilisee_tant_que_vous_nen')}</p>{/if}
		<div class="act">
			{#if session.immichUrl}<a class="btn" href={session.immichUrl} target="_blank" rel="noopener noreferrer"><Icon name="open_in_new" size={18} />{t('app_rail.ouvrir_immich')}</a>{/if}
			<button class="btn filled" disabled={savingImmich || immich.trim() === (session.user.immich_url ?? '')}>{t('settings.enregistrer')}</button>
		</div>
	</form>

	<form onsubmit={saveName}>
		<h3>{t('settings.profil')}</h3>
		<label class="field-label" for="nm">{t('settings.nom_affiche')}</label>
		<input id="nm" class="field" bind:value={name} maxlength="80" required />
		<div class="act"><button class="btn filled" disabled={name.trim() === session.user.name}>{t('settings.enregistrer')}</button></div>
	</form>

	<form onsubmit={savePassword}>
		<h3>{t('login.mot_de_passe')}</h3>
		<label class="field-label" for="cp">{t('settings.mot_de_passe_actuel')}</label>
		<input id="cp" class="field" type="password" bind:value={current} autocomplete="current-password" required />
		<label class="field-label" for="np">{t('settings.nouveau_mot_de_passe_10')}</label>
		<input id="np" class="field" type="password" bind:value={next} autocomplete="new-password" minlength="10" required />
		<label class="field-label" for="ap">{t('settings.confirmer_le_nouveau_mot_de')}</label>
		<input id="ap" class="field" type="password" bind:value={again} autocomplete="new-password" minlength="10" required />
		<div class="act"><button class="btn filled" disabled={busy}>{t('settings.changer_le_mot_de_passe')}</button></div>
	</form>

	<div class="out"><button class="btn outlined" onclick={logout}>{t('settings.se_deconnecter')}</button></div>
{/if}

<style>
	.who {
		display: flex;
		align-items: center;
		gap: 20px;
		margin-bottom: 24px;
	}
	.avatar-btn {
		position: relative;
		border: 0;
		padding: 0;
		background: transparent;
		border-radius: 50%;
		line-height: 0;
	}
	.avatar-btn:hover {
		box-shadow: 0 0 0 4px var(--hover-bg);
	}
	.cam {
		position: absolute;
		right: -2px;
		bottom: -2px;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		border-radius: 50%;
		background: var(--accent);
		color: var(--on-accent);
		border: 2px solid var(--surface);
	}
	.avatar-actions {
		display: flex;
		gap: 8px;
		margin-top: 12px;
		flex-wrap: wrap;
	}
	.sm {
		height: 36px;
		padding: 0 16px;
	}
	h2 {
		font-size: 24px;
	}
	.who p {
		margin: 2px 0 0;
		color: var(--text-2);
	}
	.appearance {
		max-width: 560px;
		margin-bottom: 32px;
	}
	.themes {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 12px;
	}
	.theme {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 4px;
		padding: 16px;
		border: 1px solid var(--line);
		border-radius: 12px;
		background: var(--card);
		text-align: left;
	}
	.theme:hover {
		background: var(--card-hover);
	}
	.theme.on {
		border-color: var(--accent);
		outline: 1px solid var(--accent);
		background: var(--accent-soft);
		color: var(--accent-soft-text);
	}
	.ti {
		display: inline-flex;
		margin-bottom: 4px;
	}
	.tl {
		font-weight: 500;
	}
	.th {
		font-size: 12px;
		opacity: 0.8;
	}
	@media (max-width: 560px) {
		.themes {
			grid-template-columns: 1fr;
		}
	}
	form {
		max-width: 480px;
		margin-bottom: 32px;
	}
	h3 {
		margin-bottom: 12px;
		font-size: 18px;
	}
	.field {
		margin-bottom: 14px;
	}
	.act {
		display: flex;
		justify-content: flex-end;
	}
	.out {
		max-width: 480px;
	}
	.immich {
		max-width: 560px;
	}
	.langsel {
		max-width: 360px;
		height: 44px;
		font-size: 15px;
	}
	.help {
		margin: 0 0 12px;
		color: var(--text-2);
		font-size: 13px;
		line-height: 18px;
	}
	.act {
		gap: 8px;
	}
	code {
		font-family: ui-monospace, Menlo, monospace;
		font-size: 12px;
		background: var(--surface-2);
		padding: 1px 6px;
		border-radius: 4px;
		overflow-wrap: anywhere;
	}
</style>
