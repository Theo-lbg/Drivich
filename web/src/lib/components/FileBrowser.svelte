<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { errorMessage, t } from '$lib/i18n/index.svelte';
	import { goto } from '$app/navigation';
	import { api, ApiError, type SearchOptions } from '$lib/api';
	import {
		askDelete, askMove, askNewVersion, askRename, askShare, deleteForever, downloadMany, downloadNode, folderHref, openNode, restoreNodes, setStarred, trashNodes
	} from '$lib/actions';
	import { daysLeft, formatBytes, formatDate, typeLabel } from '$lib/format';
	import { dialogs, nav, pickers } from '$lib/state/app.svelte';
	import { session } from '$lib/state/session.svelte';
	import { setLayout, setSort, ui } from '$lib/state/ui.svelte';
	import { enqueue, enqueueWithPaths, filesFromDataTransfer } from '$lib/state/uploads.svelte';
	import { errorToast, toast } from '$lib/state/toast.svelte';
	import type { Crumb, Filters, ListView, NodeItem, SortKey } from '$lib/types';
	import { moveNodes } from '$lib/actions';
	import Avatar from './Avatar.svelte';
	import DetailsPanel from './DetailsPanel.svelte';
	import EmptyState from './EmptyState.svelte';
	import FilterChips from './FilterChips.svelte';
	import Icon from './Icon.svelte';
	import Menu, { type MenuItem } from './Menu.svelte';
	import Thumb from './Thumb.svelte';

	type Mode = ListView | 'search';

	let {
		mode,
		parent = null,
		title,
		search = {},
		filters: extFilters = null,
		onfilters
	}: {
		mode: Mode;
		parent?: string | null;
		title: string;
		search?: SearchOptions;
		/** Search mode: filters live in the URL, owned by the page. */
		filters?: Filters | null;
		onfilters?: (patch: Partial<Filters>) => void;
	} = $props();

	// ---- data ----------------------------------------------------------------------
	let items = $state<NodeItem[]>([]);
	let cursor = $state<string | null>(null);
	let loading = $state(false);
	let loaded = $state(false);
	let error = $state<string | null>(null);
	let generation = 0;

	let localFilters = $state<Filters>({ type: '', date: '', owner: '', location: '' });
	const filters = $derived(extFilters ?? localFilters);
	const isSearch = $derived(mode === 'search');
	const sortable = $derived(mode === 'drive' || mode === 'starred' || mode === 'shared');

	let folder = $state<NodeItem | null>(null);
	let crumbs = $state<Crumb[]>([]);

	function dateFrom(d: string): string | undefined {
		const now = new Date();
		if (d === 'today') return new Date(now.getFullYear(), now.getMonth(), now.getDate()).toISOString();
		if (d === '7d') return new Date(now.getTime() - 7 * 86_400_000).toISOString();
		if (d === '30d') return new Date(now.getTime() - 30 * 86_400_000).toISOString();
		if (d === 'year') return new Date(now.getFullYear(), 0, 1).toISOString();
		return undefined;
	}

	async function fetchPage(cur: string | null, limit?: number) {
		if (isSearch) return api.search({ ...search, cursor: cur });
		return api.list({
			view: mode as ListView,
			parent,
			sort: ui.sort,
			order: ui.order,
			type: filters.type || undefined,
			modified_from: dateFrom(filters.date),
			owner: filters.owner || undefined,
			location: filters.location || undefined,
			cursor: cur,
			limit
		});
	}

	const queryKey = $derived(JSON.stringify([mode, parent, ui.sort, ui.order, filters, search, nav.tick]));
	let lastKey = '';

	$effect(() => {
		const key = queryKey;
		if (key === lastKey) return;
		const structural = lastKey === '' || JSON.parse(lastKey).slice(0, 2).join() !== JSON.parse(key).slice(0, 2).join();
		lastKey = key;
		reload(structural);
	});

	async function reload(structural: boolean) {
		const my = ++generation;
		loading = true;
		error = null;
		if (structural) {
			items = [];
			loaded = false;
			selected = new Set();
			folder = null;
			crumbs = [];
		}
		try {
			// Reload everything already shown (keeps scroll position after refreshes).
			const want = structural ? 0 : items.length;
			let page = await fetchPage(null);
			let all = page.items;
			while (want > all.length && page.next_cursor) {
				page = await fetchPage(page.next_cursor);
				all = all.concat(page.items);
			}
			if (my !== generation) return;
			items = all;
			cursor = page.next_cursor;
			selected = new Set([...selected].filter((id) => all.some((n) => n.id === id)));
			loaded = true;
		} catch (e) {
			if (my !== generation) return;
			error = e instanceof ApiError ? errorMessage(e) : t('file_browser.impossible_de_charger_la_liste');
			if (e instanceof ApiError && (e.status === 404 || e.status === 403) && mode === 'drive' && parent) {
				toast(t('file_browser.ce_dossier_est_introuvable_ou'), { kind: 'error' });
				goto('/drive');
			}
		} finally {
			if (my === generation) loading = false;
		}
		if (structural && mode === 'drive') loadFolder(my);
	}

	async function loadFolder(my: number) {
		if (!parent) {
			nav.folderId = null;
			nav.canWrite = true;
			return;
		}
		try {
			const [n, p] = await Promise.all([api.node(parent), api.path(parent)]);
			if (my !== generation) return;
			folder = n;
			crumbs = p.items;
			nav.canWrite = n.role === 'owner' || n.role === 'editor';
		} catch {
			/* handled by the list error */
		}
	}

	$effect(() => {
		if (mode === 'drive') {
			nav.folderId = parent;
			if (!parent) nav.canWrite = true;
		} else {
			nav.folderId = null;
			nav.canWrite = false;
		}
	});

	async function loadMore() {
		if (!cursor || loading) return;
		const my = generation;
		loading = true;
		try {
			const page = await fetchPage(cursor);
			if (my !== generation) return;
			const seen = new Set(items.map((n) => n.id));
			items = items.concat(page.items.filter((n) => !seen.has(n.id)));
			cursor = page.next_cursor;
		} catch (e) {
			errorToast(e);
		} finally {
			loading = false;
		}
	}

	let sentinel: HTMLDivElement | undefined = $state();
	$effect(() => {
		if (!sentinel) return;
		const io = new IntersectionObserver((e) => e[0].isIntersecting && loadMore(), { rootMargin: '400px' });
		io.observe(sentinel);
		return () => io.disconnect();
	});

	// Thumbnails are generated in the background: refresh while some are pending.
	let pollTries = 0;
	$effect(() => {
		const pending = items.some((n) => n.thumb_state === 'pending');
		if (!pending) {
			pollTries = 0;
			return;
		}
		if (pollTries >= 12) return;
		const timer = setTimeout(() => {
			pollTries++;
			reload(false);
		}, 2500);
		return () => clearTimeout(timer);
	});

	function patchItem(n: NodeItem) {
		items = items.map((i) => (i.id === n.id ? n : i));
	}

	// ---- selection -------------------------------------------------------------------
	let selected = $state(new Set<string>());
	let lastClicked: string | null = null;
	const selectedItems = $derived(items.filter((n) => selected.has(n.id)));
	const coarse = typeof matchMedia !== 'undefined' && matchMedia('(pointer: coarse)').matches;

	// Touch: a long press starts a selection, then taps add to or remove from it.
	let pressTimer: ReturnType<typeof setTimeout> | undefined;
	let longPressed = false;
	function pressStart(e: PointerEvent, n: NodeItem) {
		if (e.pointerType !== 'touch' || (e.target as HTMLElement).closest('button, a, input, textarea')) return;
		longPressed = false;
		clearTimeout(pressTimer);
		pressTimer = setTimeout(() => {
			longPressed = true;
			toggleCheck(n);
			navigator.vibrate?.(10);
		}, 450);
	}
	function pressEnd() {
		clearTimeout(pressTimer);
	}

	function select(e: MouseEvent, n: NodeItem) {
		if ((e.target as HTMLElement).closest('button, a, input, textarea')) return;
		if (longPressed) {
			longPressed = false;
			return;
		}
		if (coarse) return selected.size ? toggleCheck(n) : open(n);
		if (e.shiftKey && lastClicked) {
			const a = items.findIndex((i) => i.id === lastClicked);
			const b = items.findIndex((i) => i.id === n.id);
			const [from, to] = a < b ? [a, b] : [b, a];
			selected = new Set(items.slice(from, to + 1).map((i) => i.id));
		} else if (e.metaKey || e.ctrlKey) {
			const s = new Set(selected);
			s.has(n.id) ? s.delete(n.id) : s.add(n.id);
			selected = s;
			lastClicked = n.id;
		} else {
			selected = new Set([n.id]);
			lastClicked = n.id;
		}
	}

	/** Keyboard access on a focused row: Enter opens, Space selects. */
	function rowKey(e: KeyboardEvent, n: NodeItem) {
		if ((e.target as HTMLElement).closest('button, a, input, textarea')) return;
		if (e.key === 'Enter') {
			e.preventDefault();
			open(n);
		} else if (e.key === ' ') {
			e.preventDefault();
			toggleCheck(n);
		}
	}

	function toggleCheck(n: NodeItem) {
		const s = new Set(selected);
		s.has(n.id) ? s.delete(n.id) : s.add(n.id);
		selected = s;
		lastClicked = n.id;
	}

	const allSelected = $derived(items.length > 0 && selected.size === items.length);
	function toggleAll() {
		selected = allSelected ? new Set() : new Set(items.map((n) => n.id));
	}

	function open(n: NodeItem) {
		openNode(n, items);
		if (n.kind === 'file') api.opened(n.id).catch(() => {});
	}

	function onkeydown(e: KeyboardEvent) {
		const t = e.target as HTMLElement;
		if (t.closest('input, textarea, select') || dialogs.rename || dialogs.share || dialogs.move) return;
		const sel = selectedItems;
		if (e.key === 'Escape') selected = new Set();
		else if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'a') {
			e.preventDefault();
			selected = new Set(items.map((n) => n.id));
		} else if (e.key === 'Enter' && sel.length === 1) open(sel[0]);
		else if ((e.key === 'Delete' || e.key === 'Backspace') && sel.length && mode !== 'trash' && sel.every(isOwner)) trashNodes(sel);
		else if (e.key === 'F2' && sel.length === 1 && canEdit(sel[0])) askRename(sel[0]);
	}

	const isOwner = (n: NodeItem) => n.role === 'owner';
	const canEdit = (n: NodeItem) => n.role === 'owner' || n.role === 'editor';

	// ---- context menu ----------------------------------------------------------------------
	let menu = $state<{ x: number; y: number; items: MenuItem[] } | null>(null);

	function menuFor(targets: NodeItem[]): MenuItem[] {
		const one = targets.length === 1 ? targets[0] : null;
		if (mode === 'trash') {
			return [
				{ label: t('file_browser.restaurer'), icon: 'restore_from_trash', run: () => restoreNodes(targets) },
				{ label: t('dialogs.supprimer_definitivement_2'), icon: 'delete_forever', danger: true, run: () => askDelete(targets) }
			];
		}
		const owner = targets.every(isOwner);
		const editor = targets.every(canEdit);
		const out: MenuItem[] = [];
		if (!one) out.push({ label: t('file_browser.telecharger_zip'), icon: 'download', run: () => downloadMany(targets) });
		if (one) {
			out.push({ label: t('file_browser.ouvrir'), icon: one.kind === 'folder' ? 'folder_open' : 'open_in_new', run: () => open(one) });
			out.push({ label: one.kind === 'folder' ? t('file_browser.telecharger_zip') : t('file_browser.telecharger'), icon: 'download', run: () => downloadNode(one) });
			if (one.kind === 'file' && editor) out.push({ label: t('file_browser.new_version'), icon: 'upload_file', run: () => askNewVersion(one) });
			out.push({ divider: true });
			if (editor) out.push({ label: t('dialogs.renommer'), icon: 'edit', hint: 'F2', run: () => askRename(one) });
			if (owner) out.push({ label: t('file_browser.partager'), icon: 'share', run: () => askShare(one) });
		}
		if (owner) {
			const allStarred = targets.every((t) => t.starred);
			out.push({ label: allStarred ? t('file_browser.retirer_des_favoris') : t('file_browser.ajouter_aux_favoris'), icon: allStarred ? 'star' : 'star', run: () => setStarred(targets, !allStarred) });
		}
		if (owner) out.push({ label: t('file_browser.deplacer_vers_2'), icon: 'drive_file_move', run: () => askMove(targets) });
		if (owner) {
			out.push({ divider: true });
			out.push({ label: t('file_browser.mettre_dans_la_corbeille'), icon: 'delete', hint: t('file_browser.suppr'), run: () => trashNodes(targets) });
		}
		return out;
	}

	function onContext(e: MouseEvent, n: NodeItem) {
		e.preventDefault();
		if (coarse) return; // the long press selects instead (see pressStart)
		if (!selected.has(n.id)) {
			selected = new Set([n.id]);
			lastClicked = n.id;
		}
		menu = { x: e.clientX, y: e.clientY, items: menuFor(selectedItems) };
	}

	function onBackgroundContext(e: MouseEvent) {
		if ((e.target as HTMLElement).closest('.row, .card')) return;
		e.preventDefault();
		selected = new Set();
		if (mode === 'trash') {
			menu = items.length ? { x: e.clientX, y: e.clientY, items: [{ label: t('dialogs.vider_la_corbeille_2'), icon: 'delete_forever', danger: true, run: () => (dialogs.emptyTrash = true) }] } : null;
			return;
		}
		if (mode !== 'drive' || !nav.canWrite) return;
		menu = {
			x: e.clientX,
			y: e.clientY,
			items: [
				{ label: t('dialogs.nouveau_dossier'), icon: 'create_new_folder', run: () => (dialogs.newFolder = { parent }) },
				{ divider: true },
				{ label: t('file_browser.importer_un_fichier'), icon: 'upload_file', run: () => pickers.files?.() },
				{ label: t('file_browser.importer_un_dossier'), icon: 'drive_folder_upload', run: () => pickers.folder?.() }
			]
		};
	}

	function kebab(e: MouseEvent, n: NodeItem) {
		e.stopPropagation();
		if (!selected.has(n.id)) selected = new Set([n.id]);
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		menu = { x: r.left - 180, y: r.bottom, items: menuFor(selectedItems) };
	}

	// ---- drag and drop --------------------------------------------------------------------------
	let dropTarget = $state<string | null>(null);
	let fileDrag = $state(false);
	let dragDepth = 0;

	function dragStart(e: DragEvent, n: NodeItem) {
		if (!canEdit(n) || mode === 'trash') return e.preventDefault();
		if (!selected.has(n.id)) selected = new Set([n.id]);
		e.dataTransfer?.setData('application/x-drivich', JSON.stringify([...selected]));
		if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
	}

	const hasFiles = (e: DragEvent) => !!e.dataTransfer?.types.includes('Files');
	const hasNodes = (e: DragEvent) => !!e.dataTransfer?.types.includes('application/x-drivich');

	function folderDragOver(e: DragEvent, n: NodeItem) {
		if (n.kind !== 'folder' || !canEdit(n) || (!hasNodes(e) && !hasFiles(e))) return;
		if (hasNodes(e) && selected.has(n.id)) return;
		e.preventDefault();
		e.stopPropagation();
		dropTarget = n.id;
	}

	async function folderDrop(e: DragEvent, n: NodeItem) {
		if (n.kind !== 'folder' || !canEdit(n)) return;
		e.preventDefault();
		e.stopPropagation();
		dropTarget = null;
		fileDrag = false;
		dragDepth = 0;
		if (hasNodes(e)) {
			const ids: string[] = JSON.parse(e.dataTransfer!.getData('application/x-drivich') || '[]');
			const moving = items.filter((i) => ids.includes(i.id) && i.id !== n.id);
			if (moving.length) await moveNodes(moving, n.id);
		} else if (hasFiles(e)) {
			await dropFiles(e, n.id);
		}
	}

	async function dropFiles(e: DragEvent, target: string | null) {
		const list = await filesFromDataTransfer(e.dataTransfer!);
		if (!list.length) return;
		await enqueueWithPaths(list, target);
	}

	function panelEnter(e: DragEvent) {
		if (!hasFiles(e) || mode !== 'drive' || !nav.canWrite) return;
		e.preventDefault();
		dragDepth++;
		fileDrag = true;
	}
	function panelLeave() {
		dragDepth = Math.max(0, dragDepth - 1);
		if (dragDepth === 0) fileDrag = false;
	}
	function panelOver(e: DragEvent) {
		if (hasFiles(e) && mode === 'drive' && nav.canWrite) e.preventDefault();
	}
	async function panelDrop(e: DragEvent) {
		if (!hasFiles(e) || mode !== 'drive' || !nav.canWrite) return;
		e.preventDefault();
		fileDrag = false;
		dragDepth = 0;
		await dropFiles(e, parent);
	}

	// ---- header helpers -------------------------------------------------------------------------------
	const owned = $derived(!folder || folder.owner_id === session.user?.id);
	let headMenu = $state<{ x: number; y: number; items: MenuItem[] } | null>(null);

	function headDropdown(e: MouseEvent) {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		const its: MenuItem[] = [];
		if (nav.canWrite) {
			its.push({ label: t('dialogs.nouveau_dossier'), icon: 'create_new_folder', run: () => (dialogs.newFolder = { parent }) });
			its.push({ divider: true });
			its.push({ label: t('file_browser.importer_un_fichier'), icon: 'upload_file', run: () => pickers.files?.() });
			its.push({ label: t('file_browser.importer_un_dossier'), icon: 'drive_folder_upload', run: () => pickers.folder?.() });
		}
		if (folder && owned) {
			its.push({ divider: true });
			its.push({ label: t('file_browser.partager'), icon: 'share', run: () => askShare(folder!) });
		}
		if (its.length) headMenu = { x: r.left, y: r.bottom + 4, items: its };
	}

	const sortLabel = (k: SortKey) => (k === 'name' ? t('file_browser.nom') : k === 'size' ? t('details_panel.taille') : dateHeader);
	let sortMenu = $state<{ x: number; y: number; items: MenuItem[] } | null>(null);
	function sortMenuOpen(e: MouseEvent) {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		const keys: SortKey[] = ['name', 'modified', 'size'];
		sortMenu = {
			x: r.left,
			y: r.bottom + 4,
			items: keys.map((k) => ({ label: sortLabel(k), icon: ui.sort === k ? (ui.order === 'asc' ? 'arrow_upward' : 'arrow_downward') : undefined, checked: ui.sort === k, run: () => sortBy(k) }))
		};
	}

	function sortBy(key: SortKey) {
		if (sortable) setSort(key);
	}

	function patchFilters(p: Partial<Filters>) {
		if (onfilters) onfilters(p);
		else localFilters = { ...localFilters, ...p };
	}

	/** "Récents" is grouped by when the file was last opened or modified, like Drive. */
	function activityOf(n: NodeItem): Date {
		const opened = n.last_opened_at ? new Date(n.last_opened_at) : null;
		const updated = new Date(n.updated_at);
		return opened && opened > updated ? opened : updated;
	}

	function groupLabel(n: NodeItem): string {
		const d = activityOf(n);
		const now = new Date();
		const day = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
		const diffDays = Math.round((day(now) - day(d)) / 86_400_000);
		if (diffDays <= 0) return t('advanced_search.aujourdhui');
		if (diffDays === 1) return t('file_browser.hier');
		if (diffDays < 7) return t('file_browser.cette_semaine');
		if (d.getFullYear() === now.getFullYear() && d.getMonth() === now.getMonth()) return t('file_browser.ce_mois_ci');
		const last = new Date(now.getFullYear(), now.getMonth() - 1, 1);
		if (d.getFullYear() === last.getFullYear() && d.getMonth() === last.getMonth()) return t('file_browser.le_mois_dernier');
		if (d.getFullYear() === now.getFullYear()) return t('file_browser.plus_tot_cette_annee');
		return String(d.getFullYear());
	}

	const recentGroups = $derived.by(() => {
		if (mode !== 'recent') return [];
		const groups: { label: string; items: NodeItem[] }[] = [];
		for (const n of items) {
			const label = groupLabel(n);
			const last = groups.at(-1);
			if (last && last.label === label) last.items.push(n);
			else groups.push({ label, items: [n] });
		}
		return groups;
	});

	const dateHeader = $derived(mode === 'trash' ? t('file_browser.date_de_suppression') : mode === 'recent' ? t('file_browser.derniere_ouverture') : t('file_browser.derniere_modification'));
	/** Second line of a row on phones: "owner · date", like Drive. */
	function subtitle(n: NodeItem): string {
		const date = formatDate(dateOf(n));
		let when = mode === 'drive' || mode === 'starred' ? t('file_browser.modifie_le', { date }) : date;
		if (mode === 'trash') {
			const d = daysLeft(n.trashed_at);
			if (d !== null) when += ` · ${t('file_browser.days_left', { count: d })}`;
		}
		const mine = n.owner_id === session.user?.id;
		return mine && (mode === 'drive' || mode === 'starred') ? when : `${mine ? 'moi' : n.owner_name} · ${when}`;
	}
	const dateOf = (n: NodeItem) => (mode === 'trash' ? n.trashed_at : mode === 'recent' ? (n.last_opened_at ?? n.updated_at) : n.updated_at);
	const folders = $derived(items.filter((n) => n.kind === 'folder'));
	const files = $derived(items.filter((n) => n.kind === 'file'));

	const emptyMessage = $derived.by(() => {
		if (isSearch) return { title: t('file_browser.aucun_resultat'), text: t('file_browser.essayez_avec_dautres_mots_cles'), art: 'search' as const };
		if (filters.type || filters.date || filters.owner) return { title: t('file_browser.aucun_element_ne_correspond_a'), text: t('file_browser.modifiez_ou_effacez_les_filtres'), art: 'search' as const };
		switch (mode) {
			case 'trash': return { title: t('file_browser.la_corbeille_est_vide'), text: t('file_browser.les_elements_places_dans_la_2'), art: 'trash' as const };
			case 'starred': return { title: t('file_browser.aucun_fichier_favori'), text: t('file_browser.ajoutez_des_etoiles_aux_elements'), art: 'star' as const };
			case 'shared': return { title: t('file_browser.rien_na_ete_partage_avec'), text: t('file_browser.les_fichiers_et_dossiers_que'), art: 'share' as const };
			case 'recent': return { title: t('file_browser.aucun_fichier_recent'), text: t('file_browser.les_fichiers_que_vous_ouvrez'), art: 'files' as const };
			default: return { title: t('routes.un_emplacement_centralise_pour_tous'), text: t('file_browser.faites_glisser_vos_fichiers_et'), art: 'files' as const };
		}
	});
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<section
	class="panel"
	class:dropping={fileDrag}
	aria-label={title}
	tabindex="-1"
	{onkeydown}
	ondragenter={panelEnter}
	ondragleave={panelLeave}
	ondragover={panelOver}
	ondrop={panelDrop}
	oncontextmenu={onBackgroundContext}
