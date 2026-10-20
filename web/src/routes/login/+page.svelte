<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import LanguageSelect from '$lib/components/LanguageSelect.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { api, ApiError } from '$lib/api';
	import { loadSession, session } from '$lib/state/session.svelte';

	let mode = $state<'login' | 'signup'>('login');
	let email = $state('');
	let name = $state('');
	let password = $state('');
	let error = $state('');
	let busy = $state(false);

	async function submit(e: SubmitEvent) {
		e.preventDefault();
		if (busy) return;
		busy = true;
		error = '';
		try {
			if (mode === 'login') await api.login({ email, password });
			else await api.signup({ email, name, password });
			await loadSession();
			const next = page.url.searchParams.get('next');
			goto(next && next.startsWith('/') && !next.startsWith('//') ? next : '/', { replaceState: true });
		} catch (err) {
			error =
				err instanceof ApiError && err.status === 401
					? t('login.adresse_e_mail_ou_mot')
					: err instanceof ApiError && err.status === 429
						? t('login.trop_de_tentatives_reessayez_dans')
						: err instanceof Error
							? err.message
							: t('login.connexion_impossible');
		} finally {
			busy = false;
		}
	}
</script>

<svelte:head><title>{t('login.connexion_drivich')}</title></svelte:head>

<main class="auth">
	<form class="card" onsubmit={submit}>
		<img src="/logo.png" alt="Drivich" width="72" />
		<h1>{mode === 'login' ? t('login.connexion') : t('login.creer_un_compte')}</h1>
		<p class="sub">{mode === 'login' ? t('login.accedez_a_vos_fichiers_sur') : t('login.rejoignez_ce_serveur_drivich')}</p>
		{#if mode === 'signup'}
			<label class="field-label" for="n">{t('dialogs.nom')}</label>
			<input id="n" class="field" bind:value={name} required autocomplete="name" />
		{/if}
		<label class="field-label" for="e">{t('login.adresse_e_mail')}</label>
		<input id="e" class="field" type="email" bind:value={email} required autocomplete="username" />
		<label class="field-label" for="p">{t('login.mot_de_passe')}</label>
		<input id="p" class="field" type="password" bind:value={password} required minlength={mode === 'signup' ? 10 : undefined} autocomplete={mode === 'login' ? 'current-password' : 'new-password'} />
		{#if error}<p class="err" role="alert">{error}</p>{/if}
		<div class="row">
			{#if session.signupAllowed}
				<button type="button" class="btn" onclick={() => (mode = mode === 'login' ? 'signup' : 'login')}>{mode === 'login' ? t('login.creer_un_compte') : t('login.jai_deja_un_compte')}</button>
			{:else}<span></span>{/if}
			<button class="btn filled" disabled={busy}>{mode === 'login' ? t('login.se_connecter') : t('login.creer_le_compte')}</button>
		</div>
	</form>
	<footer>
		<LanguageSelect />
		Drivich {session.version} · AGPL-3.0 · © 2026 Theo-lbg</footer>
</main>

<style>
	.auth {
		height: 100%;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 24px;
		padding: 16px;
		overflow: auto;
	}
	.card {
		width: min(450px, 100%);
		padding: 40px;
		border: 1px solid var(--line);
		border-radius: 28px;
		background: var(--surface);
		display: flex;
		flex-direction: column;
	}
	img {
		align-self: flex-start;
	}
	h1 {
		margin-top: 16px;
		font-size: 24px;
		line-height: 32px;
	}
	.sub {
		margin: 4px 0 24px;
		font-size: 16px;
		color: var(--text-2);
	}
	.field {
		margin-bottom: 16px;
	}
	.row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-top: 24px;
	}
	.err {
		margin: 0;
		color: var(--danger);
	}
	footer {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 12px;
		color: var(--text-3);
		font-size: 12px;
	}
</style>
