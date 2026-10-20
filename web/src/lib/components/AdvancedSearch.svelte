<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import { goto } from '$app/navigation';

	let { initial = '', onclose }: { initial?: string; onclose: () => void } = $props();

	let name = $state('');
	// svelte-ignore state_referenced_locally
	let words = $state(initial);
	let type = $state('');
	let owner = $state('');
	let location = $state('');
	let date = $state('');

	function go(e: SubmitEvent) {
		e.preventDefault();
		const p = new URLSearchParams();
		if (name.trim()) p.set('name', name.trim());
		if (words.trim()) p.set('words', words.trim());
		if (type) p.set('type', type);
		if (owner) p.set('owner', owner);
		if (location) p.set('location', location);
		if (date) p.set('date', date);
		onclose();
		goto(`/search?${p}`);
	}

	function reset() {
		name = words = type = owner = location = date = '';
	}
</script>

<form class="adv" onsubmit={go} aria-label={t('advanced_search.recherche_avancee')}>
	<div class="r"><label for="adv-type">{t('advanced_search.type')}</label>
		<select id="adv-type" class="field" bind:value={type}>
			<option value="">{t('advanced_search.nimporte_lequel')}</option>
			<option value="folder">{t('advanced_search.dossiers')}</option>
			<option value="image">{t('advanced_search.images')}</option>
			<option value="video">{t('advanced_search.videos')}</option>
			<option value="audio">{t('advanced_search.audio')}</option>
			<option value="pdf">{t('advanced_search.pdf')}</option>
			<option value="document">{t('advanced_search.documents')}</option>
			<option value="text">{t('advanced_search.textes_et_code')}</option>
			<option value="archive">{t('advanced_search.archives')}</option>
		</select>
	</div>
	<div class="r"><label for="adv-owner">{t('advanced_search.contacts')}</label>
		<select id="adv-owner" class="field" bind:value={owner}>
			<option value="">{t('advanced_search.nimporte_lequel')}</option>
			<option value="me">{t('advanced_search.moi')}</option>
			<option value="others">{t('advanced_search.autres_personnes')}</option>
		</select>
	</div>
	<div class="r"><label for="adv-words">{t('advanced_search.contient_les_mots')}</label><input id="adv-words" class="field" bind:value={words} placeholder={t('advanced_search.mots_presents_dans_le_contenu')} /></div>
	<div class="r"><label for="adv-name">{t('advanced_search.nom_de_lelement')}</label><input id="adv-name" class="field" bind:value={name} placeholder={t('advanced_search.saisissez_un_terme_du_nom')} /></div>
	<div class="r"><label for="adv-loc">{t('advanced_search.emplacement')}</label>
		<select id="adv-loc" class="field" bind:value={location}>
			<option value="">{t('advanced_search.partout')}</option>
			<option value="drive">{t('advanced_search.mon_drive')}</option>
			<option value="shared">{t('advanced_search.partages_avec_moi')}</option>
			<option value="starred">{t('advanced_search.favoris')}</option>
			<option value="trash">{t('advanced_search.corbeille')}</option>
		</select>
	</div>
	<div class="r"><label for="adv-date">{t('advanced_search.date_de_modification')}</label>
		<select id="adv-date" class="field" bind:value={date}>
			<option value="">{t('advanced_search.nimporte_quand')}</option>
			<option value="today">{t('advanced_search.aujourdhui')}</option>
			<option value="7d">{t('advanced_search.7_derniers_jours')}</option>
			<option value="30d">{t('advanced_search.30_derniers_jours')}</option>
			<option value="year">{t('advanced_search.cette_annee')}</option>
		</select>
	</div>
	<div class="act">
		<button type="button" class="btn" onclick={reset}>{t('advanced_search.reinitialiser')}</button>
		<button type="submit" class="btn filled">{t('advanced_search.rechercher')}</button>
	</div>
</form>

<style>
	.adv {
		position: absolute;
		left: 0;
		right: 0;
		top: 52px;
		z-index: 60;
		padding: 20px 24px 16px;
		background: var(--elevated);
		border-radius: 16px;
		box-shadow: var(--shadow-2);
	}
	.r {
		display: grid;
		grid-template-columns: 160px 1fr;
		align-items: center;
		gap: 12px;
		margin-bottom: 12px;
		color: var(--text-2);
	}
	.field {
		height: 40px;
		font-size: 14px;
	}
	.act {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
		margin-top: 16px;
	}
	@media (max-width: 600px) {
		.r {
			grid-template-columns: 1fr;
			gap: 4px;
		}
	}
</style>
