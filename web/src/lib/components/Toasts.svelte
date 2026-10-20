<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (c) 2026 Theo-lbg -->
<script lang="ts">
	import { t } from '$lib/i18n/index.svelte';
	import { dismiss, toasts } from '$lib/state/toast.svelte';
</script>

<div class="stack" aria-live="polite">
	{#each toasts.list as toast (toast.id)}
		<div class="toast" class:error={toast.kind === 'error'} role="status">
			<span class="text">{toast.text}</span>
			{#if toast.action}
				<button class="act" onclick={() => { toast.action?.run(); dismiss(toast.id); }}>{toast.action.label}</button>
			{/if}
			<button class="x" aria-label={t('toasts.fermer')} onclick={() => dismiss(toast.id)}>✕</button>
		</div>
	{/each}
</div>

<style>
	.stack {
		position: fixed;
		left: 24px;
		bottom: 24px;
		z-index: 1200;
		display: flex;
		flex-direction: column;
		gap: 8px;
		max-width: calc(100vw - 48px);
	}
	.toast {
		display: flex;
		align-items: center;
		gap: 16px;
		min-height: 48px;
		padding: 8px 8px 8px 16px;
		border-radius: 4px;
		background: #2e3132;
		color: #f0f0f0;
		box-shadow: var(--shadow-2);
		animation: in 0.15s ease-out;
	}
	.toast.error {
		background: #601410;
	}
	.text {
		flex: 1;
	}
	.act {
		border: 0;
		background: transparent;
		color: #a8c7fa;
		font-weight: 500;
		padding: 8px;
		border-radius: 4px;
	}
	.act:hover {
		background: rgba(255, 255, 255, 0.1);
	}
	.x {
		border: 0;
		background: transparent;
		color: #c7c7c7;
		width: 32px;
		height: 32px;
		border-radius: 50%;
	}
	.x:hover {
		background: rgba(255, 255, 255, 0.1);
	}
	@keyframes in {
		from {
			opacity: 0;
			transform: translateY(8px);
		}
	}
</style>
