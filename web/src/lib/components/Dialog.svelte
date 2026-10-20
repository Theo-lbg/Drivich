<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { onMount } from 'svelte';
	import type { Snippet } from 'svelte';

	let {
		title,
		onclose,
		children,
		actions,
		width = 480
	}: { title: string; onclose: () => void; children: Snippet; actions?: Snippet; width?: number } = $props();

	let el: HTMLDivElement | undefined = $state();

	onMount(() => {
		const prev = document.activeElement as HTMLElement | null;
		el?.querySelector<HTMLElement>('input, button.btn, select, textarea')?.focus();
		return () => prev?.focus?.();
	});

	function key(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.stopPropagation();
			onclose();
		}
		if (e.key === 'Tab' && el) {
			const f = [...el.querySelectorAll<HTMLElement>('input, button, select, textarea, a[href]')].filter((n) => !(n as HTMLButtonElement).disabled);
			if (!f.length) return;
			const first = f[0];
			const last = f[f.length - 1];
			if (e.shiftKey && document.activeElement === first) {
				e.preventDefault();
				last.focus();
			} else if (!e.shiftKey && document.activeElement === last) {
				e.preventDefault();
				first.focus();
			}
		}
	}
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" onmousedown={(e) => e.target === e.currentTarget && onclose()} onkeydown={key}>
	<div bind:this={el} class="dialog" role="dialog" aria-modal="true" aria-label={title} style="width:min({width}px, calc(100vw - 32px))">
		<h2>{title}</h2>
		<div class="body">{@render children()}</div>
		{#if actions}<div class="actions">{@render actions()}</div>{/if}
	</div>
</div>

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		z-index: 900;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--scrim);
	}
	.dialog {
		background: var(--elevated);
		border-radius: 28px;
		padding: 24px;
		box-shadow: var(--shadow-2);
		max-height: calc(100vh - 32px);
		display: flex;
		flex-direction: column;
	}
	h2 {
		font-size: 24px;
		line-height: 32px;
		margin-bottom: 16px;
		color: var(--text);
	}
	.body {
		overflow: auto;
		min-height: 0;
		padding: 2px;
	}
	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
		margin-top: 24px;
	}
</style>
