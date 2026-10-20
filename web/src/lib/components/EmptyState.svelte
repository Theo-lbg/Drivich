<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	let { title, text, art = 'files' }: { title: string; text?: string; art?: 'files' | 'trash' | 'search' | 'star' | 'share' } = $props();
</script>

<div class="empty">
	<!-- Original illustration, drawn with the Drivich palette. -->
	<svg viewBox="0 0 240 160" width="240" height="160" aria-hidden="true">
		<ellipse cx="120" cy="146" rx="84" ry="7" fill="var(--line)" />
		{#if art === 'trash'}
			<path d="M80 52h80l-7 88a8 8 0 0 1-8 7H95a8 8 0 0 1-8-7z" fill="var(--accent-soft)" stroke="var(--accent)" stroke-width="3" stroke-linejoin="round" />
			<path d="M72 52h96M104 52v-9a5 5 0 0 1 5-5h22a5 5 0 0 1 5 5v9" fill="none" stroke="var(--accent)" stroke-width="3" stroke-linecap="round" />
			<path d="M105 74v52M120 74v52M135 74v52" stroke="var(--accent)" stroke-width="3" stroke-linecap="round" />
		{:else if art === 'search'}
			<circle cx="108" cy="72" r="38" fill="var(--accent-soft)" stroke="var(--accent)" stroke-width="4" />
			<path d="M136 100l36 36" stroke="var(--accent)" stroke-width="9" stroke-linecap="round" />
			<path d="M92 72h32M92 86h20" stroke="var(--accent)" stroke-width="3" stroke-linecap="round" opacity=".5" />
		{:else if art === 'star'}
			<path d="M120 28l24 50 55 8-40 38 10 55-49-27-49 27 10-55-40-38 55-8z" fill="var(--ill-yellow)" stroke="var(--file-orange)" stroke-width="3" stroke-linejoin="round" transform="translate(0,-8) scale(.82) translate(26,8)" />
		{:else if art === 'share'}
			<circle cx="92" cy="62" r="22" fill="var(--ill-green)" stroke="var(--file-green)" stroke-width="3" />
			<circle cx="152" cy="70" r="18" fill="var(--accent-soft)" stroke="var(--accent)" stroke-width="3" />
			<path d="M52 138c0-26 18-42 40-42s40 16 40 42z" fill="var(--ill-green)" stroke="var(--file-green)" stroke-width="3" stroke-linejoin="round" />
			<path d="M128 138c0-20 11-34 26-34s26 14 26 34z" fill="var(--accent-soft)" stroke="var(--accent)" stroke-width="3" stroke-linejoin="round" />
		{:else}
			<path d="M60 40h60l28 28v74a6 6 0 0 1-6 6H60a6 6 0 0 1-6-6V46a6 6 0 0 1 6-6z" fill="var(--surface)" stroke="var(--accent)" stroke-width="3" stroke-linejoin="round" />
			<path d="M120 40v28h28" fill="var(--accent-soft)" stroke="var(--accent)" stroke-width="3" stroke-linejoin="round" />
			<path d="M72 92h56M72 108h56M72 124h34" stroke="var(--accent)" stroke-width="3" stroke-linecap="round" opacity=".45" />
			<path d="M160 64c0-12 9-20 20-20 8 0 14 4 17 10 9 0 15 7 15 15s-6 15-15 15h-37c-7 0-12-5-12-12s6-12 12-12z" fill="var(--ill-green)" stroke="var(--file-green)" stroke-width="3" stroke-linejoin="round" transform="translate(-10 22)" />
		{/if}
	</svg>
	<h2>{title}</h2>
	{#if text}<p>{text}</p>{/if}
</div>

<style>
	.empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		text-align: center;
		padding: 64px 24px;
		min-height: 360px;
	}
	h2 {
		margin-top: 24px;
		font-size: 22px;
		line-height: 28px;
		color: var(--text);
	}
	p {
		margin: 8px 0 0;
		max-width: 520px;
		font-size: 16px;
		line-height: 24px;
		color: var(--text-2);
	}
</style>
