<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import { page } from '$app/state';
	import { newMenuItems } from '$lib/newMenu';
	import { nav } from '$lib/state/app.svelte';
	import Icon from './Icon.svelte';
	import Menu from './Menu.svelte';

	let menu = $state<{ x: number; y: number } | null>(null);
	const blocked = $derived(nav.folderId !== null && !nav.canWrite);

	function open(e: MouseEvent) {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		// The menu is pushed back inside the screen; start from above the button.
		menu = { x: r.left - 160, y: r.top - 160 };
	}
</script>

<!-- "New" button of the phone layout, floating above the bottom bar like Drive's "+". -->
{#if !page.url.pathname.startsWith('/settings')}
<button class="fab" onclick={open} disabled={blocked} title={blocked ? t('sidebar.vous_ne_pouvez_pas_modifier') : t('sidebar.nouveau')} aria-label={t('sidebar.nouveau')} aria-haspopup="menu">
	<Icon name="add" size={32} />
</button>
{/if}

{#if menu}<Menu x={menu.x} y={menu.y} minWidth={260} items={newMenuItems()} onclose={() => (menu = null)} />{/if}

<style>
	.fab {
		display: none;
	}
	@media (max-width: 900px) {
		.fab {
			position: fixed;
			z-index: 41;
			right: 16px;
			bottom: calc(var(--bnav-h) + 16px);
			width: 64px;
			height: 64px;
			display: flex;
			align-items: center;
			justify-content: center;
			border: 0;
			border-radius: 20px;
			background: var(--card);
			color: var(--accent);
			box-shadow: var(--shadow-2);
		}
		.fab:disabled {
			opacity: 0.5;
		}
	}
</style>
