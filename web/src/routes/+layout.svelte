<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { i18n, initI18n, setLocale, t } from '$lib/i18n/index.svelte';
	import '../app.css';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import type { Snippet } from 'svelte';
	import { setUnauthorizedHandler } from '$lib/api';
	import AppRail from '$lib/components/AppRail.svelte';
	import Dialogs from '$lib/components/Dialogs.svelte';
	import ShareDialog from '$lib/components/ShareDialog.svelte';
	import Sidebar from '$lib/components/Sidebar.svelte';
	import Toasts from '$lib/components/Toasts.svelte';
	import Topbar from '$lib/components/Topbar.svelte';
	import UploadPanel from '$lib/components/UploadPanel.svelte';
	import Viewer from '$lib/components/Viewer.svelte';
	import { nav, pickers } from '$lib/state/app.svelte';
	import { loadSession, session } from '$lib/state/session.svelte';
	import { loadUi, ui } from '$lib/state/ui.svelte';
	import { enqueue, enqueueWithPaths } from '$lib/state/uploads.svelte';

	let { children }: { children: Snippet } = $props();

	const path = $derived(page.url.pathname);
	const bare = $derived(path === '/login' || path === '/setup' || path.startsWith('/s/'));

	let fileInput: HTMLInputElement | undefined = $state();
	let folderInput: HTMLInputElement | undefined = $state();

	onMount(() => {
		loadUi();
		initI18n().then(loadSession);
		setUnauthorizedHandler(() => {
			if (session.user) {
				session.user = null;
				if (!bare) goto('/login');
			}
		});
		pickers.files = () => fileInput?.click();
		pickers.folder = () => folderInput?.click();
		// Dropping a file outside the list must not make the browser navigate to it.
		const stop = (e: DragEvent) => {
			if (e.dataTransfer?.types.includes('Files')) e.preventDefault();
		};
		window.addEventListener('dragover', stop);
		window.addEventListener('drop', stop);
		return () => {
			window.removeEventListener('dragover', stop);
			window.removeEventListener('drop', stop);
		};
	});

	// The language chosen on the account wins once we know who is signed in.
	$effect(() => {
		const wanted = session.user?.locale;
		if (i18n.ready && wanted && wanted !== i18n.locale) setLocale(wanted);
	});

	// Route guard.
	$effect(() => {
		if (!session.ready) return;
		if (session.setupRequired) {
			if (path !== '/setup') goto('/setup', { replaceState: true });
		} else if (!session.user && !bare) {
			goto(`/login${path !== '/' ? `?next=${encodeURIComponent(path + page.url.search)}` : ''}`, { replaceState: true });
		} else if (path === '/setup') {
			// The installation is done: there is nothing to set up any more.
			goto(session.user ? '/' : '/login', { replaceState: true });
		} else if (session.user && path === '/login') {
			goto('/', { replaceState: true });
		}
	});

	$effect(() => {
		path;
		ui.drawer = false;
	});

	function target(): string | null {
		return nav.canWrite ? nav.folderId : null;
	}

	function onFiles(e: Event) {
		const input = e.currentTarget as HTMLInputElement;
		const list = Array.from(input.files ?? []);
		input.value = '';
		if (list.length) enqueue(list.map((file) => ({ file, parentId: target() })));
	}

	function onFolder(e: Event) {
		const input = e.currentTarget as HTMLInputElement;
		const list = Array.from(input.files ?? []);
		input.value = '';
		if (list.length) enqueueWithPaths(list.map((file) => ({ file, path: (file as File & { webkitRelativePath: string }).webkitRelativePath || file.name })), target());
	}
</script>

{#if !i18n.ready || !session.ready || (path === '/setup' && !session.setupRequired)}
	<!-- Loading, or /setup requested after the installation was completed: the guard below redirects. -->
	<div class="boot" role="status"><img src="/logo.png" alt="" width="64" /><span class="sr-only">{t('file_browser.chargement')}</span></div>
{:else if bare}
	{@render children()}
{:else if !session.user}
	<!-- Signed out: the route guard is redirecting to /login; render nothing meanwhile. -->
	<div class="boot" role="status"><img src="/logo.png" alt="" width="64" /><span class="sr-only">{t('layout_routes.redirection')}</span></div>
{:else}
	<div class="shell">
		<Topbar />
		<div class="body">
			<Sidebar />
			<main>{@render children()}</main>
			<AppRail />
		</div>
	</div>
	<input bind:this={fileInput} type="file" multiple hidden onchange={onFiles} />
	<input bind:this={folderInput} type="file" multiple hidden onchange={onFolder} {...{ webkitdirectory: true }} />
	<UploadPanel />
	<Dialogs />
	<ShareDialog />
	<Viewer />
{/if}
<Toasts />

<style>
	.boot {
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.shell {
		height: 100%;
		display: flex;
		flex-direction: column;
	}
	.body {
		flex: 1;
		display: flex;
		min-height: 0;
	}
	main {
		flex: 1;
		min-width: 0;
		display: flex;
		padding: 0 16px 16px 0;
	}
	@media (min-width: 1100px) {
		main {
			padding-right: 4px;
		}
	}
	@media (max-width: 900px) {
		main {
			padding: 0 8px 8px 8px;
		}
	}
</style>
