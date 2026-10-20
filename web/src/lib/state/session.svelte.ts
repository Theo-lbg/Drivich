// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

import { api, ApiError } from '$lib/api';
import type { Me, User } from '$lib/types';

export const session = $state<{
	ready: boolean;
	user: User | null;
	used: number;
	quota: number | null;
	disk: Me['disk'];
	office: boolean;
	immichUrl: string | null;
	immichDefault: string | null;
	setupRequired: boolean;
	signupAllowed: boolean;
	version: string;
}>({ ready: false, user: null, used: 0, quota: null, disk: null, office: false, immichUrl: null, immichDefault: null, setupRequired: false, signupAllowed: false, version: '' });

export async function loadSession() {
	try {
		const st = await api.status();
		session.setupRequired = st.setup_required;
		session.signupAllowed = st.signup_allowed;
		session.version = st.version;
		if (!st.setup_required) {
			try {
				applyMe(await api.me());
			} catch (e) {
				if (!(e instanceof ApiError) || e.status !== 401) throw e;
				session.user = null;
			}
		}
	} finally {
		session.ready = true;
	}
}

export function applyMe(me: Me) {
	session.user = me.user;
	session.used = me.used_bytes;
	session.quota = me.quota_bytes;
	session.disk = me.disk;
	session.office = !!me.office;
	session.immichUrl = me.immich_url;
	session.immichDefault = me.immich_default_url;
}

export async function refreshStorage() {
	try {
		applyMe(await api.me());
	} catch {
		/* ignore: transient */
	}
}

export async function signOut() {
	try {
		await api.logout();
	} finally {
		session.user = null;
	}
}
