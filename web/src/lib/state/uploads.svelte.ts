// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

import { api, ApiError, uploadChunk } from '$lib/api';
import { errorMessage, t } from '$lib/i18n/index.svelte';
import { refreshAll } from './app.svelte';
import { refreshStorage } from './session.svelte';

/** 8 MiB per request: small enough to resume cheaply, large enough to stay fast. */
const CHUNK = 8 * 1024 * 1024;
const CONCURRENCY = 3;
const MAX_RETRIES = 6;

export type UploadStatus = 'queued' | 'uploading' | 'done' | 'error' | 'canceled';

export interface UploadJob {
	id: number;
	name: string;
	size: number;
	sent: number;
	status: UploadStatus;
	error?: string;
	nodeId?: string;
	parentId: string | null;
	file: File;
	uploadId?: string;
	/** Upload as a new version of this file instead of a new file. */
	replaceNodeId?: string;
	controller?: AbortController;
}

export const uploads = $state<{ jobs: UploadJob[]; collapsed: boolean; doneVersion: number }>({
	jobs: [],
	collapsed: false,
	doneVersion: 0
});

let seq = 0;
let running = 0;
let refreshTimer: ReturnType<typeof setTimeout> | undefined;

/** Reload the visible list once a burst of uploads has settled. */
function scheduleRefresh() {
	clearTimeout(refreshTimer);
	refreshTimer = setTimeout(refreshAll, 400);
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

export function enqueue(items: { file: File; parentId: string | null }[]) {
	for (const { file, parentId } of items) {
		uploads.jobs.push({ id: ++seq, name: file.name, size: file.size, sent: 0, status: 'queued', parentId, file });
	}
	uploads.collapsed = false;
	pump();
}

/** Upload `file` as a new version of an existing file. */
export function enqueueVersion(file: File, node: { id: string; name: string }) {
	uploads.jobs.push({ id: ++seq, name: node.name, size: file.size, sent: 0, status: 'queued', parentId: null, file, replaceNodeId: node.id });
	uploads.collapsed = false;
	pump();
}

/** Upload files, creating the folder structure of `relativePath` below `parentId`. */
export async function enqueueWithPaths(items: { file: File; path: string }[], parentId: string | null) {
	const cache = new Map<string, string | null>();
	cache.set('', parentId);
	const out: { file: File; parentId: string | null }[] = [];
	for (const { file, path } of items) {
		const dirs = path.split('/').slice(0, -1).filter(Boolean);
		let key = '';
		let current = parentId;
		for (const d of dirs) {
			key += '/' + d;
			if (!cache.has(key)) {
				try {
					const f = await api.createFolder(current, d);
					cache.set(key, f.id);
				} catch (e) {
					uploads.jobs.push({ id: ++seq, name: file.name, size: file.size, sent: 0, status: 'error', error: errorMessage(e), parentId, file });
					current = null;
					break;
				}
			}
			current = cache.get(key) ?? null;
		}
		out.push({ file, parentId: current });
	}
	enqueue(out);
}

/** Collect files (and nested folders) from a drop event. */
export async function filesFromDataTransfer(dt: DataTransfer): Promise<{ file: File; path: string }[]> {
	const result: { file: File; path: string }[] = [];
	const entries: FileSystemEntry[] = [];
	for (const item of Array.from(dt.items ?? [])) {
		const e = item.kind === 'file' ? item.webkitGetAsEntry?.() : null;
		if (e) entries.push(e);
	}
	if (!entries.length) {
		for (const f of Array.from(dt.files)) result.push({ file: f, path: f.name });
		return result;
	}
	const walk = async (entry: FileSystemEntry, prefix: string): Promise<void> => {
		if (entry.isFile) {
			const file = await new Promise<File>((res, rej) => (entry as FileSystemFileEntry).file(res, rej));
			result.push({ file, path: prefix + entry.name });
		} else if (entry.isDirectory) {
			const reader = (entry as FileSystemDirectoryEntry).createReader();
			let batch: FileSystemEntry[];
			do {
				batch = await new Promise<FileSystemEntry[]>((res, rej) => reader.readEntries(res, rej));
				for (const child of batch) await walk(child, prefix + entry.name + '/');
			} while (batch.length);
		}
	};
	for (const e of entries) await walk(e, '');
	return result;
}

export function cancel(job: UploadJob) {
	if (job.status === 'done') return;
	job.controller?.abort();
	job.status = 'canceled';
	if (job.uploadId) api.cancelUpload(job.uploadId).catch(() => {});
}

export function clearFinished() {
	uploads.jobs = uploads.jobs.filter((j) => j.status === 'queued' || j.status === 'uploading');
}

export function retry(job: UploadJob) {
	job.status = 'queued';
	job.error = undefined;
	pump();
}

export const activeCount = () => uploads.jobs.filter((j) => j.status === 'queued' || j.status === 'uploading').length;

function pump() {
	while (running < CONCURRENCY) {
		const next = uploads.jobs.find((j) => j.status === 'queued');
		if (!next) return;
		next.status = 'uploading';
		running++;
		void run(next).finally(() => {
			running--;
			pump();
		});
	}
}

async function run(job: UploadJob) {
	// `status` can be flipped to 'canceled' from the UI while awaiting, which TypeScript cannot see.
	const canceled = () => (job.status as UploadStatus) === 'canceled';
	job.controller = new AbortController();
	try {
		if (!job.uploadId) {
			const { upload } = job.replaceNodeId ? await api.createVersionUpload(job.replaceNodeId, job.size) : await api.createUpload(job.parentId, job.name, job.size);
			job.uploadId = upload.id;
		}
		let offset = job.sent > 0 ? await api.uploadOffset(job.uploadId) : 0;
		let attempts = 0;
		for (;;) {
			if (canceled()) return;
			const blob = job.file.slice(offset, Math.min(offset + CHUNK, job.size));
			try {
				const r = await uploadChunk(job.uploadId, offset, blob, (loaded) => (job.sent = offset + loaded), job.controller.signal);
				attempts = 0;
				offset = r.received;
				job.sent = offset;
				if (r.node) {
					job.nodeId = r.node.id;
					job.status = 'done';
					job.sent = job.size;
					uploads.doneVersion++;
					refreshStorage();
					scheduleRefresh();
					return;
				}
			} catch (e) {
				if (canceled() || (e instanceof ApiError && e.code === 'aborted')) return;
				const err = e instanceof ApiError ? e : new ApiError(0, 'network', 'Network error');
				if (err.code === 'offset_mismatch') {
					offset = await api.uploadOffset(job.uploadId);
					continue;
				}
				const transient = err.status === 0 || err.status >= 500;
				if (!transient || ++attempts > MAX_RETRIES) throw err;
				await sleep(Math.min(30_000, 1000 * 2 ** (attempts - 1)));
				// Resume from what the server really has.
				try {
					offset = await api.uploadOffset(job.uploadId);
				} catch {
					/* retry loop will handle it */
				}
			}
		}
	} catch (e) {
		if (canceled()) return;
		job.status = 'error';
		const err = e instanceof ApiError ? e : null;
		job.error =
			err?.code === 'quota_exceeded' ? t('upload.error.quota') : err?.code === 'too_large' ? t('upload.error.too_large') : e instanceof Error ? errorMessage(e) : t('upload.error.failed');
	}
}

if (typeof window !== 'undefined') {
	window.addEventListener('beforeunload', (e) => {
		if (activeCount() > 0) {
			e.preventDefault();
			e.returnValue = '';
		}
	});
}
