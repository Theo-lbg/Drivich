<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import LanguageSelect from '$lib/components/LanguageSelect.svelte';
	import { goto } from '$app/navigation';
	import { api } from '$lib/api';
	import { loadSession, session } from '$lib/state/session.svelte';

	let email = $state('');
	let name = $state('');
	let password = $state('');
	let confirm = $state('');
	let error = $state('');
	let busy = $state(false);

	async function submit(e: SubmitEvent) {
		e.preventDefault();
		if (password !== confirm) {
			error = t('settings.les_mots_de_passe_ne');
			return;
		}
		busy = true;
		error = '';
		try {
			await api.setup({ email, name, password });
			await loadSession();
			goto('/', { replaceState: true });
		} catch (err) {
			error = err instanceof Error ? err.message : t('setup.configuration_impossible');
		} finally {
			busy = false;
		}
	}
</script>

<svelte:head><title>{t('setup.configuration_drivich')}</title></svelte:head>

<main class="auth">
	<form class="card" onsubmit={submit}>
		<img src="/logo.png" alt="Drivich" width="72" />
		<h1>{t('setup.bienvenue_sur_drivich')}</h1>
		<p class="sub">{t('setup.creez_le_compte_administrateur_pour')}</p>
		<label class="field-label" for="n">{t('dialogs.nom')}</label>
		<input id="n" class="field" bind:value={name} required autocomplete="name" />
		<label class="field-label" for="e">{t('login.adresse_e_mail')}</label>
		<input id="e" class="field" type="email" bind:value={email} required autocomplete="username" />
		<label class="field-label" for="p">{t('setup.mot_de_passe_10_caracteres')}</label>
		<input id="p" class="field" type="password" bind:value={password} required minlength="10" autocomplete="new-password" />
		<label class="field-label" for="c">{t('setup.confirmer_le_mot_de_passe')}</label>
		<input id="c" class="field" type="password" bind:value={confirm} required minlength="10" autocomplete="new-password" />
		{#if error}<p class="err" role="alert">{error}</p>{/if}
		<div class="row"><span></span><button class="btn filled" disabled={busy}>{t('setup.terminer_linstallation')}</button></div>
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
