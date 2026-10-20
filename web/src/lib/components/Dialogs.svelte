<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import { api } from '$lib/api';
	import { deleteForever, emptyTrash, moveNodes } from '$lib/actions';
	import { dialogs, refreshAll } from '$lib/state/app.svelte';
	import { errorToast, toast } from '$lib/state/toast.svelte';
	import Dialog from './Dialog.svelte';
	import FolderTree from './FolderTree.svelte';
	import Icon from './Icon.svelte';

	// ---- rename -----------------------------------------------------------------------
	let renameValue = $state('');
	let busy = $state(false);
	$effect(() => {
		if (dialogs.rename) renameValue = dialogs.rename.name;
	});

	function selectBase(el: HTMLInputElement) {
		const n = dialogs.rename;
		el.focus();
		const dot = n && n.kind === 'file' ? n.name.lastIndexOf('.') : -1;
		el.setSelectionRange(0, dot > 0 ? dot : el.value.length);
	}

	async function doRename(e: SubmitEvent) {
		e.preventDefault();
		const n = dialogs.rename;
		if (!n || busy) return;
		busy = true;
		try {
			if (renameValue.trim() && renameValue.trim() !== n.name) {
				await api.rename(n.id, renameValue);
				refreshAll();
			}
			dialogs.rename = null;
		} catch (err) {
			errorToast(err);
		} finally {
			busy = false;
		}
	}

	// ---- new folder --------------------------------------------------------------------------
	let folderName = $state(t('dialogs.dossier_sans_titre'));
	$effect(() => {
		if (dialogs.newFolder) folderName = t('dialogs.dossier_sans_titre');
	});

	async function doCreate(e: SubmitEvent) {
		e.preventDefault();
		const d = dialogs.newFolder;
		if (!d || busy) return;
		busy = true;
		try {
			await api.createFolder(d.parent, folderName);
			toast(t('dialogs.dossier_cree'));
			refreshAll();
			dialogs.newFolder = null;
		} catch (err) {
			errorToast(err);
		} finally {
			busy = false;
		}
	}

	// ---- move ------------------------------------------------------------------------------------
	let target = $state<string | null>(null);
	$effect(() => {
		if (dialogs.move) target = null;
	});

	async function doMove() {
		const items = dialogs.move;
		if (!items) return;
		dialogs.move = null;
		await moveNodes(items, target);
	}

	// ---- permanent delete ---------------------------------------------------------------------------
	async function doDelete() {
		const items = dialogs.confirmDelete;
		dialogs.confirmDelete = null;
		if (items) await deleteForever(items);
	}

	async function doEmpty() {
		dialogs.emptyTrash = false;
		await emptyTrash();
	}
</script>

{#if dialogs.rename}
	<Dialog title={t('dialogs.renommer')} onclose={() => (dialogs.rename = null)} width={420}>
		<form id="rename-form" onsubmit={doRename}>
			<label class="field-label" for="rename-input">{t('dialogs.nom')}</label>
			<input id="rename-input" class="field" bind:value={renameValue} maxlength="255" use:selectBase autocomplete="off" />
		</form>
		{#snippet actions()}
			<button class="btn" onclick={() => (dialogs.rename = null)}>{t('dialogs.annuler')}</button>
			<button class="btn" type="submit" form="rename-form" disabled={busy || !renameValue.trim()}>{t('dialogs.ok')}</button>
		{/snippet}
	</Dialog>
{/if}

{#if dialogs.newFolder}
	<Dialog title={t('dialogs.nouveau_dossier')} onclose={() => (dialogs.newFolder = null)} width={420}>
		<form id="folder-form" onsubmit={doCreate}>
			<input class="field" bind:value={folderName} maxlength="255" aria-label={t('dialogs.nom_du_dossier')} autocomplete="off" onfocus={(e) => e.currentTarget.select()} />
		</form>
		{#snippet actions()}
			<button class="btn" onclick={() => (dialogs.newFolder = null)}>{t('dialogs.annuler')}</button>
			<button class="btn" type="submit" form="folder-form" disabled={busy || !folderName.trim()}>{t('dialogs.creer')}</button>
		{/snippet}
	</Dialog>
{/if}

{#if dialogs.move}
	<Dialog title={dialogs.move.length === 1 ? t('dialogs.move_one', { name: dialogs.move[0].name }) : t('dialogs.move_many', { count: dialogs.move.length })} onclose={() => (dialogs.move = null)} width={480}>
		<div class="picker">
			<button class="root" class:act={target === null} onclick={() => (target = null)}><Icon name="folder" size={20} fill /> {t('advanced_search.mon_drive')}</button>
			<FolderTree parentId={null} depth={0} activeId={target} onselect={(id) => (target = id)} exclude={dialogs.move.map((m) => m.id)} />
		</div>
		{#snippet actions()}
			<button class="btn" onclick={() => (dialogs.move = null)}>{t('dialogs.annuler')}</button>
			<button class="btn filled" onclick={doMove}>{t('dialogs.deplacer_ici')}</button>
		{/snippet}
	</Dialog>
{/if}

{#if dialogs.confirmDelete}
	<Dialog title={t('dialogs.supprimer_definitivement')} onclose={() => (dialogs.confirmDelete = null)} width={440}>
		<p class="msg">
			{dialogs.confirmDelete.length === 1 ? t('dialogs.delete_confirm_one', { name: dialogs.confirmDelete[0].name }) : t('dialogs.delete_confirm_many', { count: dialogs.confirmDelete.length })}
		</p>
		{#snippet actions()}
			<button class="btn" onclick={() => (dialogs.confirmDelete = null)}>{t('dialogs.annuler')}</button>
			<button class="btn danger" onclick={doDelete}>{t('dialogs.supprimer_definitivement_2')}</button>
		{/snippet}
	</Dialog>
{/if}

{#if dialogs.emptyTrash}
	<Dialog title={t('dialogs.vider_la_corbeille')} onclose={() => (dialogs.emptyTrash = false)} width={440}>
		<p class="msg">{t('dialogs.tous_les_elements_de_la')}</p>
		{#snippet actions()}
			<button class="btn" onclick={() => (dialogs.emptyTrash = false)}>{t('dialogs.annuler')}</button>
			<button class="btn danger" onclick={doEmpty}>{t('dialogs.vider_la_corbeille_2')}</button>
		{/snippet}
	</Dialog>
{/if}

<style>
	.msg {
		margin: 0;
		color: var(--text-2);
		font-size: 16px;
		line-height: 24px;
	}
	.picker {
		border: 1px solid var(--line);
		border-radius: 12px;
		padding: 8px;
		max-height: 320px;
		overflow: auto;
	}
	.root {
		display: flex;
		align-items: center;
		gap: 12px;
		width: 100%;
		height: 32px;
		padding: 0 12px 0 28px;
		border: 0;
		border-radius: 16px;
		background: transparent;
		color: var(--text);
		text-align: left;
	}
	.root:hover {
		background: var(--hover-bg);
	}
	.root.act {
		background: var(--accent-soft);
	}
</style>
