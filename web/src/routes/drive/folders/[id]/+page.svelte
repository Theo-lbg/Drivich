<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { page } from '$app/state';
	import FileBrowser from '$lib/components/FileBrowser.svelte';
</script>

<svelte:head><title>Dossier – Drivich</title></svelte:head>
<FileBrowser mode="drive" parent={page.params.id ?? null} title="Dossier" />
