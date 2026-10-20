<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import { session } from '$lib/state/session.svelte';
	import Icon from './Icon.svelte';
	import ImmichIcon from './ImmichIcon.svelte';
</script>

<!-- Slim rail on the right, like Drive's side panel of apps: quick access to Immich. -->
<aside class="rail" aria-label={t('app_rail.applications')}>
	{#if session.immichUrl}
		<a class="btn-rail" href={session.immichUrl} target="_blank" rel="noopener noreferrer" title={t('app_rail.ouvrir_immich')} aria-label={t('app_rail.ouvrir_immich_nouvel_onglet')}>
			<ImmichIcon url={session.immichUrl} size={28} />
		</a>
		<span class="sep"></span>
	{/if}
	<a class="btn-rail" href="/settings#immich" title={session.immichUrl ? t('app_rail.modifier_ladresse_dimmich') : t('app_rail.ajouter_immich')} aria-label={session.immichUrl ? t('app_rail.modifier_ladresse_dimmich') : t('app_rail.ajouter_immich')}>
		<Icon name="add" size={24} />
	</a>
</aside>

<style>
	.rail {
		display: none;
		flex-direction: column;
		align-items: center;
		gap: 8px;
		width: 48px;
		flex: none;
		padding: 8px 0;
	}
	@media (min-width: 1100px) {
		.rail {
			display: flex;
		}
	}
	.btn-rail {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 40px;
		height: 40px;
		border-radius: 50%;
		color: var(--accent);
		text-decoration: none;
	}
	.btn-rail:hover {
		background: var(--hover-bg);
		text-decoration: none;
	}
	.sep {
		width: 24px;
		height: 1px;
		background: var(--line);
		margin: 4px 0;
	}
</style>
