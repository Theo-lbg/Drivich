<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import { onMount } from 'svelte';
	import { session } from '$lib/state/session.svelte';
	import Icon from './Icon.svelte';
	import ImmichIcon from './ImmichIcon.svelte';

	let { onclose }: { onclose: () => void } = $props();

	let el: HTMLDivElement | undefined = $state();

	onMount(() => {
		const down = (e: MouseEvent) => {
			if (el && !el.contains(e.target as Node) && !(e.target as HTMLElement).closest('[data-apps-button]')) onclose();
		};
		const key = (e: KeyboardEvent) => e.key === 'Escape' && onclose();
		window.addEventListener('mousedown', down, true);
		window.addEventListener('keydown', key, true);
		return () => {
			window.removeEventListener('mousedown', down, true);
			window.removeEventListener('keydown', key, true);
		};
	});
</script>

<div bind:this={el} class="apps" role="menu" aria-label={t('apps_menu.applications_drivich')}>
	{#if session.immichUrl}
		<a class="tile" role="menuitem" href={session.immichUrl} target="_blank" rel="noopener noreferrer" onclick={onclose}>
			<span class="ico logo"><ImmichIcon url={session.immichUrl} size={36} /></span>
			<span class="lbl">Immich</span>
		</a>
	{/if}
	<a class="tile" role="menuitem" href="/settings#immich" onclick={onclose}>
		<span class="ico plain"><Icon name={session.immichUrl ? 'settings' : 'add'} size={28} /></span>
		<span class="lbl">{session.immichUrl ? t('apps_menu.modifier_ladresse') : t('app_rail.ajouter_immich')}</span>
	</a>
</div>

<style>
	.apps {
		position: fixed;
		right: 56px;
		top: calc(var(--topbar-h) - 6px);
		z-index: 80;
		display: grid;
		grid-template-columns: repeat(3, 96px);
		gap: 4px;
		padding: 16px;
		background: var(--elevated);
		border-radius: 24px;
		box-shadow: var(--shadow-2);
	}
	.tile {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 8px;
		padding: 12px 4px;
		border-radius: 16px;
		color: var(--text);
		text-align: center;
		text-decoration: none;
	}
	.tile:hover {
		background: var(--hover-bg);
		text-decoration: none;
	}
	.ico {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 52px;
		height: 52px;
		border-radius: 50%;
		background: var(--accent-soft);
		color: var(--accent-soft-text);
	}
	.ico.logo {
		background: var(--surface-2);
	}
	.ico.plain {
		background: var(--surface-2);
		color: var(--text-2);
	}
	.lbl {
		font-size: 13px;
		line-height: 16px;
	}
</style>
