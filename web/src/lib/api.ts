// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

import type {
	AdminOverview, AdminUserRow, BackupItem, CommentItem, Crumb, JobControls, ListView, LinkEntry, Me, NodeItem, Page, ShareEntry,
	ShareInfo, SortKey, Suggestion, User, UserBrief, VersionItem
} from './types';

const BASE = '/api/v1';

export class ApiError extends Error {
	constructor(
		public status: number,
		public code: string,
		message: string
	) {
		super(message);
	}
}

/** Raised session-wide when the server says the session is gone. */
let onUnauthorized: (() => void) | null = null;
export function setUnauthorizedHandler(fn: () => void) {
	onUnauthorized = fn;
}

async function request<T>(method: string, path: string, body?: unknown, init: RequestInit = {}): Promise<T> {
	const headers = new Headers(init.headers);
	if (method !== 'GET' && method !== 'HEAD') headers.set('X-Drivich-CSRF', '1');
	let payload: BodyInit | undefined;
	if (body !== undefined) {
		headers.set('Content-Type', 'application/json');
		payload = JSON.stringify(body);
	}
	const res = await fetch(BASE + path, { method, headers, body: payload, credentials: 'same-origin', ...init });
	if (!res.ok) {
		let code = 'error';
		let message = res.statusText;
		try {
			const j = await res.json();
			code = j?.error?.code ?? code;
			message = j?.error?.message ?? message;
		} catch {
			/* not JSON */
		}
		if (res.status === 401 && code === 'unauthorized' && onUnauthorized) onUnauthorized();
		throw new ApiError(res.status, code, message);
	}
	if (res.status === 204) return undefined as T;
	return (await res.json()) as T;
}

const get = <T>(p: string, init?: RequestInit) => request<T>('GET', p, undefined, init);
const post = <T>(p: string, b?: unknown) => request<T>('POST', p, b ?? {});
const patch = <T>(p: string, b: unknown) => request<T>('PATCH', p, b);
const put = <T>(p: string, b: unknown) => request<T>('PUT', p, b);
const del = <T>(p: string) => request<T>('DELETE', p);

function qs(params: Record<string, string | number | boolean | null | undefined>): string {
	const u = new URLSearchParams();
	for (const [k, v] of Object.entries(params)) if (v !== undefined && v !== null && v !== '') u.set(k, String(v));
	const s = u.toString();
	return s ? `?${s}` : '';
}

export interface ListOptions {
	view?: ListView;
	parent?: string | null;
	sort?: SortKey;
	order?: 'asc' | 'desc';
	type?: string;
	modified_from?: string;
	owner?: string;
	location?: string;
	cursor?: string | null;
	limit?: number;
}

export interface SearchOptions {
	q?: string;
	name?: string;
	words?: string;
	type?: string;
	owner?: string;
	modified_from?: string;
	modified_to?: string;
	location?: string;
	in_folder?: string;
	cursor?: string | null;
}

