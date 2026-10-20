<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { urls } from '$lib/api';
	import { CLASS_COLOR, CLASS_ICON, classify } from '$lib/format';
	import type { NodeItem } from '$lib/types';
	import Icon from './Icon.svelte';

	let {
		node,
		size = 24,
		cover = false,
		token = null
	}: { node: Pick<NodeItem, 'id' | 'kind' | 'mime' | 'name' | 'thumb_state'>; size?: number; cover?: boolean; token?: string | null } = $props();

	const cls = $derived(classify(node));
	let broken = $state(false);
	const showImg = $derived(cover && node.kind === 'file' && node.thumb_state === 'ready' && !broken);
</script>

{#if showImg}
	<img src={token ? urls.publicThumb(token, node.id) : urls.thumb(node.id)} alt="" loading="lazy" onerror={() => (broken = true)} />
{:else}
	<span class="ico" style="color:{CLASS_COLOR[cls]}">
		<Icon name={CLASS_ICON[cls]} {size} fill={cls === 'folder'} />
	</span>
{/if}

<style>
	img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}
	.ico {
		display: inline-flex;
	}
</style>
