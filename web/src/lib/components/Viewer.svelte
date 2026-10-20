<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import { api, urls } from '$lib/api';
	import { askShare, downloadNode } from '$lib/actions';
	import { classify, CLASS_COLOR, CLASS_ICON, formatBytes, isOffice } from '$lib/format';
	import { viewer } from '$lib/state/app.svelte';
	import { session } from '$lib/state/session.svelte';
	import OfficeEditor from './OfficeEditor.svelte';
	import Comments from './Comments.svelte';
	import Icon from './Icon.svelte';

	const node = $derived(viewer.items[viewer.index]);
	const kind = $derived(node ? classify(node) : 'file');
	let text = $state<string | null>(null);
	let textError = $state(false);
	let zoom = $state(1);
	let imgFailed = $state(false);
	let uiHidden = $state(false);
	let showComments = $state(false);
	/** State of the PDF rendition of an Office document (made by the server on first view). */
	let officeState = $state<'pending' | 'ready' | 'failed' | 'unsupported'>('pending');
	const office = $derived(!!node && !viewer.token && isOffice(node));
	/** Opened in the ONLYOFFICE editor when the server has one, else shown as a PDF. */
	let editorFailed = $state(false);
	const editing = $derived(office && session.office && !editorFailed);
	const officeCapable = $derived(!!node && isOffice(node));

	const src = $derived(node ? (viewer.token ? urls.publicContent(viewer.token, node.id) : urls.content(node.id)) : '');
	const downloadHref = $derived(node ? (viewer.token ? urls.publicDownload(viewer.token, node.id) : urls.download(node.id)) : '');

	$effect(() => {
		const n = node;
		text = null;
		textError = false;
		zoom = 1;
		imgFailed = false;
		editorFailed = false;
		if (!n) return;
		if ((kind === 'text' || kind === 'code') && n.size <= 2 * 1024 * 1024) {
			const ctl = new AbortController();
			fetch(src, { credentials: 'same-origin', signal: ctl.signal })
				.then((r) => (r.ok ? r.text() : Promise.reject()))
				.then((body) => (text = body))
				.catch(() => (textError = true));
			return () => ctl.abort();
		}
	});

	// PDF rendition of an Office document, when it is not opened in the editor.
	$effect(() => {
		const n = node;
		if (!n || viewer.token || !isOffice(n) || (session.office && !editorFailed)) return;
		officeState = 'pending';
		let stop = false;
		(async () => {
			while (!stop) {
				try {
					const r = await api.requestPreview(n.id);
					officeState = r.state === 'none' ? 'pending' : r.state;
					if (r.state !== 'pending' && r.state !== 'none') return;
				} catch {
					officeState = 'failed';
					return;
				}
				await new Promise((res) => setTimeout(res, 2000));
			}
		})();
		return () => (stop = true);
	});

	function close() {
		viewer.open = false;
	}
	function go(d: number) {
		const i = viewer.index + d;
		if (i >= 0 && i < viewer.items.length) viewer.index = i;
	}

	function onkeydown(e: KeyboardEvent) {
		if (!viewer.open) return;
		if (e.key === 'Escape') close();
		else if (e.key === 'ArrowRight') go(1);
		else if (e.key === 'ArrowLeft') go(-1);
		else if ((e.key === '+' || e.key === '=') && kind === 'image') zoom = Math.min(5, zoom + 0.25);
		else if (e.key === '-' && kind === 'image') zoom = Math.max(0.25, zoom - 0.25);
	}

	function download() {
		if (viewer.token) {
			const a = document.createElement('a');
			a.href = downloadHref;
			a.download = node.name;
			a.click();
		} else downloadNode(node);
	}
</script>

<svelte:window {onkeydown} />

