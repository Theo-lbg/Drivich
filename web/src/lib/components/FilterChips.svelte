<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import Icon from './Icon.svelte';
	import Menu, { type MenuItem } from './Menu.svelte';

	import type { Filters } from '$lib/types';

	let {
		filters,
		onchange,
		showLocation = false,
		showOwner = true,
		locations = null
	}: {
		filters: Filters;
		onchange: (patch: Partial<Filters>) => void;
		showLocation?: boolean;
		showOwner?: boolean;
		/** Restrict the Source menu (value → label); defaults to every location. */
		locations?: [string, string][] | null;
	} = $props();

	let menu = $state<{ x: number; y: number; items: MenuItem[] } | null>(null);

	const TYPES: [string, string, string][] = $derived([
		['', t('filter_chips.tous_les_types'), 'filter_list'],
		['folder', t('advanced_search.dossiers'), 'folder'],
		['image', t('advanced_search.images'), 'image'],
		['video', t('advanced_search.videos'), 'movie'],
		['audio', t('advanced_search.audio'), 'audio_file'],
		['pdf', 'PDF', 'picture_as_pdf'],
		['document', t('advanced_search.documents'), 'description'],
		['text', t('advanced_search.textes_et_code'), 'text_snippet'],
		['archive', t('advanced_search.archives'), 'folder_zip']
	]);
	const DATES: [string, string][] = $derived([
		['', t('advanced_search.nimporte_quand')],
		['today', t('advanced_search.aujourdhui')],
		['7d', '7 derniers jours'],
		['30d', '30 derniers jours'],
		['year', t('advanced_search.cette_annee')]
	]);
	const OWNERS: [string, string][] = $derived([
		['', t('filter_chips.tout_le_monde')],
		['me', t('advanced_search.moi')],
		['others', t('advanced_search.autres_personnes')]
	]);
	const ALL_LOCATIONS: [string, string][] = $derived([
		['', t('advanced_search.partout')],
		['drive', t('advanced_search.mon_drive')],
		['shared', t('advanced_search.partages_avec_moi')],
		['starred', t('advanced_search.favoris')],
		['trash', t('advanced_search.corbeille')]
	]);

	const LOCATIONS = $derived(locations ?? ALL_LOCATIONS);

	function open(e: MouseEvent, key: keyof Filters, options: [string, string, string?][]) {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		menu = {
			x: r.left,
			y: r.bottom + 4,
			items: options.map(([value, label, icon]) => ({ label, icon, checked: filters[key] === value, run: () => onchange({ [key]: value }) }))
		};
	}

	const label = (opts: [string, string, string?][], v: string, fallback: string) => (v ? (opts.find((o) => o[0] === v)?.[1] ?? fallback) : fallback);
</script>

<div class="chips" role="toolbar" aria-label={t('filter_chips.filtres')}>
	<button class="chip" class:active={!!filters.type} onclick={(e) => open(e, 'type', TYPES)}>
		{label(TYPES, filters.type, t('advanced_search.type'))}<Icon name="arrow_drop_down" size={20} />
	</button>
	{#if showOwner}
		<button class="chip" class:active={!!filters.owner} onclick={(e) => open(e, 'owner', OWNERS)}>
			{filters.owner ? `Contacts : ${label(OWNERS, filters.owner, '')}` : t('advanced_search.contacts')}<Icon name="arrow_drop_down" size={20} />
		</button>
	{/if}
	<button class="chip" class:active={!!filters.date} onclick={(e) => open(e, 'date', DATES)}>
		{filters.date ? label(DATES, filters.date, '') : t('advanced_search.date_de_modification')}<Icon name="arrow_drop_down" size={20} />
	</button>
	{#if showLocation}
		<button class="chip" class:active={!!filters.location} onclick={(e) => open(e, 'location', LOCATIONS)}>
			{filters.location ? `Source : ${label(LOCATIONS, filters.location, '')}` : t('filter_chips.source')}<Icon name="arrow_drop_down" size={20} />
		</button>
	{/if}
	{#if filters.type || filters.date || filters.owner || filters.location}
		<button class="chip clear" onclick={() => onchange({ type: '', date: '', owner: '', location: '' })}><Icon name="close" size={18} />{t('filter_chips.effacer_les_filtres')}</button>
	{/if}
</div>

{#if menu}<Menu x={menu.x} y={menu.y} items={menu.items} onclose={() => (menu = null)} />{/if}

<style>
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
		padding: 0 24px 8px;
	}
	.chip {
		padding-right: 6px;
	}
	.clear {
		border-color: transparent;
		color: var(--accent);
		padding-right: 12px;
	}
	@media (max-width: 900px) {
		/* One scrolling row on phones, like Drive's filter chips. */
		.chips {
			flex-wrap: nowrap;
			overflow-x: auto;
			padding: 0 16px 8px;
			scrollbar-width: none;
		}
		.chip {
			flex: none;
			white-space: nowrap;
		}
	}
</style>
