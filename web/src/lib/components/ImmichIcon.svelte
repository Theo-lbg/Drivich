<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import Icon from './Icon.svelte';

	/**
	 * Shows the icon served by the user's own Immich instance (their browser
	 * fetches it from their server, nothing is bundled or redistributed by Drivich),
	 * and falls back to a generic photo-library icon if it cannot be loaded
	 * (blocked mixed content, unreachable server, different Immich version...).
	 */
	let { url, size = 24 }: { url: string; size?: number } = $props();

	const CANDIDATES = ['/favicon-96.png', '/favicon.png', '/apple-icon-180.png'];
	let attempt = $state(0);

	// A new address means a fresh try.
	$effect(() => {
		url;
		attempt = 0;
	});
</script>

{#if attempt < CANDIDATES.length}
	<img
		src={url + CANDIDATES[attempt]}
		alt=""
		width={size}
		height={size}
		referrerpolicy="no-referrer"
		decoding="async"
		onerror={() => attempt++}
	/>
{:else}
	<Icon name="photo_library" {size} />
{/if}

<style>
	img {
		display: block;
		object-fit: contain;
		border-radius: 6px;
	}
</style>