{#if viewer.open && node}
	<div class="viewer" role="dialog" aria-modal="true" aria-label={t('viewer.preview_of', { name: node.name })}>
		<header class:hide={uiHidden}>
			<button class="icon-btn" aria-label={t('viewer.fermer_lapercu')} onclick={close}><Icon name="arrow_back" size={24} /></button>
			<span class="ico" style="color:{CLASS_COLOR[kind]}"><Icon name={CLASS_ICON[kind]} size={22} /></span>
			<h1 class="ellipsis">{node.name}</h1>
			<span class="count">{viewer.items.length > 1 ? `${viewer.index + 1} / ${viewer.items.length}` : ''}</span>
			<div class="acts">
				{#if kind === 'image'}
					<button class="icon-btn" aria-label={t('viewer.zoom_arriere')} onclick={() => (zoom = Math.max(0.25, zoom - 0.25))}><Icon name="zoom_out" size={24} /></button>
					<button class="icon-btn" aria-label={t('viewer.zoom_avant')} onclick={() => (zoom = Math.min(5, zoom + 0.25))}><Icon name="zoom_in" size={24} /></button>
				{/if}
				{#if !viewer.token}
					<button class="icon-btn" class:on={showComments} aria-pressed={showComments} aria-label={t('comments.title')} title={t('comments.title')} onclick={() => (showComments = !showComments)}><Icon name="comment" size={24} /></button>
				{/if}
				{#if !viewer.token && node.role === 'owner'}
					<button class="icon-btn" aria-label={t('file_browser.partager')} onclick={() => { close(); askShare(node); }}><Icon name="person_add" size={24} /></button>
				{/if}
				<button class="icon-btn" aria-label={t('file_browser.telecharger')} onclick={download}><Icon name="download" size={24} /></button>
			</div>
		</header>

		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div class="stage" onclick={(e) => { if (e.target === e.currentTarget) close(); }}>
			{#if kind === 'image' && !imgFailed}
				{#key node.id}
					<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
					<img src={src} alt={node.name} style="transform:scale({zoom})" onerror={() => (imgFailed = true)} onclick={() => (uiHidden = !uiHidden)} />
				{/key}
			{:else if kind === 'video'}
				{#key node.id}<!-- svelte-ignore a11y_media_has_caption --><video src={src} controls autoplay playsinline></video>{/key}
			{:else if kind === 'audio'}
				{#key node.id}
					<div class="audio"><Icon name="audio_file" size={96} /><p>{node.name}</p><audio src={src} controls autoplay></audio></div>
				{/key}
			{:else if kind === 'pdf'}
				{#key node.id}<iframe title={node.name} src={src}></iframe>{/key}
			{:else if editing}
				{#key node.id}<OfficeEditor {node} onfail={() => { editorFailed = true; }} />{/key}
			{:else if office && officeState === 'ready'}
				{#key node.id}<iframe title={node.name} src={urls.preview(node.id)}></iframe>{/key}
			{:else if office && officeState === 'pending'}
				<div class="none"><Icon name={CLASS_ICON[kind]} size={96} /><h2>{t('viewer.office_preparing')}</h2><p>{node.name}</p></div>
			{:else if (kind === 'text' || kind === 'code') && text !== null}
				<pre class="text">{text}</pre>
			{:else if (kind === 'text' || kind === 'code') && !textError && node.size <= 2 * 1024 * 1024}
				<div class="none">{t('file_browser.chargement')}</div>
			{:else}
				<div class="none">
					<Icon name={CLASS_ICON[kind]} size={96} />
					<h2>{t('viewer.apercu_non_disponible')}</h2>
					<p>{node.name} · {formatBytes(node.size)}</p>
					{#if officeCapable && !viewer.token}<p class="why">{t('viewer.office_unavailable')}</p>{/if}
					<button class="btn filled" onclick={download}><Icon name="download" size={20} />{t('file_browser.telecharger')}</button>
				</div>
			{/if}

			{#if showComments && !viewer.token}
				<aside class="side" aria-label={t('comments.title')}><Comments {node} /></aside>
			{/if}

			{#if viewer.index > 0}
				<button class="nav prev" aria-label={t('viewer.fichier_precedent')} onclick={() => go(-1)}><Icon name="keyboard_arrow_left" size={32} /></button>
			{/if}
			{#if viewer.index < viewer.items.length - 1}
				<button class="nav next" aria-label={t('viewer.fichier_suivant')} onclick={() => go(1)}><Icon name="keyboard_arrow_right" size={32} /></button>
			{/if}
		</div>
	</div>
{/if}

<style>
	.viewer {
		position: fixed;
		inset: 0;
		z-index: 950;
		display: flex;
		flex-direction: column;
		background: rgba(32, 33, 36, 0.96);
		color: #e8eaed;
	}
	header {
		display: flex;
		align-items: center;
		gap: 12px;
		height: 64px;
		padding: 0 12px;
		background: linear-gradient(rgba(0, 0, 0, 0.55), transparent);
		z-index: 2;
		transition: opacity 0.15s;
	}
	header.hide {
		opacity: 0;
		pointer-events: none;
	}
	header :global(.icon-btn) {
		color: #e8eaed;
	}
	header :global(.icon-btn:hover) {
		background: rgba(255, 255, 255, 0.12);
	}
	h1 {
		flex: 1;
		font-size: 18px;
		font-weight: 500;
		color: #fff;
	}
	.count {
		color: #bdc1c6;
		font-size: 13px;
	}
	.acts {
		display: flex;
	}
	.ico {
		display: inline-flex;
		background: #fff;
		border-radius: 4px;
		padding: 2px;
	}
	.stage {
		position: relative;
		flex: 1;
		min-height: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		margin-top: -64px;
		padding-top: 64px;
	}
	img,
	video {
		max-width: 100%;
		max-height: 100%;
		object-fit: contain;
		transition: transform 0.1s;
	}
	iframe {
		width: min(1000px, 100%);
		height: 100%;
		border: 0;
		background: #fff;
	}
	.text {
		width: min(1000px, calc(100% - 32px));
		height: calc(100% - 24px);
		margin: 0;
		padding: 16px 24px;
		overflow: auto;
		background: #fff;
		color: #1f1f1f;
		border-radius: 8px;
		font: 13px/1.5 ui-monospace, 'SF Mono', Menlo, Consolas, monospace;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		tab-size: 4;
	}
	.audio,
	.none {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 12px;
		text-align: center;
		color: #bdc1c6;
	}
	.none h2 {
		color: #fff;
		font-size: 22px;
	}
	.none p,
	.audio p {
		margin: 0;
	}
	.side {
		position: absolute;
		right: 0;
		top: 64px;
		bottom: 0;
		width: min(380px, 100%);
		overflow: auto;
		padding: 0 20px 20px;
		background: var(--surface);
		color: var(--text);
		z-index: 3;
		box-shadow: var(--shadow-2);
	}
	.side :global(.comments) {
		border-top: 0;
	}
	.why {
		max-width: 460px;
		font-size: 13px;
	}
	header :global(.icon-btn.on) {
		background: rgba(255, 255, 255, 0.2);
	}
	.nav {
		position: absolute;
		top: 50%;
		transform: translateY(-50%);
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 56px;
		height: 56px;
		border: 0;
		border-radius: 50%;
		background: rgba(255, 255, 255, 0.12);
		color: #fff;
	}
	.nav:hover {
		background: rgba(255, 255, 255, 0.24);
	}
	.prev {
		left: 16px;
	}
	.next {
		right: 16px;
	}
</style>
