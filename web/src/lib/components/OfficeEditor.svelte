<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import { api } from '$lib/api';
	import type { NodeItem } from '$lib/types';

	let { node, onfail }: { node: NodeItem; onfail: () => void } = $props();
	let host = $state<HTMLDivElement>();
	let loading = $state(true);

	type DocEditor = { destroyEditor?: () => void };

	const scripts = new Map<string, Promise<void>>();
	function loadScript(src: string): Promise<void> {
		let p = scripts.get(src);
		if (!p) {
			p = new Promise<void>((resolve, reject) => {
				const s = document.createElement('script');
				s.src = src;
				s.onload = () => resolve();
				s.onerror = () => reject(new Error('editor script'));
				document.head.appendChild(s);
			});
			p.catch(() => scripts.delete(src));
			scripts.set(src, p);
		}
		return p;
	}

	$effect(() => {
		const el = host;
		const id = node.id;
		if (!el) return;
		let editor: DocEditor | undefined;
		let dead = false;
		loading = true;
		(async () => {
			const r = await api.officeConfig(id);
			await loadScript(`${r.server}/web-apps/apps/api/documents/api.js`);
			if (dead || !window.DocsAPI) return;
			// The editor replaces the element it is given with its own iframe.
			const slot = document.createElement('div');
			slot.id = `office-${id}`;
			el.replaceChildren(slot);
			editor = new window.DocsAPI.DocEditor(slot.id, r.config);
			loading = false;
		})().catch(() => {
			if (!dead) onfail();
		});
		return () => {
			dead = true;
			// Closing the editor lets the Document Server save the document right away.
			editor?.destroyEditor?.();
			el.replaceChildren();
		};
	});
</script>

<div class="office" bind:this={host}></div>
{#if loading}<p class="load">{t('file_browser.chargement')}</p>{/if}

<style>
	.office {
		width: 100%;
		height: 100%;
		background: #fff;
	}
	.load {
		position: absolute;
		inset: 0;
		display: grid;
		place-items: center;
		margin: 0;
		color: var(--text-2);
		pointer-events: none;
	}
</style>
