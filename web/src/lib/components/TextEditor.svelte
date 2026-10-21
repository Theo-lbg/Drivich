<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import { renderMarkdown } from '$lib/markdown';
	import { enqueueVersion } from '$lib/state/uploads.svelte';
	import Icon from './Icon.svelte';

	let { node, initial }: { node: { id: string; name: string }; initial: string } = $props();

	const isMd = $derived(/\.(md|markdown)$/i.test(node.name));
	// The viewer remounts the editor per file, so the initial text is read once on purpose.
	// svelte-ignore state_referenced_locally
	let value = $state(initial);
	// svelte-ignore state_referenced_locally
	let saved = $state(initial);
	let preview = $state(false);
	const dirty = $derived(value !== saved);
	const html = $derived(isMd && preview ? renderMarkdown(value) : '');

	function save() {
		if (!dirty) return;
		enqueueVersion(new File([value], node.name, { type: isMd ? 'text/markdown' : 'text/plain' }), node);
		saved = value;
	}

	function onkeydown(e: KeyboardEvent) {
		if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
			e.preventDefault();
			save();
		}
		e.stopPropagation();
	}
</script>

<div class="editor">
	<div class="bar">
		{#if isMd}
			<button class="btn" class:on={preview} aria-pressed={preview} onclick={() => (preview = !preview)}>
				<Icon name="visibility" size={20} />{t('editor.apercu')}
			</button>
		{/if}
		<span class="grow"></span>
		<span class="state">{dirty ? t('editor.modifie') : t('editor.enregistre')}</span>
		<button class="btn filled" disabled={!dirty} onclick={save}>{t('editor.enregistrer')}</button>
	</div>
	{#if preview}
		<article class="md">{@html html}</article>
	{:else}
		<textarea bind:value spellcheck="false" aria-label={node.name} {onkeydown}></textarea>
	{/if}
</div>

<style>
	.editor {
		display: flex;
		flex-direction: column;
		width: min(960px, 100%);
		height: 100%;
		background: var(--surface);
		color: var(--text);
		border-radius: 8px;
		overflow: hidden;
	}
	.bar {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 8px 12px;
		border-bottom: 1px solid var(--line);
	}
	.grow {
		flex: 1;
	}
	.state {
		color: var(--text-3);
		font-size: 13px;
	}
	.on {
		background: var(--hover-bg);
	}
	textarea {
		flex: 1;
		width: 100%;
		padding: 16px;
		border: 0;
		outline: none;
		resize: none;
		background: transparent;
		color: inherit;
		font: 14px/1.6 ui-monospace, SFMono-Regular, Menlo, monospace;
	}
	.md {
		flex: 1;
		overflow: auto;
		padding: 16px 24px;
		line-height: 1.6;
	}
	.md :global(pre) {
		padding: 12px;
		overflow: auto;
		border-radius: 6px;
		background: var(--hover-bg);
	}
	.md :global(code) {
		font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
	}
	.md :global(blockquote) {
		margin: 8px 0;
		padding-left: 12px;
		border-left: 3px solid var(--line);
		color: var(--text-2);
	}
</style>
