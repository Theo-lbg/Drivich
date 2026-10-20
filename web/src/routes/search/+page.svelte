<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import type { SearchOptions } from '$lib/api';
	import FileBrowser from '$lib/components/FileBrowser.svelte';
	import type { Filters } from '$lib/types';

	const sp = $derived(page.url.searchParams);
	const filters = $derived<Filters>({ type: sp.get('type') ?? '', date: sp.get('date') ?? '', owner: sp.get('owner') ?? '', location: sp.get('location') ?? '' });

	function from(d: string): string | undefined {
		const now = new Date();
		if (d === 'today') return new Date(now.getFullYear(), now.getMonth(), now.getDate()).toISOString();
		if (d === '7d') return new Date(now.getTime() - 7 * 86_400_000).toISOString();
		if (d === '30d') return new Date(now.getTime() - 30 * 86_400_000).toISOString();
		if (d === 'year') return new Date(now.getFullYear(), 0, 1).toISOString();
		return undefined;
	}

	const search = $derived<SearchOptions>({
		q: sp.get('q') ?? undefined,
		name: sp.get('name') ?? undefined,
		words: sp.get('words') ?? undefined,
		type: filters.type || undefined,
		owner: filters.owner || undefined,
		location: filters.location || undefined,
		modified_from: from(filters.date)
	});

	function change(patch: Partial<Filters>) {
		const u = new URL(page.url.href);
		for (const [k, v] of Object.entries(patch)) (v ? u.searchParams.set(k, v) : u.searchParams.delete(k));
		goto(u.pathname + u.search, { replaceState: true });
	}

	const label = $derived(sp.get('q') ?? sp.get('words') ?? sp.get('name') ?? '');
</script>

<svelte:head><title>{label ? `${label} – ` : ''}{t('search_page.title')} – Drivich</title></svelte:head>
<FileBrowser mode="search" title={label ? t('search_page.results_for', { q: label }) : t('search.resultats_de_recherche')} {search} {filters} onfilters={change} />
