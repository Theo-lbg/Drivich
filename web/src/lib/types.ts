// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

export type Role = 'owner' | 'editor' | 'commenter' | 'viewer';

export interface User {
	id: string;
	email: string;
	name: string;
	is_admin: boolean;
	disabled: boolean;
	quota_bytes: number | null;
	immich_url: string | null;
	/** Profile picture version; 0 = none (shows the initial). */
	avatar_version: number;
	/** Interface language (BCP 47) or null = follow the browser. */
	locale: string | null;
	created_at: string;
}

export interface NodeItem {
	id: string;
	kind: 'file' | 'folder';
	name: string;
	mime: string | null;
	size: number;
	parent_id: string | null;
	owner_id: string;
	owner_name: string;
	owner_avatar_version: number;
	starred: boolean;
	trashed_at: string | null;
	created_at: string;
	updated_at: string;
	last_opened_at: string | null;
	description: string | null;
	thumb_state: 'none' | 'pending' | 'ready' | 'failed' | 'unsupported' | null;
	role: Role;
}

export interface Page {
	items: NodeItem[];
	next_cursor: string | null;
}

export interface Suggestion {
	id: string;
	kind: 'file' | 'folder';
	name: string;
	mime: string | null;
	parent_id: string | null;
	parent_name: string | null;
	thumb_state: string | null;
}

export interface Crumb {
	id: string;
	name: string;
}

export interface UserBrief {
	id: string;
	email: string;
	name: string;
	avatar_version: number;
}

export interface ShareEntry {
	id: string;
	grantee_id: string;
	grantee_name: string;
	grantee_email: string;
	grantee_avatar_version: number;
	role: 'viewer' | 'commenter' | 'editor';
	created_at: string;
}

export interface LinkEntry {
	id: string;
	role: 'viewer';
	expires_at: string | null;
	revoked_at: string | null;
	created_at: string;
}

export interface ShareInfo {
	owner: UserBrief;
	users: ShareEntry[];
	links: LinkEntry[];
}

export interface Me {
	user: User;
	used_bytes: number;
	quota_bytes: number | null;
	disk: { free_bytes: number; total_bytes: number } | null;
	/** Effective Immich address: personal, else the server default. */
	immich_url: string | null;
	immich_default_url: string | null;
}

export type ListView = 'drive' | 'shared' | 'recent' | 'starred' | 'trash';
export type SortKey = 'name' | 'modified' | 'created' | 'size';

export interface AdminOverview {
	version: string;
	database: { ok: boolean; version: string };
	counts: { users: number; files: number; folders: number; stored_bytes: number };
	disk: { free_bytes: number; total_bytes: number } | null;
	data_dir: string;
	limits: { max_file_bytes: number; trash_days: number; upload_ttl_hours: number };
	jobs: {
		stats: { kind: string; status: string; count: number }[];
		recent_failures: { id: number; kind: string; attempts: number; last_error: string | null; updated_at: string }[];
	};
}

export interface JobControls {
	paused: string[];
	/** Max simultaneous jobs per kind (absent = no limit). */
	limits: Record<string, number>;
	kinds?: string[];
}

export interface BackupItem {
	name: string;
	size: number;
	created_at: string;
}

export interface AdminUserRow {
	user: User;
	used_bytes: number;
}

export interface Filters {
	type: string;
	date: string;
	owner: string;
	location: string;
}

export interface CommentItem {
	id: string;
	node_id: string;
	author_id: string;
	author_name: string;
	author_avatar_version: number;
	body: string;
	created_at: string;
	edited_at: string | null;
	can_edit: boolean;
	can_delete: boolean;
}

export interface VersionItem {
	id: string;
	size: number;
	mime: string;
	saved_at: string;
	replaced_at: string;
}