>
	<div class="main">
		<header class="head">
			<div class="title">
				{#if mode === 'drive'}
					<nav class="crumbs" aria-label={t('file_browser.fil_dariane')}>
						{#if owned || !parent}
							{#if parent}
								<a class="crumb" href="/drive">{t('advanced_search.mon_drive')}</a>
								<Icon name="chevron_right" size={20} class="sep" />
							{/if}
						{:else}
							<a class="crumb" href="/shared">{t('advanced_search.partages_avec_moi')}</a>
							<Icon name="chevron_right" size={20} class="sep" />
						{/if}
						{#each crumbs.slice(0, -1) as c}
							<a class="crumb" href={folderHref(c.id)}>{c.name}</a>
							<Icon name="chevron_right" size={20} class="sep" />
						{/each}
						<button class="crumb last" onclick={headDropdown} aria-haspopup="menu">
							<span class="ellipsis">{parent ? (crumbs.at(-1)?.name ?? folder?.name ?? '…') : t('advanced_search.mon_drive')}</span>
							<Icon name="arrow_drop_down" size={24} />
						</button>
					</nav>
				{:else}
					<h1 class="ellipsis">{title}</h1>
				{/if}
			</div>
			<div class="tools">
				{#if mode === 'trash' && items.length}
					<button class="btn" onclick={() => (dialogs.emptyTrash = true)}>{t('dialogs.vider_la_corbeille_2')}</button>
				{/if}
				<div class="seg" role="radiogroup" aria-label={t('file_browser.disposition_en_liste_ou_grille')}>
					<button class="segbtn" class:on={ui.layout === 'list'} role="radio" aria-checked={ui.layout === 'list'} title={t('file_browser.affichage_en_liste')} onclick={() => setLayout('list')}>
						<Icon name={ui.layout === 'list' ? 'check' : 'view_list'} size={20} />
						{#if ui.layout === 'list'}<Icon name="view_list" size={20} />{/if}
					</button>
					<button class="segbtn" class:on={ui.layout === 'grid'} role="radio" aria-checked={ui.layout === 'grid'} title={t('file_browser.affichage_en_grille')} onclick={() => setLayout('grid')}>
						<Icon name={ui.layout === 'grid' ? 'check' : 'grid_view'} size={20} />
						{#if ui.layout === 'grid'}<Icon name="grid_view" size={20} />{/if}
					</button>
				</div>
				<button class="icon-btn" class:on={ui.details} title={t('file_browser.afficher_les_details')} aria-label={t('file_browser.afficher_les_details')} aria-pressed={ui.details} onclick={() => (ui.details = !ui.details)}>
					<Icon name="info" size={24} />
				</button>
			</div>
		</header>

		{#if mode === 'trash'}
			<div class="banner">
				<Icon name="info" size={20} /> {t('file_browser.les_elements_places_dans_la')}
			</div>
		{/if}

		{#if selectedItems.length}
			<div class="selbar" role="toolbar" aria-label={t('file_browser.actions')}>
				<button class="icon-btn small" aria-label={t('file_browser.deselectionner')} onclick={() => (selected = new Set())}><Icon name="close" size={20} /></button>
				<span class="count">{t('file_browser.selected', { count: selectedItems.length })}</span>
				{#if mode === 'trash'}
					<button class="btn" onclick={() => restoreNodes(selectedItems)}><Icon name="restore_from_trash" size={20} />{t('file_browser.restaurer')}</button>
					<button class="btn danger" onclick={() => askDelete(selectedItems)}><Icon name="delete_forever" size={20} />{t('dialogs.supprimer_definitivement_2')}</button>
				{:else}
					<button class="icon-btn" title={selectedItems.length === 1 && selectedItems[0].kind === 'file' ? t('file_browser.telecharger') : t('file_browser.telecharger_zip')} aria-label={t('file_browser.telecharger')} onclick={() => downloadMany(selectedItems)}><Icon name="download" size={22} /></button>
					{#if selectedItems.length === 1 && isOwner(selectedItems[0])}
						<button class="icon-btn" title={t('file_browser.partager')} aria-label={t('file_browser.partager')} onclick={() => askShare(selectedItems[0])}><Icon name="share" size={22} /></button>
					{/if}
					{#if selectedItems.every(isOwner)}
						<button class="icon-btn" title={t('file_browser.deplacer_vers')} aria-label={t('file_browser.deplacer_vers')} onclick={() => askMove(selectedItems)}><Icon name="drive_file_move" size={22} /></button>
					{/if}
					{#if selectedItems.every(isOwner)}
						<button class="icon-btn" title={t('file_browser.ajouter_aux_favoris')} aria-label={t('file_browser.ajouter_aux_favoris')} onclick={() => setStarred(selectedItems, !selectedItems.every((s) => s.starred))}><Icon name="star" size={22} fill={selectedItems.every((s) => s.starred)} /></button>
						<button class="icon-btn" title={t('file_browser.mettre_dans_la_corbeille')} aria-label={t('file_browser.mettre_dans_la_corbeille')} onclick={() => trashNodes(selectedItems)}><Icon name="delete" size={22} /></button>
					{/if}
				{/if}
			</div>
		{:else if mode !== 'trash'}
			<FilterChips
				{filters}
				onchange={patchFilters}
				showLocation={isSearch || mode === 'recent'}
				locations={mode === 'recent'
					? [
							['', t('advanced_search.partout')],
							['drive', t('advanced_search.mon_drive')],
							['shared', t('advanced_search.partages_avec_moi')]
						]
					: null}
				showOwner={mode !== 'starred'}
			/>
		{/if}

		<div class="msort">
			<button class="chip" disabled={!sortable} onclick={sortMenuOpen} aria-haspopup="menu">
				{sortLabel(ui.sort)}
				{#if sortable}<Icon name={ui.order === 'asc' ? 'arrow_upward' : 'arrow_downward'} size={18} />{/if}
			</button>
			<button class="icon-btn" title={ui.layout === 'list' ? t('file_browser.affichage_en_grille') : t('file_browser.affichage_en_liste')} aria-label={ui.layout === 'list' ? t('file_browser.affichage_en_grille') : t('file_browser.affichage_en_liste')} onclick={() => setLayout(ui.layout === 'list' ? 'grid' : 'list')}>
				<Icon name={ui.layout === 'list' ? 'grid_view' : 'view_list'} size={24} />
			</button>
		</div>

		<div class="scroller">
			{#if ui.layout === 'list'}
				{#if items.length}
					<div class="row head-row" role="row">
						<div class="c-name">
							<input type="checkbox" class="chk" aria-label={t('file_browser.tout_selectionner')} checked={allSelected} indeterminate={selected.size > 0 && !allSelected} onchange={toggleAll} />
							<button class="th" class:sortable disabled={!sortable} onclick={() => sortBy('name')}>
								Nom {#if sortable && ui.sort === 'name'}<Icon name={ui.order === 'asc' ? 'arrow_upward' : 'arrow_downward'} size={16} />{/if}
							</button>
						</div>
						<div class="c-owner th">{t('details_panel.proprietaire')}</div>
						<div class="c-date">
							<button class="th" class:sortable disabled={!sortable} onclick={() => sortBy('modified')}>
								{dateHeader} {#if sortable && ui.sort === 'modified'}<Icon name={ui.order === 'asc' ? 'arrow_upward' : 'arrow_downward'} size={16} />{/if}
							</button>
						</div>
						<div class="c-size">
							<button class="th" class:sortable disabled={!sortable} onclick={() => sortBy('size')}>
								Taille {#if sortable && ui.sort === 'size'}<Icon name={ui.order === 'asc' ? 'arrow_upward' : 'arrow_downward'} size={16} />{/if}
							</button>
						</div>
						<div class="c-act"></div>
					</div>
					{#each items as n, i (n.id)}
						{#if mode === 'recent' && (i === 0 || groupLabel(items[i - 1]) !== groupLabel(n))}
							<div class="grouprow" role="presentation">{groupLabel(n)}</div>
						{/if}
						<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
						<div
							class="row"
							class:sel={selected.has(n.id)}
							class:drop={dropTarget === n.id}
							role="row"
							tabindex="0"
							aria-selected={selected.has(n.id)}
							draggable={canEdit(n) && mode !== 'trash'}
							onclick={(e) => select(e, n)}
							onkeydown={(e) => rowKey(e, n)}
							ondblclick={() => open(n)}
							oncontextmenu={(e) => onContext(e, n)}
							onpointerdown={(e) => pressStart(e, n)}
							onpointerup={pressEnd}
							onpointermove={pressEnd}
							onpointercancel={pressEnd}
							ondragstart={(e) => dragStart(e, n)}
							ondragover={(e) => folderDragOver(e, n)}
							ondragleave={() => dropTarget === n.id && (dropTarget = null)}
							ondrop={(e) => folderDrop(e, n)}
						>
							<div class="c-name">
								<span class="iconslot">
									<input type="checkbox" class="chk" aria-label={t('file_browser.select_item', { name: n.name })} checked={selected.has(n.id)} onchange={() => toggleCheck(n)} />
									<span class="typeico"><Thumb node={n} size={24} /></span>
								</span>
								<span class="nmcol">
									<span class="name ellipsis" title={n.name}>{n.name}</span>
									<span class="sub ellipsis">{subtitle(n)}</span>
								</span>
								{#if n.starred}<Icon name="star" size={16} fill class="starred" />{/if}
							</div>
							<div class="c-owner">
								<Avatar name={n.owner_name} size={24} userId={n.owner_id} version={n.owner_avatar_version} />
								<span class="ellipsis">{n.owner_id === session.user?.id ? 'moi' : n.owner_name}</span>
							</div>
							<div class="c-date ellipsis" title={dateOf(n) ?? ''}>
								{formatDate(dateOf(n))}{#if mode === 'trash'}{@const d = daysLeft(n.trashed_at)}{#if d !== null}<span class="muted small"> · {t('file_browser.days_left', { count: d })}</span>{/if}{/if}
							</div>
							<div class="c-size ellipsis">{n.kind === 'folder' ? '—' : formatBytes(n.size)}</div>
							<div class="c-act">
								{#if mode === 'trash'}
									<button class="icon-btn small hov" title={t('file_browser.restaurer')} aria-label={t('file_browser.restaurer')} onclick={() => restoreNodes([n])}><Icon name="restore_from_trash" size={20} /></button>
								{:else}
									<button class="icon-btn small hov" title={n.kind === 'folder' ? t('file_browser.telecharger_zip') : t('file_browser.telecharger')} aria-label={t('file_browser.telecharger')} onclick={() => downloadNode(n)}><Icon name="download" size={20} /></button>
									{#if isOwner(n)}<button class="icon-btn small hov" title={t('file_browser.partager')} aria-label={t('file_browser.partager')} onclick={() => askShare(n)}><Icon name="person_add" size={20} /></button>{/if}
								{/if}
								<button class="icon-btn small" title={t('file_browser.plus_dactions')} aria-label={t('file_browser.plus_dactions')} onclick={(e) => kebab(e, n)}><Icon name="more_vert" size={20} /></button>
							</div>
						</div>
					{/each}
				{/if}
			{:else}
				<div class="grid-wrap">
					{#if folders.length}
						<h3 class="sec">{t('advanced_search.dossiers')}</h3>
						<div class="grid folders">
							{#each folders as n (n.id)}
								<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
								<div
									class="card folder"
									class:sel={selected.has(n.id)}
									class:drop={dropTarget === n.id}
									role="row"
									tabindex="0"
									draggable={canEdit(n) && mode !== 'trash'}
									onclick={(e) => select(e, n)}
									onkeydown={(e) => rowKey(e, n)}
									ondblclick={() => open(n)}
									oncontextmenu={(e) => onContext(e, n)}
							onpointerdown={(e) => pressStart(e, n)}
							onpointerup={pressEnd}
							onpointermove={pressEnd}
							onpointercancel={pressEnd}
									ondragstart={(e) => dragStart(e, n)}
									ondragover={(e) => folderDragOver(e, n)}
									ondragleave={() => dropTarget === n.id && (dropTarget = null)}
									ondrop={(e) => folderDrop(e, n)}
								>
									<Thumb node={n} size={24} />
									<span class="name ellipsis" title={n.name}>{n.name}</span>
									<button class="icon-btn small" aria-label={t('file_browser.plus_dactions')} onclick={(e) => kebab(e, n)}><Icon name="more_vert" size={20} /></button>
								</div>
							{/each}
						</div>
					{/if}
					{#if mode === 'recent'}
						{#each recentGroups as g (g.label)}
							<h3 class="sec">{g.label}</h3>
							<div class="grid files">
								{#each g.items as n (n.id)}{@render fileCard(n)}{/each}
							</div>
						{/each}
					{:else if files.length}
						<h3 class="sec">{t('file_browser.fichiers')}</h3>
						<div class="grid files">
							{#each files as n (n.id)}{@render fileCard(n)}{/each}
						</div>
					{/if}
				</div>
			{/if}

			{#if error}
				<EmptyState title={t('file_browser.chargement_impossible')} text={error} art="search" />
			{:else if loaded && !items.length}
				<EmptyState title={emptyMessage.title} text={emptyMessage.text} art={emptyMessage.art} />
			{/if}
			{#if loading && !loaded}
				<div class="loading" role="status"><span class="spinner"></span> {t('file_browser.chargement')}</div>
			{/if}
			<div bind:this={sentinel} class="sentinel"></div>
			{#if loading && loaded}<div class="loading small" role="status"><span class="spinner"></span></div>{/if}
		</div>
	</div>

	{#if ui.details}
		<DetailsPanel node={selectedItems.length === 1 ? selectedItems[0] : mode === 'drive' && !selectedItems.length ? folder : null} count={selectedItems.length} onclose={() => (ui.details = false)} onchanged={patchItem} />
	{/if}

	{#if fileDrag}
		<div class="dropveil"><div><Icon name="cloud_upload" size={48} /><p>{t('file_browser.drop_hint', { folder: parent ? (crumbs.at(-1)?.name ?? '…') : t('advanced_search.mon_drive') })}</p></div></div>
	{/if}
</section>

{#snippet fileCard(n: NodeItem)}
	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<div
		class="card file"
		class:sel={selected.has(n.id)}
		role="row"
		tabindex="0"
		draggable={canEdit(n) && mode !== 'trash'}
		onclick={(e) => select(e, n)}
		onkeydown={(e) => rowKey(e, n)}
		ondblclick={() => open(n)}
		oncontextmenu={(e) => onContext(e, n)}
							onpointerdown={(e) => pressStart(e, n)}
							onpointerup={pressEnd}
							onpointermove={pressEnd}
							onpointercancel={pressEnd}
		ondragstart={(e) => dragStart(e, n)}
	>
		<div class="cardhead">
			<Thumb node={n} size={20} />
			<span class="name ellipsis" title={n.name}>{n.name}</span>
			<button class="icon-btn small" aria-label={t('file_browser.plus_dactions')} onclick={(e) => kebab(e, n)}><Icon name="more_vert" size={20} /></button>
		</div>
		<div class="cardbody"><Thumb node={n} size={64} cover /></div>
	</div>
{/snippet}

{#if menu}<Menu x={menu.x} y={menu.y} items={menu.items} onclose={() => (menu = null)} />{/if}
{#if sortMenu}<Menu x={sortMenu.x} y={sortMenu.y} items={sortMenu.items} onclose={() => (sortMenu = null)} />{/if}
{#if headMenu}<Menu x={headMenu.x} y={headMenu.y} items={headMenu.items} onclose={() => (headMenu = null)} />{/if}

<style>
	.panel {
		position: relative;
		flex: 1;
		min-width: 0;
		display: flex;
		background: var(--surface);
		border-radius: var(--radius-panel);
		overflow: hidden;
		outline: none;
	}
	.panel.dropping {
		outline: 2px dashed var(--accent);
		outline-offset: -3px;
	}
	.main {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}
	.head {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 12px 16px 8px 24px;
		min-height: 64px;
	}
	.title {
		flex: 1;
		min-width: 0;
	}
	h1 {
		font-size: 24px;
		line-height: 32px;
	}
	.crumbs {
		display: flex;
		align-items: center;
		min-width: 0;
		font-family: var(--font);
		font-size: 24px;
		line-height: 32px;
	}
	.crumb {
		color: var(--text-2);
		padding: 6px 10px;
		border-radius: 18px;
		border: 0;
		background: transparent;
		font: inherit;
		white-space: nowrap;
	}
	a.crumb:hover,
	.crumb.last:hover {
		background: var(--hover-bg);
		text-decoration: none;
	}
	.crumb.last {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		color: var(--text);
		min-width: 0;
		padding-right: 4px;
	}
	:global(.sep) {
		color: var(--text-2);
	}
	.tools {
		display: flex;
		align-items: center;
		gap: 12px;
	}
	.icon-btn.on {
		background: var(--accent-soft);
		color: var(--accent-soft-text);
	}
	.seg {
		display: flex;
		border: 1px solid var(--line-strong);
		border-radius: var(--radius-pill);
		overflow: hidden;
		height: 40px;
	}
	.segbtn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 4px;
		width: 52px;
		border: 0;
		background: transparent;
		color: var(--text-2);
	}
	.segbtn + .segbtn {
		border-left: 1px solid var(--line-strong);
	}
	.segbtn.on {
		background: var(--accent-soft);
		color: var(--accent-soft-text);
		width: 68px;
	}
	.banner {
		display: flex;
		align-items: center;
		gap: 12px;
		margin: 0 24px 8px;
		padding: 12px 16px;
		border-radius: 12px;
		background: var(--surface-2);
		color: var(--text-2);
	}
	.selbar {
		display: flex;
		align-items: center;
		gap: 4px;
		margin: 0 16px 8px;
		padding: 4px 8px;
		min-height: 48px;
		border-radius: 24px;
		background: var(--accent-wash);
		color: var(--accent-soft-text);
	}
	.selbar .count {
		margin: 0 16px 0 4px;
		font-weight: 500;
	}
	.scroller {
		flex: 1;
		overflow: auto;
		padding-bottom: 24px;
	}

	/* list */
	.row {
		display: grid;
		grid-template-columns: minmax(220px, 1fr) 160px 180px 120px 112px;
		align-items: center;
		min-height: 48px;
		padding: 0 8px 0 16px;
		border-bottom: 1px solid var(--line);
		color: var(--text);
		cursor: default;
		user-select: none;
		outline: none;
	}
	.head-row {
		position: sticky;
		top: 0;
		z-index: 2;
		background: var(--surface);
		min-height: 48px;
		border-bottom: 1px solid var(--line);
		color: var(--text-2);
		font-weight: 500;
	}
	.row:not(.head-row):hover {
		background: var(--surface-2);
	}
	.row.sel {
		background: var(--select-bg) !important;
	}
	.row.drop,
	.card.drop {
		outline: 2px solid var(--accent);
		outline-offset: -2px;
		background: var(--accent-wash) !important;
	}
	.row:focus-visible {
		box-shadow: inset 0 0 0 2px var(--accent);
	}
	.c-name {
		display: flex;
		align-items: center;
		gap: 12px;
		min-width: 0;
	}
	.c-owner,
	.c-date,
	.c-size {
		padding-right: 16px;
		color: var(--text-2);
	}
	.c-owner {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}
	.c-act {
		display: flex;
		justify-content: flex-end;
		gap: 0;
	}
	.hov {
		opacity: 0;
	}
	.row:hover .hov,
	.row:focus-within .hov,
	.row.sel .hov {
		opacity: 1;
	}
	.th {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		border: 0;
		background: transparent;
		padding: 0;
		font: inherit;
		color: inherit;
		font-weight: 500;
		cursor: default;
		white-space: nowrap;
	}
	.th.sortable {
		cursor: pointer;
		padding: 6px 8px;
		margin-left: -8px;
		border-radius: 16px;
	}
	.th.sortable:hover {
		background: var(--hover-bg);
	}
	.nmcol {
		flex: 1;
		min-width: 0;
		display: flex;
	}
	.name {
		flex: 1;
		min-width: 0;
		font-weight: 500;
	}
	.sub,
	.msort {
		display: none;
	}
	:global(.starred) {
		color: var(--icon-gray);
	}
	.iconslot {
		position: relative;
		width: 24px;
		height: 24px;
		flex: none;
		display: inline-flex;
	}
	.chk {
		width: 18px;
		height: 18px;
		margin: 0;
		accent-color: var(--accent);
	}
	.iconslot .chk {
		position: absolute;
		inset: 3px;
		opacity: 0;
		z-index: 1;
	}
	.row:hover .iconslot .chk,
	.row.sel .iconslot .chk,
	.iconslot .chk:checked {
		opacity: 1;
	}
	.row:hover .typeico,
	.row.sel .typeico {
		opacity: 0;
	}
	.head-row .chk {
		margin-right: 6px;
	}
	.small {
		font-size: 12px;
	}

	/* grid */
	.grid-wrap {
		padding: 0 24px;
	}
	.grouprow {
		padding: 16px 16px 8px;
		font-family: var(--font);
		font-size: 14px;
		font-weight: 500;
		color: var(--text-2);
	}
	.sec {
		margin: 16px 0 8px;
		font-family: var(--font);
		font-size: 14px;
		font-weight: 500;
		color: var(--text-2);
	}
	.grid {
		display: grid;
		gap: 16px;
	}
	.grid.folders {
		grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
	}
	.grid.files {
		grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
	}
	.card {
		border: 1px solid var(--line);
		border-radius: 12px;
		background: var(--card);
		user-select: none;
		outline: none;
		cursor: default;
	}
	.card:hover {
		background: var(--card-hover);
	}
	.card.sel {
		background: var(--select-bg);
		border-color: transparent;
	}
	.card.folder {
		display: flex;
		align-items: center;
		gap: 12px;
		height: 48px;
		padding: 0 4px 0 16px;
	}
	.card.file {
		overflow: hidden;
	}
	.cardhead {
		display: flex;
		align-items: center;
		gap: 12px;
		height: 48px;
		padding: 0 4px 0 16px;
	}
	.cardbody {
		height: 164px;
		margin: 0 8px 8px;
		border-radius: 8px;
		overflow: hidden;
		background: var(--media-bg);
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.card.sel .cardbody {
		background: var(--surface);
	}

	.loading {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 12px;
		padding: 48px;
		color: var(--text-2);
	}
	.loading.small {
		padding: 16px;
	}
	.spinner {
		width: 20px;
		height: 20px;
		border: 3px solid var(--accent-soft);
		border-top-color: var(--accent);
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
	.sentinel {
		height: 1px;
	}
	.dropveil {
		position: absolute;
		inset: 0;
		z-index: 30;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--dropveil);
		color: var(--accent-soft-text);
		text-align: center;
		pointer-events: none;
		font-size: 18px;
	}

	@media (max-width: 1200px) {
		.row {
			grid-template-columns: minmax(180px, 1fr) 150px 112px 96px;
		}
		.c-owner {
			display: none;
		}
	}
	@media (max-width: 900px) {
		.panel {
			border-radius: 0;
		}
		.head {
			padding: 8px 16px 0;
			min-height: 48px;
		}
		.tools .icon-btn,
		.seg {
			display: none;
		}
		.crumbs,
		h1 {
			font-size: 20px;
			line-height: 28px;
		}
		.crumb {
			padding: 4px 6px;
		}
		.banner,
		.selbar {
			margin: 0 8px 8px;
		}
		.msort {
			display: flex;
			align-items: center;
			justify-content: space-between;
			padding: 4px 16px 4px 16px;
		}
		.chip {
			display: inline-flex;
			align-items: center;
			gap: 8px;
			height: 32px;
			padding: 0 8px 0 12px;
			border: 0;
			border-radius: 16px;
			background: transparent;
			color: var(--text);
			font-weight: 500;
		}
		.chip:not(:disabled) :global(svg) {
			padding: 3px;
			border-radius: 50%;
			background: var(--accent-soft);
			color: var(--accent-soft-text);
			width: 24px;
			height: 24px;
			box-sizing: border-box;
		}
		.head-row,
		.c-size,
		.c-owner,
		.c-date,
		.hov,
		.iconslot .chk {
			display: none;
		}
		.row {
			grid-template-columns: minmax(0, 1fr) 40px;
			min-height: 72px;
			padding: 0 8px 0 16px;
			border-bottom: 0;
		}
		.row:hover .typeico,
		.row.sel .typeico {
			opacity: 1;
		}
		.row:not(.head-row):hover {
			background: transparent;
		}
		.row.sel {
			background: var(--select-bg) !important;
		}
		.c-name {
			gap: 16px;
		}
		.nmcol {
			flex-direction: column;
			gap: 2px;
		}
		.name {
			flex: none;
			font-weight: 400;
			font-size: 16px;
			line-height: 24px;
		}
		.sub {
			display: block;
			color: var(--text-2);
			font-size: 14px;
			line-height: 20px;
		}
		.grid-wrap {
			padding: 0 16px;
		}
		.grid.folders,
		.grid.files {
			grid-template-columns: repeat(2, minmax(0, 1fr));
			gap: 12px;
		}
		.cardbody {
			height: 120px;
		}
	}
</style>