export const api = {
	// auth
	status: () => get<{ setup_required: boolean; signup_allowed: boolean; version: string }>('/auth/status'),
	setup: (b: { email: string; name: string; password: string }) => post<{ user: User }>('/auth/setup', b),
	login: (b: { email: string; password: string }) => post<{ user: User }>('/auth/login', b),
	signup: (b: { email: string; name: string; password: string }) => post<{ user: User }>('/auth/signup', b),
	logout: () => post<{ ok: boolean }>('/auth/logout'),
	me: () => get<Me>('/me'),
	updateMe: (b: { name?: string; immich_url?: string | null; locale?: string | null }) => patch<{ user: User }>('/me', b),
	/** Upload a profile picture (any common image format; the server re-encodes it). */
	uploadAvatar: async (file: Blob): Promise<{ user: User }> => {
		const res = await fetch(BASE + '/me/avatar', {
			method: 'PUT',
			headers: { 'X-Drivich-CSRF': '1', 'Content-Type': file.type || 'application/octet-stream' },
			body: file,
			credentials: 'same-origin'
		});
		if (!res.ok) {
			let message = res.statusText;
			let code = 'error';
			try {
				const j = await res.json();
				message = j?.error?.message ?? message;
				code = j?.error?.code ?? code;
			} catch {
				/* not JSON */
			}
			throw new ApiError(res.status, code, message);
		}
		return res.json();
	},
	removeAvatar: () => del<{ user: User }>('/me/avatar'),
	changePassword: (current: string, next: string) => post<{ user: User }>('/me/password', { current, new: next }),

	// nodes
	list: (o: ListOptions) =>
		get<Page>('/nodes' + qs({ view: o.view, parent: o.parent ?? undefined, sort: o.sort, order: o.order, type: o.type, modified_from: o.modified_from, owner: o.owner, location: o.location, cursor: o.cursor, limit: o.limit })),
	node: (id: string) => get<NodeItem>(`/nodes/${id}`),
	path: (id: string) => get<{ items: Crumb[] }>(`/nodes/${id}/path`),
	createFolder: (parent_id: string | null, name: string) => post<NodeItem>('/nodes/folder', { parent_id, name }),
	rename: (id: string, name: string) => patch<NodeItem>(`/nodes/${id}`, { name }),
	move: (id: string, parent_id: string | null) => patch<NodeItem>(`/nodes/${id}`, { parent_id }),
	star: (id: string, starred: boolean) => patch<NodeItem>(`/nodes/${id}`, { starred }),
	describe: (id: string, description: string | null) => patch<NodeItem>(`/nodes/${id}`, { description }),
	trash: (id: string) => post<{ ok: boolean }>(`/nodes/${id}/trash`),
	restore: (id: string) => post<NodeItem>(`/nodes/${id}/restore`),
	deleteForever: (id: string) => del<{ ok: boolean }>(`/nodes/${id}`),
	emptyTrash: () => post<{ deleted: number }>('/trash/empty'),
	batch: (ids: string[], action: 'trash' | 'restore' | 'star' | 'unstar' | 'move' | 'delete', parent_id?: string | null) =>
		post<{ ok: string[]; failed: [string, string][] }>('/nodes/batch', { ids, action, parent_id: parent_id ?? null }),
	opened: (id: string) => post<void>(`/nodes/${id}/open`),

	// search
	search: (o: SearchOptions) => get<Page>('/search' + qs({ ...o })),
	suggest: (q: string, signal?: AbortSignal) => get<{ items: Suggestion[] }>('/search/suggest' + qs({ q }), { signal }),
	lookupUsers: (q: string, signal?: AbortSignal) => get<{ items: UserBrief[] }>('/users/lookup' + qs({ q }), { signal }),

	// sharing
	shares: (id: string) => get<ShareInfo>(`/nodes/${id}/shares`),
	addShare: (id: string, email: string, role: string) => post<{ share: ShareEntry }>(`/nodes/${id}/shares`, { email, role }),
	updateShare: (shareId: string, role: string) => patch<{ ok: boolean }>(`/shares/${shareId}`, { role }),
	removeShare: (shareId: string) => del<{ ok: boolean }>(`/shares/${shareId}`),
	createLink: (id: string, expires_at: string | null) => post<{ link: LinkEntry; token: string; path: string }>(`/nodes/${id}/links`, { expires_at }),
	revokeLink: (linkId: string) => del<{ ok: boolean }>(`/links/${linkId}`),

	// public links
	publicRoot: (token: string) => get<{ root: NodeItem }>(`/public/${token}`),
	publicNode: (token: string, id: string) => get<{ node: NodeItem }>(`/public/${token}/nodes/${id}`),
	publicChildren: (token: string, id: string, cursor?: string | null) => get<Page>(`/public/${token}/nodes/${id}/children` + qs({ cursor })),

	// comments
	comments: (id: string) => get<{ items: CommentItem[] }>(`/nodes/${id}/comments`),
	addComment: (id: string, body: string) => post<{ comment: CommentItem }>(`/nodes/${id}/comments`, { body }),
	editComment: (id: string, body: string) => patch<{ comment: CommentItem }>(`/comments/${id}`, { body }),
	deleteComment: (id: string) => del<{ ok: boolean }>(`/comments/${id}`),

	// versions and previews
	versions: (id: string) => get<{ items: VersionItem[]; max_versions: number }>(`/nodes/${id}/versions`),
	restoreVersion: (id: string, version: string) => post<{ node: NodeItem }>(`/nodes/${id}/versions/${version}/restore`),
	deleteVersion: (id: string, version: string) => del<{ ok: boolean }>(`/nodes/${id}/versions/${version}`),
	requestPreview: (id: string) => post<{ state: 'none' | 'pending' | 'ready' | 'failed' | 'unsupported' }>(`/nodes/${id}/preview`),
	createVersionUpload: (node_id: string, size: number) =>
		post<{ upload: { id: string; received: number; total_size: number } }>('/uploads', { replace_node_id: node_id, size }),

	// uploads
	createUpload: (parent_id: string | null, name: string, size: number) =>
		post<{ upload: { id: string; received: number; total_size: number } }>('/uploads', { parent_id, name, size }),
	uploadOffset: async (id: string): Promise<number> => {
		const res = await fetch(`${BASE}/uploads/${id}`, { method: 'HEAD', credentials: 'same-origin' });
		if (!res.ok) throw new ApiError(res.status, 'error', res.statusText);
		return Number(res.headers.get('upload-offset') ?? 0);
	},
	cancelUpload: (id: string) => del<{ ok: boolean }>(`/uploads/${id}`),

	// admin
	adminOverview: () => get<AdminOverview>('/admin/overview'),
	adminUsers: () => get<{ items: AdminUserRow[] }>('/admin/users'),
	adminCreateUser: (b: { email: string; name: string; password: string; is_admin: boolean; quota_bytes: number | null }) => post<{ user: User }>('/admin/users', b),
	adminUpdateUser: (id: string, b: { is_admin?: boolean; disabled?: boolean; quota_bytes?: number | null }) => patch<{ user: User }>(`/admin/users/${id}`, b),
	adminResetPassword: (id: string, password: string) => post<{ ok: boolean }>(`/admin/users/${id}/password`, { password }),
	adminJobControls: () => get<Required<JobControls>>('/admin/jobs/config'),
	adminSaveJobControls: (c: JobControls) => put<Required<JobControls>>('/admin/jobs/config', { paused: c.paused, limits: c.limits }),
	adminBackups: () => get<{ items: BackupItem[]; enabled: boolean; keep: number }>('/admin/backups'),
	adminCreateBackup: () => post<{ queued: boolean }>('/admin/backups'),
	adminRetryJobs: () => post<{ requeued: number }>('/admin/jobs/retry'),
	adminSettings: () => get<{ allow_signup: boolean; default_quota_bytes: number | null; immich_url: string | null }>('/admin/settings'),
	adminSaveSettings: (b: { allow_signup?: boolean; default_quota_bytes?: number | null; immich_url?: string | null }) =>
		put<{ allow_signup: boolean; default_quota_bytes: number | null; immich_url: string | null }>('/admin/settings', b)
};

