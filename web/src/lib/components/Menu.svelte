<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts" module>
	export interface MenuItem {
		label?: string;
		icon?: string;
		run?: () => void;
		danger?: boolean;
		disabled?: boolean;
		divider?: boolean;
		hint?: string;
		checked?: boolean;
		/** Colour of the icon (any CSS colour), e.g. a file-type variable. */
		color?: string;
		submenu?: MenuItem[];
	}
</script>

<script lang="ts">
	import { onMount, tick } from 'svelte';
	import Icon from './Icon.svelte';
		import Menu from './Menu.svelte';

	let {
		x,
		y,
		items,
		onclose,
		minWidth = 220,
		nested = false,
		anchorLeft = 0,
		onback
	}: { x: number; y: number; items: MenuItem[]; onclose: () => void; minWidth?: number; nested?: boolean; anchorLeft?: number; onback?: () => void } = $props();

	let el: HTMLDivElement | undefined = $state();
	let pos = $state({ left: 0, top: 0 });
	let active = $state(-1);
	let sub = $state<{ index: number; x: number; y: number; left: number } | null>(null);

	$effect(() => {
		pos = { left: x, top: y };
	});

	onMount(() => {
		tick().then(() => {
			if (!el) return;
			const r = el.getBoundingClientRect();
			const vw = window.innerWidth;
			const vh = window.innerHeight;
			pos = {
				left: Math.max(8, Math.min(x, vw - r.width - 8)),
				top: Math.max(8, y + r.height > vh - 8 ? Math.max(8, vh - r.height - 8) : y)
			};
			el.focus();
		});
		const down = (e: MouseEvent) => {
			if (el && !el.contains(e.target as Node)) onclose();
		};
		const key = (e: KeyboardEvent) => {
			if (e.key === 'Escape') {
				e.preventDefault();
				onclose();
			}
		};
		const blur = () => onclose();
		window.addEventListener('mousedown', down, true);
		window.addEventListener('keydown', key, true);
		window.addEventListener('blur', blur);
		window.addEventListener('resize', blur);
		return () => {
			window.removeEventListener('mousedown', down, true);
			window.removeEventListener('keydown', key, true);
			window.removeEventListener('blur', blur);
			window.removeEventListener('resize', blur);
		};
	});

	const enabled = $derived(items.map((it, i) => (it.divider || it.disabled ? -1 : i)).filter((i) => i >= 0));

	function openSub(i: number, target: HTMLElement) {
		const r = target.getBoundingClientRect();
		const m = el?.getBoundingClientRect();
		sub = { index: i, x: r.right - 4, y: r.top - 8, left: m ? m.left + 4 : r.left };
	}

	function choose(it: MenuItem, i = -1, target?: HTMLElement) {
		if (it.disabled || it.divider) return;
		if (it.submenu) {
			if (target) openSub(i, target);
			return;
		}
		onclose();
		it.run?.();
	}

	function onkeydown(e: KeyboardEvent) {
		if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
			e.preventDefault();
			const cur = enabled.indexOf(active);
			const next = e.key === 'ArrowDown' ? (cur + 1) % enabled.length : (cur - 1 + enabled.length) % enabled.length;
			active = enabled[next];
			sub = null;
		} else if (e.key === 'ArrowRight' && active >= 0 && items[active].submenu) {
			e.preventDefault();
			const target = el?.querySelector<HTMLElement>(`:scope > [data-i="${active}"]`);
			if (target) openSub(active, target);
		} else if (e.key === 'ArrowLeft' && nested) {
			e.preventDefault();
			onback?.();
		} else if (e.key === 'Enter' && active >= 0) {
			e.preventDefault();
			const target = el?.querySelector<HTMLElement>(`:scope > [data-i="${active}"]`);
			choose(items[active], active, target ?? undefined);
		}
	}
</script>

<div bind:this={el} class="menu" role="menu" tabindex="-1" style="left:{pos.left}px; top:{pos.top}px; min-width:{minWidth}px" {onkeydown}>
	{#each items as it, i}
		{#if it.divider}
			<div class="divider" role="separator"></div>
		{:else}
			<button
				class="item"
				class:danger={it.danger}
				class:active={active === i}
				role="menuitem"
				disabled={it.disabled}
				class:open={sub?.index === i}
				data-i={i}
				aria-haspopup={it.submenu ? 'menu' : undefined}
				aria-expanded={it.submenu ? sub?.index === i : undefined}
				onclick={(e) => choose(it, i, e.currentTarget)}
				onmouseenter={(e) => {
					active = it.disabled ? -1 : i;
					if (it.submenu && !it.disabled) openSub(i, e.currentTarget);
					else sub = null;
				}}
			>
				<span class="ico" style:color={it.color}>{#if it.checked}<Icon name="check" size={20} />{:else if it.icon}<Icon name={it.icon} size={20} />{/if}</span>
				<span class="label">{it.label}</span>
				{#if it.hint}<span class="hint">{it.hint}</span>{/if}
				{#if it.submenu}<span class="chev"><Icon name="arrow_right" size={20} /></span>{/if}
			</button>
		{/if}
	{/each}
	{#if sub}
		{@const parent = items[sub.index]}
		{#if parent?.submenu}
			<Menu
				x={sub.x}
				y={sub.y}
				anchorLeft={sub.left}
				items={parent.submenu}
				nested
				{onclose}
				onback={() => {
					sub = null;
					el?.focus();
				}}
				{minWidth}
			/>
		{/if}
	{/if}
</div>

<style>
	.menu {
		position: fixed;
		z-index: 1000;
		padding: 8px 0;
		background: var(--elevated);
		border-radius: 8px;
		box-shadow: var(--shadow-menu);
		outline: none;
		max-height: calc(100vh - 16px);
		overflow: auto;
	}
	.item {
		display: flex;
		align-items: center;
		gap: 12px;
		width: 100%;
		height: 36px;
		padding: 0 16px 0 12px;
		border: 0;
		background: transparent;
		text-align: left;
		color: var(--text);
		font-size: 14px;
	}
	.item.active,
	.item.open,
	.item:hover:not(:disabled) {
		background: var(--hover-bg);
	}
	.item:disabled {
		color: var(--text-3);
		cursor: default;
	}
	.item.danger {
		color: var(--danger);
	}
	.ico {
		width: 20px;
		height: 20px;
		display: inline-flex;
		color: var(--text-2);
	}
	.danger .ico {
		color: var(--danger);
	}
	.label {
		flex: 1;
	}
	.chev {
		display: inline-flex;
		margin-right: -8px;
		color: var(--text-2);
	}
	.hint {
		color: var(--text-3);
		font-size: 12px;
	}
	.divider {
		height: 1px;
		margin: 8px 0;
		background: var(--line);
	}
</style>
