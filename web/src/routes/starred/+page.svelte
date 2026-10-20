<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import FileBrowser from '$lib/components/FileBrowser.svelte';
</script>

<svelte:head><title>{t('starred.favoris_drivich')}</title></svelte:head>
<FileBrowser mode="starred" title={t('advanced_search.favoris')} />
