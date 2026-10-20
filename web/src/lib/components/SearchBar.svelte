<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { api } from '$lib/api';
	import { folderHref } from '$lib/actions';
	import { openViewer } from '$lib/state/app.svelte';
	import { CLASS_COLOR, CLASS_ICON, classify, isPreviewable } from '$lib/format';
	import type { Suggestion } from '$lib/types';
	import Icon from './Icon.svelte';
	import AdvancedSearch from './AdvancedSearch.svelte';

	let { onmenu }: { onmenu?: () => void } = $props();

	let q = $state('');
	let focused = $state(false);
	let suggestions = $state<Suggestion[]>([]);
	let active = $state(-1);
	let advanced = $state(false);
	let input: HTMLInputElement | undefined = $state();
	let box: HTMLDivElement | undefined = $state();
	let timer: ReturnType<typeof setTimeout> | undefined;
	let controller: AbortController | undefined;

	// Keep the box in sync with the current search page.
	$effect(() => {
		if (page.url.pathname === '/search') q = page.url.searchParams.get('q') ?? '';
	});

	function onInput() {
		active = -1;
		clearTimeout(timer);
		controller?.abort();
		const value = q.trim();
		if (!value) {
			suggestions = [];
			return;
		}
		timer = setTimeout(async () => {
			controller = new AbortController();
			try {
				suggestions = (await api.suggest(value, controller.signal)).items;
			} catch {
				/* aborted or transient */
			}
		}, 150);
	}

	function submit() {
		const v = q.trim();
		if (!v) return;
		focused = false;
		input?.blur();
		goto(`/search?q=${encodeURIComponent(v)}`);
	}

	async function pick(s: Suggestion) {
		focused = false;
		input?.blur();
		if (s.kind === 'folder') return goto(folderHref(s.id));
		try {
			const n = await api.node(s.id);
			if (isPreviewable(n)) {
				openViewer([n], 0);
				api.opened(n.id).catch(() => {});
			} else if (n.parent_id) goto(folderHref(n.parent_id));
			else goto('/drive');
		} catch {
			goto('/drive');
		}
	}

	function onkeydown(e: KeyboardEvent) {
		if (e.key === 'ArrowDown') {
			e.preventDefault();
			active = Math.min(active + 1, suggestions.length - 1);
		} else if (e.key === 'ArrowUp') {
			e.preventDefault();
			active = Math.max(active - 1, -1);
		} else if (e.key === 'Enter') {
			e.preventDefault();
			if (active >= 0 && suggestions[active]) pick(suggestions[active]);
			else submit();
		} else if (e.key === 'Escape') {
			focused = false;
			input?.blur();
		}
	}

	function outside(e: MouseEvent) {
		if (box && !box.contains(e.target as Node)) {
			focused = false;
			advanced = false;
		}
	}
</script>

<svelte:window onmousedown={outside} />

<div class="search" class:focus={focused || advanced} bind:this={box} role="search">
	<button class="icon-btn menu" aria-label={t('search_bar.menu_principal')} onclick={onmenu}><Icon name="menu" size={24} /></button>
	<button class="icon-btn lens" aria-label={t('advanced_search.rechercher')} onclick={submit}><Icon name="search" size={24} /></button>
	<input
		bind:this={input}
		bind:value={q}
		type="search"
		placeholder={t('search_bar.rechercher_dans_drivich')}
		aria-label={t('search_bar.rechercher_dans_drivich')}
		autocomplete="off"
		spellcheck="false"
		oninput={onInput}
		onfocus={() => (focused = true)}
		{onkeydown}
	/>
	{#if q}
		<button class="icon-btn clear" aria-label={t('search_bar.effacer_la_recherche')} onclick={() => { q = ''; suggestions = []; input?.focus(); }}><Icon name="close" size={22} /></button>
	{/if}
	<button class="icon-btn" aria-label={t('search_bar.options_de_recherche')} aria-expanded={advanced} onclick={() => (advanced = !advanced)}><Icon name="tune" size={24} /></button>

	{#if focused && !advanced && q.trim() && suggestions.length}
		<ul class="drop" role="listbox">
			{#each suggestions as s, i (s.id)}
				<li role="option" aria-selected={i === active}>
					<button class:act={i === active} onmouseenter={() => (active = i)} onclick={() => pick(s)}>
						<span style="color:{CLASS_COLOR[classify(s)]}"><Icon name={CLASS_ICON[classify(s)]} size={22} fill={s.kind === 'folder'} /></span>
						<span class="n ellipsis">{s.name}</span>
						<span class="p ellipsis">{s.parent_name ?? t('advanced_search.mon_drive')}</span>
					</button>
				</li>
			{/each}
			<li class="all"><button onclick={submit}><Icon name="search" size={20} /> {t('search_bar.all', { q: q.trim() })}</button></li>
		</ul>
	{/if}

	{#if advanced}
		<AdvancedSearch initial={q} onclose={() => (advanced = false)} />
	{/if}
</div>

<style>
	.search {
		position: relative;
		display: flex;
		align-items: center;
		gap: 2px;
		flex: 1;
		max-width: 720px;
		height: 48px;
		padding: 0 8px;
		border-radius: 24px;
		background: var(--search-bg);
		transition: background 0.1s, box-shadow 0.1s;
	}
	.search.focus {
		background: var(--elevated);
		box-shadow: var(--shadow-1);
	}
	.menu {
		display: none;
	}
	input {
		flex: 1;
		min-width: 0;
		height: 100%;
		border: 0;
		outline: 0;
		background: transparent;
		font-size: 16px;
		padding: 0 8px;
	}
	input::-webkit-search-cancel-button {
		display: none;
	}
	.drop {
		position: absolute;
		left: 0;
		right: 0;
		top: 52px;
		z-index: 60;
		margin: 0;
		padding: 8px 0;
		list-style: none;
		background: var(--elevated);
		border-radius: 16px;
		box-shadow: var(--shadow-2);
	}
	.drop button {
		display: flex;
		align-items: center;
		gap: 16px;
		width: 100%;
		height: 44px;
		padding: 0 20px;
		border: 0;
		background: transparent;
		text-align: left;
	}
	.drop button.act,
	.drop button:hover {
		background: var(--surface-2);
	}
	.n {
		flex: 1;
		font-weight: 500;
	}
	.p {
		max-width: 40%;
		color: var(--text-2);
		font-size: 12px;
	}
	.all {
		border-top: 1px solid var(--line);
		margin-top: 4px;
		padding-top: 4px;
	}
	.all button {
		color: var(--accent);
	}
	@media (max-width: 900px) {
		.menu {
			display: inline-flex;
		}
		.lens {
			display: none;
		}
	}
</style>
