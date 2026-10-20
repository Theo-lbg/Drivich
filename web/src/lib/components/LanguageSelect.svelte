<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { i18n, LANGUAGES, setLocale, t } from '$lib/i18n/index.svelte';

	/** Compact language picker for pages without an account (sign-in, setup). */
	let { class: cls = '' }: { class?: string } = $props();
</script>

<label class="lang {cls}">
	<span class="sr-only">{t('settings.language')}</span>
	<select aria-label={t('settings.language')} value={i18n.locale} onchange={(e) => setLocale(e.currentTarget.value)}>
		{#each LANGUAGES as l}<option value={l.code}>{l.name}</option>{/each}
	</select>
</label>

<style>
	.lang select {
		height: 36px;
		padding: 0 12px;
		border: 1px solid var(--line-strong);
		border-radius: 18px;
		background: transparent;
		color: var(--text-2);
		font-size: 13px;
	}
</style>
