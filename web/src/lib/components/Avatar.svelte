<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { urls } from '$lib/api';

	/**
	 * A user's picture, or a coloured initial when they have none (or it fails to load).
	 * `version` comes from the API and changes with every upload (cache busting).
	 */
	let { name, size = 32, userId = null, version = 0 }: { name: string; size?: number; userId?: string | null; version?: number } = $props();

	const PALETTE = ['#6b4c3b', '#1a73e8', '#188038', '#a142f4', '#e37400', '#d93025', '#007b83', '#5f6368'];
	const color = $derived(PALETTE[[...name].reduce((a, c) => a + c.charCodeAt(0), 0) % PALETTE.length]);
	const initial = $derived((name.trim()[0] ?? '?').toUpperCase());

	let failed = $state(false);
	$effect(() => {
		userId;
		version;
		failed = false;
	});
	const src = $derived(userId && version > 0 && !failed ? urls.avatar(userId, version) : null);
</script>

{#if src}
	<img class="avatar" {src} alt="" width={size} height={size} style="width:{size}px;height:{size}px" onerror={() => (failed = true)} />
{:else}
	<span class="avatar" style="width:{size}px;height:{size}px;background:{color};font-size:{Math.round(size * 0.45)}px" aria-hidden="true">{initial}</span>
{/if}

<style>
	.avatar {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		border-radius: 50%;
		color: #fff;
		font-family: var(--font);
		font-weight: 500;
		flex: none;
		user-select: none;
		object-fit: cover;
	}
</style>
