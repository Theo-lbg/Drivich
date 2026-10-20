// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

import { locale, t } from './i18n/index.svelte';
import type { NodeItem } from './types';

const UNIT_KEYS = ['unit.b', 'unit.kb', 'unit.mb', 'unit.gb', 'unit.tb'];

/** Human size in the current language: "137,4 Mo" (fr), "137.4 MB" (en). */
export function formatBytes(n: number | null | undefined): string {
	if (n === null || n === undefined) return '—';
	let v = n;
	let i = 0;
	while (v >= 1024 && i < UNIT_KEYS.length - 1) {
		v /= 1024;
		i++;
	}
	const digits = i === 0 || v >= 100 ? 0 : 1;
	return `${v.toLocaleString(locale(), { maximumFractionDigits: digits, minimumFractionDigits: 0 })} ${t(UNIT_KEYS[i])}`;
}

const DAY = 86_400_000;

/** Drive-style date: time today, "5 Oct" this year, "5 Oct 2024" otherwise (localised). */
export function formatDate(iso: string | null | undefined): string {
	if (!iso) return '—';
	const d = new Date(iso);
	const now = new Date();
	if (d.toDateString() === now.toDateString()) return d.toLocaleTimeString(locale(), { hour: '2-digit', minute: '2-digit' });
	if (d.getFullYear() === now.getFullYear()) return d.toLocaleDateString(locale(), { day: 'numeric', month: 'short' });
	return d.toLocaleDateString(locale(), { day: 'numeric', month: 'short', year: 'numeric' });
}

export function formatDateLong(iso: string | null | undefined): string {
	if (!iso) return '—';
	return new Date(iso).toLocaleString(locale(), { dateStyle: 'long', timeStyle: 'short' });
}

export function daysLeft(iso: string | null, days = 30): number | null {
	if (!iso) return null;
	return Math.max(0, Math.ceil((new Date(iso).getTime() + days * DAY - Date.now()) / DAY));
}

export type FileClass = 'folder' | 'image' | 'video' | 'audio' | 'pdf' | 'archive' | 'sheet' | 'slides' | 'doc' | 'code' | 'text' | 'file';

export function classify(n: Pick<NodeItem, 'kind' | 'mime' | 'name'>): FileClass {
	if (n.kind === 'folder') return 'folder';
	const m = n.mime ?? '';
	if (m.startsWith('image/')) return 'image';
	if (m.startsWith('video/')) return 'video';
	if (m.startsWith('audio/')) return 'audio';
	if (m === 'application/pdf') return 'pdf';
	if (/zip|tar|gzip|rar|7z|apple-diskimage/.test(m)) return 'archive';
	if (/spreadsheet|ms-excel|csv/.test(m) || m === 'text/csv') return 'sheet';
	if (/presentation|ms-powerpoint/.test(m)) return 'slides';
	if (/wordprocessing|msword|opendocument\.text|rtf/.test(m)) return 'doc';
	if (/json|xml|javascript|x-sh|x-python|x-rust|x-c/.test(m) || /\.(rs|ts|tsx|js|svelte|py|go|c|h|cpp|java|sh|sql|toml|yaml|yml)$/i.test(n.name)) return 'code';
	if (m.startsWith('text/')) return 'text';
	return 'file';
}

/** Label of a file class in the current language. */
export const classLabel = (c: FileClass): string => t(`file.${c}`);

export const CLASS_ICON: Record<FileClass, string> = {
	folder: 'folder',
	image: 'image',
	video: 'movie',
	audio: 'audio_file',
	pdf: 'picture_as_pdf',
	archive: 'folder_zip',
	sheet: 'table_chart',
	slides: 'slideshow',
	doc: 'description',
	code: 'code',
	text: 'text_snippet',
	file: 'draft'
};

/** Accent colour of a file type, as in Drive's list view. */
export const CLASS_COLOR: Record<FileClass, string> = {
	folder: 'var(--file-gray)',
	image: 'var(--file-red)',
	video: 'var(--file-red)',
	audio: 'var(--file-orange)',
	pdf: 'var(--file-red)',
	archive: 'var(--file-gray)',
	sheet: 'var(--file-green)',
	slides: 'var(--file-orange)',
	doc: 'var(--file-blue)',
	code: 'var(--file-gray)',
	text: 'var(--file-gray)',
	file: 'var(--file-gray)'
};

export function typeLabel(n: Pick<NodeItem, 'kind' | 'mime' | 'name'>): string {
	return classLabel(classify(n));
}

export function isPreviewable(n: Pick<NodeItem, 'kind' | 'mime' | 'name'>): boolean {
	const c = classify(n);
	return c === 'image' || c === 'video' || c === 'audio' || c === 'pdf' || c === 'text' || c === 'code';
}

export function pct(used: number, total: number | null): number {
	if (!total) return 0;
	return Math.min(100, (used / total) * 100);
}
