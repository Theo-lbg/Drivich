<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { ICONS } from '$lib/icons';

	let { name, size = 24, fill = false, class: cls = '' }: { name: string; size?: number; fill?: boolean; class?: string } = $props();

	const inner = $derived(ICONS[fill && ICONS[name + '-fill'] ? name + '-fill' : name] ?? '');
</script>

<svg class={cls} width={size} height={size} viewBox="0 -960 960 960" fill="currentColor" aria-hidden="true" focusable="false">
	{@html inner}
</svg>

<style>
	svg {
		flex: none;
		display: block;
	}
</style>
