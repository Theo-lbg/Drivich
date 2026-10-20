<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t, errorMessage } from '$lib/i18n/index.svelte';
	import { api } from '$lib/api';
	import { formatDateLong } from '$lib/format';
	import type { CommentItem, NodeItem } from '$lib/types';
	import { errorToast } from '$lib/state/toast.svelte';
	import Avatar from './Avatar.svelte';
	import Icon from './Icon.svelte';

	let { node }: { node: NodeItem } = $props();

	let items = $state<CommentItem[]>([]);
	let loading = $state(true);
	let draft = $state('');
	let busy = $state(false);
	let editingId = $state<string | null>(null);
	let editDraft = $state('');
	let error = $state('');

	const canComment = $derived(['owner', 'editor', 'commenter'].includes(node.role));

	$effect(() => {
		const id = node.id;
		loading = true;
		items = [];
		error = '';
		editingId = null;
		api.comments(id)
			.then((r) => {
				if (node.id === id) items = r.items;
			})
			.catch((e) => (error = errorMessage(e)))
			.finally(() => (loading = false));
	});

	async function send(e: Event) {
		e.preventDefault();
		if (busy || !draft.trim()) return;
		busy = true;
		try {
			const r = await api.addComment(node.id, draft);
			items = [...items, r.comment];
			draft = '';
		} catch (err) {
			errorToast(err);
		} finally {
			busy = false;
		}
	}

	async function saveEdit(c: CommentItem) {
		if (busy || !editDraft.trim()) return;
		busy = true;
		try {
			const r = await api.editComment(c.id, editDraft);
			items = items.map((x) => (x.id === c.id ? r.comment : x));
			editingId = null;
		} catch (err) {
			errorToast(err);
		} finally {
			busy = false;
		}
	}

	async function remove(c: CommentItem) {
		try {
			await api.deleteComment(c.id);
			items = items.filter((x) => x.id !== c.id);
		} catch (err) {
			errorToast(err);
		}
	}

	function onkeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) send(e);
	}
</script>

<section class="comments" aria-label={t('comments.title')}>
	<h3>{t('comments.title')}</h3>
	{#if loading}
		<p class="hint">{t('file_browser.chargement')}</p>
	{:else if error}
		<p class="hint">{error}</p>
	{:else if !items.length}
		<p class="hint">{t('comments.empty')}</p>
	{/if}
	<ul>
		{#each items as c (c.id)}
			<li>
				<Avatar name={c.author_name} size={28} userId={c.author_id} version={c.author_avatar_version} />
				<div class="body">
					<div class="meta"><strong>{c.author_name}</strong> <span>{formatDateLong(c.created_at)}</span>{#if c.edited_at}<span>· {t('comments.edited')}</span>{/if}</div>
					{#if editingId === c.id}
						<textarea class="field area" bind:value={editDraft} rows="3" maxlength="4000" aria-label={t('comments.edit')}></textarea>
						<div class="row">
							<button class="btn" onclick={() => (editingId = null)}>{t('dialogs.annuler')}</button>
							<button class="btn filled" disabled={busy || !editDraft.trim()} onclick={() => saveEdit(c)}>{t('comments.save')}</button>
						</div>
					{:else}
						<p class="text">{c.body}</p>
						{#if c.can_edit || c.can_delete}
							<div class="row">
								{#if c.can_edit}<button class="linkish" onclick={() => { editingId = c.id; editDraft = c.body; }}>{t('comments.edit')}</button>{/if}
								{#if c.can_delete}<button class="linkish" onclick={() => remove(c)}>{t('comments.delete')}</button>{/if}
							</div>
						{/if}
					{/if}
				</div>
			</li>
		{/each}
	</ul>
	{#if canComment}
		<form onsubmit={send}>
			<textarea class="field area" bind:value={draft} rows="3" maxlength="4000" placeholder={t('comments.placeholder')} aria-label={t('comments.placeholder')} {onkeydown}></textarea>
			<button class="btn filled" type="submit" disabled={busy || !draft.trim()}><Icon name="send" size={18} />{t('comments.send')}</button>
		</form>
	{:else}
		<p class="hint">{t('comments.read_only')}</p>
	{/if}
</section>

<style>
	.comments {
		margin-top: 24px;
		border-top: 1px solid var(--line);
		padding-top: 16px;
	}
	h3 {
		font-size: 14px;
		margin: 0 0 8px;
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 14px;
	}
	li {
		display: flex;
		gap: 10px;
	}
	.body {
		flex: 1;
		min-width: 0;
	}
	.meta {
		font-size: 12px;
		color: var(--text-2);
	}
	.meta strong {
		color: var(--text);
		font-weight: 500;
	}
	.text {
		margin: 2px 0 0;
		font-size: 14px;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
	.row {
		display: flex;
		gap: 12px;
		margin-top: 4px;
		font-size: 12px;
	}
	.linkish {
		border: 0;
		background: transparent;
		padding: 0;
		color: var(--accent);
	}
	.hint {
		color: var(--text-2);
		font-size: 13px;
		margin: 8px 0;
	}
	form {
		display: flex;
		flex-direction: column;
		gap: 8px;
		align-items: flex-end;
		margin-top: 14px;
	}
	.area {
		width: 100%;
		height: auto;
		padding: 8px 12px;
		font-size: 14px;
	}
</style>