// ---- URLs for <img>, <video>, downloads ------------------------------------------
export const urls = {
	adminBackup: (name: string) => `${BASE}/admin/backups/${encodeURIComponent(name)}/download`,
	avatar: (userId: string, version: number) => `${BASE}/users/${userId}/avatar?v=${version}`,
	download: (id: string) => `${BASE}/nodes/${id}/download`,
	content: (id: string) => `${BASE}/nodes/${id}/content`,
	zip: (id: string) => `${BASE}/nodes/${id}/zip`,
	zipSelection: (ids: string[]) => `${BASE}/nodes/zip?ids=${ids.join(',')}`,
	publicZip: (t: string, id: string) => `${BASE}/public/${t}/nodes/${id}/zip`,
	preview: (id: string) => `${BASE}/nodes/${id}/preview`,
	versionDownload: (id: string, version: string) => `${BASE}/nodes/${id}/versions/${version}/download`,
	thumb: (id: string) => `${BASE}/nodes/${id}/thumbnail`,
	publicDownload: (t: string, id: string) => `${BASE}/public/${t}/nodes/${id}/download`,
	publicContent: (t: string, id: string) => `${BASE}/public/${t}/nodes/${id}/content`,
	publicThumb: (t: string, id: string) => `${BASE}/public/${t}/nodes/${id}/thumbnail`
};

export function uploadChunk(
	uploadId: string,
	offset: number,
	blob: Blob,
	onProgress: (loaded: number) => void,
	signal: AbortSignal
): Promise<{ node: NodeItem | null; received: number }> {
	return new Promise((resolve, reject) => {
		const xhr = new XMLHttpRequest();
		xhr.open('PATCH', `${BASE}/uploads/${uploadId}`);
		xhr.withCredentials = true;
		xhr.setRequestHeader('X-Drivich-CSRF', '1');
		xhr.setRequestHeader('Upload-Offset', String(offset));
		xhr.setRequestHeader('Content-Type', 'application/offset+octet-stream');
		xhr.upload.onprogress = (e) => onProgress(e.loaded);
		xhr.onload = () => {
			let j: any = null;
			try {
				j = JSON.parse(xhr.responseText);
			} catch {
				/* ignore */
			}
			if (xhr.status >= 200 && xhr.status < 300) resolve({ node: j?.node ?? null, received: j?.upload?.received ?? offset + blob.size });
			else reject(new ApiError(xhr.status, j?.error?.code ?? 'error', j?.error?.message ?? xhr.statusText));
		};
		xhr.onerror = () => reject(new ApiError(0, 'network', 'Network error'));
		xhr.onabort = () => reject(new ApiError(0, 'aborted', 'Canceled'));
		signal.addEventListener('abort', () => xhr.abort(), { once: true });
		xhr.send(blob);
	});
}
