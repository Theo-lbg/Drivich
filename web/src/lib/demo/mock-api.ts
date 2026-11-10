// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

// In-browser stand-in for the Drivich API, used only by the static demo build
// (VITE_DEMO=true, hosted on GitHub Pages). It keeps a small drive in memory:
// everything resets when the page is reloaded and nothing leaves the browser.
// Nothing here is reachable from a normal build (the call sites are guarded by
// `import.meta.env.VITE_DEMO`, so the bundler drops this module).

import { asset as kitAsset } from '$app/paths';
import type {
	AdminOverview, AdminUserRow, CommentItem, Crumb, LinkEntry, Me, NodeItem, Page, Role, ShareEntry, ShareInfo, Suggestion, User, UserBrief
} from '$lib/types';

export class DemoError extends Error {
	constructor(public status: number, public code: string, message: string) {
		super(message);
	}
}

interface DNode extends NodeItem {
	/** URL of the file content (static demo asset or blob: URL). */
	url?: string;
	thumb?: string;
}

interface Store {
	users: User[];
	nodes: Map<string, DNode>;
	shares: (ShareEntry & { node_id: string })[];
	links: (LinkEntry & { node_id: string })[];
	comments: CommentItem[];
	uploads: Map<string, { parent_id: string | null; name: string }>;
	seq: number;
}

/** Static files of the demo (web/static/demo); a plain string is fine here. */
const asset = (file: string) => kitAsset(file as never);

const ME = 'u-camille';
const LUCAS = 'u-lucas';
let store: Store | null = null;

const MIME: Record<string, string> = {
	jpg: 'image/jpeg', jpeg: 'image/jpeg', png: 'image/png', gif: 'image/gif', webp: 'image/webp', pdf: 'application/pdf',
	txt: 'text/plain', md: 'text/markdown', csv: 'text/csv', py: 'text/x-python', json: 'application/json',
	docx: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
	xlsx: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
	pptx: 'application/vnd.openxmlformats-officedocument.presentationml.presentation',
	zip: 'application/zip', mp3: 'audio/mpeg', mp4: 'video/mp4'
};
const mimeOf = (name: string) => MIME[name.split('.').pop()?.toLowerCase() ?? ''] ?? 'application/octet-stream';

function textUrl(text: string, mime = 'text/plain'): string {
	return URL.createObjectURL(new Blob([text], { type: `${mime};charset=utf-8` }));
}

function ago(minutes: number): string {
	return new Date(Date.now() - minutes * 60_000).toISOString();
}

function init(): Store {
	if (store) return store;
	const s: Store = { users: [], nodes: new Map(), shares: [], links: [], comments: [], uploads: new Map(), seq: 100 };
	const user = (id: string, name: string, email: string, admin: boolean): User => ({
		id, name, email, is_admin: admin, disabled: false, quota_bytes: 15 * 1024 ** 3, immich_url: null, avatar_version: 0,
		locale: null, created_at: ago(60 * 24 * 40)
	});
	s.users = [user(ME, 'Camille Martin', 'camille@example.org', true), user(LUCAS, 'Lucas Bernard', 'lucas@example.org', false)];

	const owner = (id: string) => s.users.find((u) => u.id === id)!;
	let order = 0;
	const add = (
		id: string, kind: 'file' | 'folder', name: string, parent: string | null,
		o: { size?: number; url?: string; thumb?: string; starred?: boolean; trashed?: boolean; owner?: string; mins?: number; opened?: number; role?: Role; description?: string } = {}
	) => {
		const ow = owner(o.owner ?? ME);
		const mime = kind === 'file' ? mimeOf(name) : null;
		const mins = o.mins ?? 60 * 24 * 3 + order++ * 37;
		s.nodes.set(id, {
			id, kind, name, mime, size: kind === 'file' ? (o.size ?? 48_000) : 0, parent_id: parent, owner_id: ow.id, owner_name: ow.name,
			owner_avatar_version: 0, starred: !!o.starred, trashed_at: o.trashed ? ago(60 * 24) : null, created_at: ago(mins + 500), updated_at: ago(mins),
			last_opened_at: o.opened !== undefined ? ago(o.opened) : null, description: o.description ?? null,
			thumb_state: mime?.startsWith('image/') ? 'ready' : kind === 'file' ? 'unsupported' : null, role: o.role ?? 'owner', url: o.url, thumb: o.thumb
		});
	};
	const photo = (n: number) => ({ url: asset(`demo/photos/p${String(n).padStart(2, '0')}.jpg`), thumb: asset(`demo/thumbs/p${String(n).padStart(2, '0')}.jpg`), size: 780_000 + n * 21_000 });
	const doc = (n: string, size: number) => ({ url: asset(`demo/docs/${n}.pdf`), size });

	add('f-photos', 'folder', 'Photos', null, { mins: 60 * 5 });
	add('f-vac', 'folder', 'Vacances 2026', 'f-photos', { mins: 60 * 3 });
	add('f-mont', 'folder', 'Montagne', 'f-photos', { mins: 60 * 30 });
	add('f-docs', 'folder', 'Documents', null, { mins: 60 * 2 });
	add('f-fact', 'folder', 'Factures', 'f-docs', { mins: 60 * 26 });
	add('f-proj', 'folder', 'Projets', null, { mins: 60 * 8 });
	const vac = ['Coucher de soleil', 'Crépuscule', 'Forêt', 'Lac', 'Montagne', 'Panorama', 'Plage du matin', 'Randonnée'];
	vac.forEach((n, i) => add(`p${i + 1}`, 'file', `${n}.jpg`, 'f-vac', { ...photo(i + 1), starred: [0, 1, 4].includes(i), mins: 60 * 3 + i * 9, opened: i === 0 ? 20 : undefined }));
	['Col alpin', 'Ciel rose', 'Vallée', 'Bord de mer'].forEach((n, i) => add(`p${i + 9}`, 'file', `${n}.jpg`, 'f-mont', { ...photo(i + 9), mins: 60 * 30 + i * 11 }));
	add('d-rapport', 'file', 'Rapport annuel 2026.pdf', 'f-docs', { ...doc('rapport', 82_000), starred: true, mins: 90, opened: 45 });
	add('d-contrat', 'file', 'Contrat de location.pdf', 'f-docs', { ...doc('contrat', 81_300), mins: 60 * 20 });
	add('d-plan', 'file', 'Plan de projet.pdf', 'f-docs', { ...doc('plan', 79_500), mins: 60 * 7 });
	add('d-fm', 'file', 'Facture mars.pdf', 'f-fact', { ...doc('facture-mars', 74_000), mins: 60 * 24 * 9 });
	add('d-fa', 'file', 'Facture avril.pdf', 'f-fact', { ...doc('facture-avril', 74_500), mins: 60 * 24 * 4 });
	add('d-notes', 'file', 'Notes de réunion.txt', 'f-docs', { url: textUrl('Points à aborder :\n- Budget\n- Planning\n- Prochaine démo\n'), size: 52, mins: 60 * 4, opened: 120 });
	add('r-readme', 'file', 'README.md', 'f-proj', { url: textUrl('# Mon projet\n\nBienvenue sur la démo de **Drivich**.\n\n- Fichiers et dossiers\n- Partage et liens\n- Corbeille\n', 'text/markdown'), size: 118, mins: 60 * 6 });
	add('r-budget', 'file', 'budget.csv', 'f-proj', { url: textUrl('mois,montant\njanvier,1200\nfévrier,980\nmars,1430\n', 'text/csv'), size: 54, mins: 60 * 15 });
	add('r-script', 'file', 'script.py', 'f-proj', { url: textUrl('print("bonjour depuis Drivich")\n', 'text/x-python'), size: 31, mins: 60 * 12 });
	add('t-old', 'file', 'Ancien brouillon.txt', null, { url: textUrl('brouillon'), size: 9, trashed: true });

	add('l-folder', 'folder', 'Projet partagé', null, { owner: LUCAS, role: 'editor', mins: 200 });
	add('l-maq', 'file', 'Maquette.jpg', 'l-folder', { ...photo(4), owner: LUCAS, role: 'editor', mins: 30, opened: 10 });
	add('l-cahier', 'file', 'Cahier des charges.pdf', 'l-folder', { ...doc('cahier', 85_300), owner: LUCAS, role: 'editor', mins: 50 });
	s.shares.push({ id: 'sh-1', node_id: 'l-folder', grantee_id: ME, grantee_name: 'Camille Martin', grantee_email: 'camille@example.org', grantee_avatar_version: 0, role: 'editor', created_at: ago(300) });
	s.shares.push({ id: 'sh-2', node_id: 'f-vac', grantee_id: LUCAS, grantee_name: 'Lucas Bernard', grantee_email: 'lucas@example.org', grantee_avatar_version: 0, role: 'viewer', created_at: ago(900) });
	s.comments.push({ id: 'c-1', node_id: 'p1', author_id: LUCAS, author_name: 'Lucas Bernard', author_avatar_version: 0, body: 'Superbe lumière !', created_at: ago(120), edited_at: null, can_edit: false, can_delete: false });
	store = s;
	return s;
}

// ---- helpers ---------------------------------------------------------------------
const strip = (n: DNode): NodeItem => {
	const { url: _u, thumb: _t, ...rest } = n;
	return rest;
};
const nid = (s: Store, p: string) => `${p}-${++s.seq}`;
const err = (status: number, code: string, message: string): never => {
	throw new DemoError(status, code, message);
};
const nodeOr404 = (s: Store, id: string) => s.nodes.get(id) ?? err(404, 'not_found', 'Introuvable');
const touch = (n: DNode) => (n.updated_at = new Date().toISOString());

function inSubtree(s: Store, id: string, ancestor: string): boolean {
	for (let cur: string | null = id; cur; cur = s.nodes.get(cur)?.parent_id ?? null) if (cur === ancestor) return true;
	return false;
}
function descendants(s: Store, id: string): DNode[] {
	const out: DNode[] = [];
	for (const n of s.nodes.values()) if (n.id !== id && inSubtree(s, n.id, id)) out.push(n);
	return out;
}
function dropSubtree(s: Store, id: string) {
	for (const d of descendants(s, id)) s.nodes.delete(d.id);
	s.nodes.delete(id);
}
function typeMatches(n: NodeItem, t: string): boolean {
	const m = n.mime ?? '';
	switch (t) {
		case 'folder': return n.kind === 'folder';
		case 'image': case 'video': case 'audio': return m.startsWith(t + '/');
		case 'pdf': return m === 'application/pdf';
		case 'text': return m.startsWith('text/') || m === 'application/json';
		case 'document': return m.includes('officedocument') || m === 'application/msword';
		case 'archive': return m === 'application/zip';
		default: return true;
	}
}
function collator(a: string, b: string) {
	return a.localeCompare(b, 'fr', { sensitivity: 'base', numeric: true });
}
function sortNodes(items: DNode[], sort: string, order: string): DNode[] {
	const dir = order === 'desc' ? -1 : 1;
	return [...items].sort((a, b) => {
		if ((a.kind === 'folder') !== (b.kind === 'folder')) return a.kind === 'folder' ? -1 : 1;
		let c = 0;
		if (sort === 'modified') c = a.updated_at.localeCompare(b.updated_at);
		else if (sort === 'created') c = a.created_at.localeCompare(b.created_at);
		else if (sort === 'size') c = a.size - b.size;
		else c = collator(a.name, b.name);
		return c * dir || collator(a.name, b.name);
	});
}
function applyFilters(items: DNode[], q: URLSearchParams): DNode[] {
	const type = q.get('type');
	const from = q.get('modified_from');
	const owner = q.get('owner');
	return items.filter(
		(n) =>
			(!type || typeMatches(n, type)) &&
			(!from || n.updated_at >= from) &&
			(!owner || owner === 'any' || (owner === 'me' ? n.owner_id === ME : n.owner_id !== ME))
	);
}
function page(items: DNode[], q: URLSearchParams): Page {
	const limit = Math.min(200, Number(q.get('limit') ?? 100));
	const off = Number(q.get('cursor') ?? 0) || 0;
	const slice = items.slice(off, off + limit);
	return { items: slice.map(strip), next_cursor: off + limit < items.length ? String(off + limit) : null };
}
const trashedAncestor = (s: Store, n: DNode) => {
	for (let cur: string | null = n.parent_id; cur; cur = s.nodes.get(cur)?.parent_id ?? null) if (s.nodes.get(cur)?.trashed_at) return true;
	return false;
};
const live = (s: Store) => [...s.nodes.values()].filter((n) => !n.trashed_at && !trashedAncestor(s, n));
const mineOrShared = (s: Store, n: DNode) => n.owner_id === ME || s.shares.some((sh) => sh.grantee_id === ME && inSubtree(s, n.id, sh.node_id));

function shareInfo(s: Store, id: string): ShareInfo {
	const n = nodeOr404(s, id);
	const o = s.users.find((u) => u.id === n.owner_id)!;
	return {
		owner: { id: o.id, email: o.email, name: o.name, avatar_version: 0 },
		users: s.shares.filter((x) => x.node_id === id).map(({ node_id: _n, ...r }) => r),
		links: s.links.filter((l) => l.node_id === id && !l.revoked_at).map(({ node_id: _n, ...r }) => r)
	};
}

const used = (s: Store) => [...s.nodes.values()].filter((n) => n.owner_id === ME && n.kind === 'file').reduce((a, n) => a + n.size, 0);

// ---- request router --------------------------------------------------------------
export async function demoRequest<T>(method: string, rawPath: string, body?: any): Promise<T> {
	const s = init();
	const u = new URL(rawPath, 'https://demo.invalid');
	const path = u.pathname;
	const q = u.searchParams;
	const m = (re: RegExp) => path.match(re);
	let r: RegExpMatchArray | null;
	const ok = { ok: true };
	await Promise.resolve();

	// auth / account
	if (path === '/auth/status') return { setup_required: false, signup_allowed: false, version: 'démo' } as T;
	if (path === '/auth/login' || path === '/auth/signup' || path === '/auth/setup') return { user: s.users[0] } as T;
	if (path === '/auth/logout') return ok as T;
	if (path === '/me' && method === 'GET') {
		const me: Me = { user: s.users[0], used_bytes: used(s), quota_bytes: s.users[0].quota_bytes, disk: null, office: false, immich_url: s.users[0].immich_url, immich_default_url: null };
		return me as T;
	}
	if (path === '/me' && method === 'PATCH') {
		Object.assign(s.users[0], body ?? {});
		return { user: s.users[0] } as T;
	}
	if (path === '/me/password') return { user: s.users[0] } as T;
	if (path === '/me/avatar') return { user: s.users[0] } as T;

	// users
	if (path === '/users/lookup') {
		const t = (q.get('q') ?? '').toLowerCase();
		const items: UserBrief[] = s.users.filter((x) => x.id !== ME && (x.name + x.email).toLowerCase().includes(t)).map((x) => ({ id: x.id, email: x.email, name: x.name, avatar_version: 0 }));
		return { items } as T;
	}

	// listing
	if (path === '/nodes' && method === 'GET') {
		const view = q.get('view') ?? 'drive';
		const parent = q.get('parent');
		let items: DNode[];
		if (view === 'drive') items = live(s).filter((n) => (parent ? n.parent_id === parent : n.parent_id === null && n.owner_id === ME));
		else if (view === 'starred') items = live(s).filter((n) => n.starred && n.owner_id === ME);
		else if (view === 'shared') items = live(s).filter((n) => s.shares.some((sh) => sh.grantee_id === ME && sh.node_id === n.id));
		else if (view === 'trash') {
			const t = [...s.nodes.values()].filter((n) => n.trashed_at && n.owner_id === ME);
			return page(sortNodes(t, 'modified', 'desc'), q) as T;
		} else {
			const act = (n: DNode) => (n.last_opened_at && n.last_opened_at > n.updated_at ? n.last_opened_at : n.updated_at);
			items = live(s).filter((n) => n.kind === 'file' && mineOrShared(s, n)).sort((a, b) => act(b).localeCompare(act(a)));
			return page(applyFilters(items, q), q) as T;
		}
		return page(sortNodes(applyFilters(items, q), q.get('sort') ?? 'name', q.get('order') ?? 'asc'), q) as T;
	}
	if (path === '/nodes/folder' && method === 'POST') {
		const id = nid(s, 'f');
		const parent = body.parent_id ?? null;
		const ow = s.users[0];
		s.nodes.set(id, { id, kind: 'folder', name: String(body.name).trim() || 'Dossier', mime: null, size: 0, parent_id: parent, owner_id: ow.id, owner_name: ow.name, owner_avatar_version: 0, starred: false, trashed_at: null, created_at: new Date().toISOString(), updated_at: new Date().toISOString(), last_opened_at: null, description: null, thumb_state: null, role: 'owner' });
		return strip(s.nodes.get(id)!) as T;
	}
	if (path === '/nodes/file' && method === 'POST') {
		const id = nid(s, 'n');
		const ow = s.users[0];
		const name = String(body.name);
		s.nodes.set(id, { id, kind: 'file', name, mime: mimeOf(name), size: 0, parent_id: body.parent_id ?? null, owner_id: ow.id, owner_name: ow.name, owner_avatar_version: 0, starred: false, trashed_at: null, created_at: new Date().toISOString(), updated_at: new Date().toISOString(), last_opened_at: null, description: null, thumb_state: 'unsupported', role: 'owner', url: textUrl('') });
		return strip(s.nodes.get(id)!) as T;
	}
	if (path === '/nodes/batch' && method === 'POST') {
		const okIds: string[] = [];
		for (const id of body.ids as string[]) {
			const n = s.nodes.get(id);
			if (!n) continue;
			if (body.action === 'trash') n.trashed_at = new Date().toISOString();
			else if (body.action === 'restore') n.trashed_at = null;
			else if (body.action === 'star') n.starred = true;
			else if (body.action === 'unstar') n.starred = false;
			else if (body.action === 'move') n.parent_id = body.parent_id ?? null;
			else if (body.action === 'delete') dropSubtree(s, id);
			okIds.push(id);
		}
		return { ok: okIds, failed: [] } as T;
	}
	if (path === '/trash/empty' && method === 'POST') {
		const gone = [...s.nodes.values()].filter((n) => n.trashed_at && n.owner_id === ME);
		for (const n of gone) dropSubtree(s, n.id);
		return { deleted: gone.length } as T;
	}

	// one node
	if ((r = m(/^\/nodes\/([^/]+)$/))) {
		const n = nodeOr404(s, r[1]);
		if (method === 'GET') return strip(n) as T;
		if (method === 'PATCH') {
			if (body.name !== undefined && body.name !== null) n.name = String(body.name).trim() || n.name;
			if ('parent_id' in body) n.parent_id = body.parent_id;
			if (body.starred !== undefined && body.starred !== null) n.starred = !!body.starred;
			if ('description' in body) n.description = body.description;
			touch(n);
			return strip(n) as T;
		}
		if (method === 'DELETE') {
			dropSubtree(s, n.id);
			return ok as T;
		}
	}
	if ((r = m(/^\/nodes\/([^/]+)\/path$/))) {
		const crumbs: Crumb[] = [];
		for (let cur: DNode | undefined = nodeOr404(s, r[1]); cur; cur = cur.parent_id ? s.nodes.get(cur.parent_id) : undefined) crumbs.unshift({ id: cur.id, name: cur.name });
		return { items: crumbs } as T;
	}
	if ((r = m(/^\/nodes\/([^/]+)\/copy$/)) && method === 'POST') {
		const src = nodeOr404(s, r[1]);
		const id = nid(s, 'n');
		const now = new Date().toISOString();
		s.nodes.set(id, { ...src, id, name: String(body.name), owner_id: ME, owner_name: s.users[0].name, role: 'owner', starred: false, trashed_at: null, created_at: now, updated_at: now, last_opened_at: null });
		return strip(s.nodes.get(id)!) as T;
	}
	if ((r = m(/^\/nodes\/([^/]+)\/trash$/))) {
		nodeOr404(s, r[1]).trashed_at = new Date().toISOString();
		return ok as T;
	}
	if ((r = m(/^\/nodes\/([^/]+)\/restore$/))) {
		const n = nodeOr404(s, r[1]);
		n.trashed_at = null;
		return strip(n) as T;
	}
	if ((r = m(/^\/nodes\/([^/]+)\/open$/))) {
		nodeOr404(s, r[1]).last_opened_at = new Date().toISOString();
		return undefined as T;
	}
	if ((r = m(/^\/nodes\/([^/]+)\/preview$/))) return { state: 'unsupported' } as T;
	if ((r = m(/^\/nodes\/([^/]+)\/versions$/))) return { items: [], max_versions: 20 } as T;

	// search
	if (path === '/search/suggest') {
		const t = (q.get('q') ?? '').toLowerCase();
		const items: Suggestion[] = live(s)
			.filter((n) => mineOrShared(s, n) && n.name.toLowerCase().includes(t))
			.slice(0, 8)
			.map((n) => ({ id: n.id, kind: n.kind, name: n.name, mime: n.mime, parent_id: n.parent_id, parent_name: n.parent_id ? (s.nodes.get(n.parent_id)?.name ?? null) : null, thumb_state: n.thumb_state }));
		return { items } as T;
	}
	if (path === '/search') {
		const t = (q.get('q') ?? q.get('name') ?? q.get('words') ?? '').toLowerCase();
		const inFolder = q.get('in_folder');
		const items = live(s).filter((n) => mineOrShared(s, n) && (!t || n.name.toLowerCase().includes(t)) && (!inFolder || n.parent_id === inFolder));
		return page(sortNodes(applyFilters(items, q), 'name', 'asc'), q) as T;
	}

	// sharing
	if ((r = m(/^\/nodes\/([^/]+)\/shares$/))) {
		if (method === 'GET') return shareInfo(s, r[1]) as T;
		const target = s.users.find((x) => x.email.toLowerCase() === String(body.email).toLowerCase());
		if (!target) err(404, 'not_found', 'Aucun utilisateur avec cette adresse (essayez lucas@example.org)');
		const entry = { id: nid(s, 'sh'), node_id: r[1], grantee_id: target!.id, grantee_name: target!.name, grantee_email: target!.email, grantee_avatar_version: 0, role: body.role, created_at: new Date().toISOString() };
		s.shares.push(entry);
		const { node_id: _n, ...share } = entry;
		return { share } as T;
	}
	if ((r = m(/^\/shares\/([^/]+)$/))) {
		const i = s.shares.findIndex((x) => x.id === r![1]);
		if (i >= 0 && method === 'PATCH') s.shares[i].role = body.role;
		if (i >= 0 && method === 'DELETE') s.shares.splice(i, 1);
		return ok as T;
	}
	if ((r = m(/^\/nodes\/([^/]+)\/links$/))) {
		const link = { id: nid(s, 'lk'), node_id: r[1], role: 'viewer' as const, expires_at: body.expires_at ?? null, revoked_at: null, created_at: new Date().toISOString() };
		s.links.push(link);
		const { node_id: _n, ...l } = link;
		return { link: l, token: 'demo-' + link.id, path: '/s/demo-' + link.id } as T;
	}
	if ((r = m(/^\/links\/([^/]+)$/))) {
		const l = s.links.find((x) => x.id === r![1]);
		if (l) l.revoked_at = new Date().toISOString();
		return ok as T;
	}

	// comments
	if ((r = m(/^\/nodes\/([^/]+)\/comments$/))) {
		if (method === 'GET') return { items: s.comments.filter((c) => c.node_id === r![1]) } as T;
		const c: CommentItem = { id: nid(s, 'c'), node_id: r[1], author_id: ME, author_name: s.users[0].name, author_avatar_version: 0, body: String(body.body), created_at: new Date().toISOString(), edited_at: null, can_edit: true, can_delete: true };
		s.comments.push(c);
		return { comment: c } as T;
	}
	if ((r = m(/^\/comments\/([^/]+)$/))) {
		const i = s.comments.findIndex((x) => x.id === r![1]);
		if (i >= 0 && method === 'PATCH') {
			s.comments[i].body = String(body.body);
			s.comments[i].edited_at = new Date().toISOString();
			return { comment: s.comments[i] } as T;
		}
		if (i >= 0) s.comments.splice(i, 1);
		return ok as T;
	}

	// uploads
	if (path === '/uploads' && method === 'POST') {
		const id = nid(s, 'up');
		s.uploads.set(id, { parent_id: body.parent_id ?? null, name: String(body.name ?? 'fichier') });
		return { upload: { id, received: 0, total_size: body.size ?? 0 } } as T;
	}
	if ((r = m(/^\/uploads\/([^/]+)$/))) {
		s.uploads.delete(r[1]);
		return ok as T;
	}

	// administration (read-mostly, plausible figures)
	if (path === '/admin/overview') {
		const files = [...s.nodes.values()].filter((n) => n.kind === 'file').length;
		const o: AdminOverview = {
			version: 'démo', database: { ok: true, version: 'PostgreSQL 17' },
			counts: { users: s.users.length, files, folders: s.nodes.size - files, stored_bytes: used(s) }, disk: null, data_dir: '/data',
			limits: { max_file_bytes: 0, trash_days: 30, upload_ttl_hours: 48 },
			jobs: { stats: [{ kind: 'thumbnail', status: 'done', count: 12 }, { kind: 'checksum', status: 'done', count: 26 }], recent_failures: [] }
		};
		return o as T;
	}
	if (path === '/admin/users' && method === 'GET') {
		const items: AdminUserRow[] = s.users.map((x) => ({ user: x, used_bytes: x.id === ME ? used(s) : 1_480_000 }));
		return { items } as T;
	}
	if (path === '/admin/users') return { user: s.users[1] } as T;
	if (/^\/admin\/users\//.test(path)) return { user: s.users[1], ok: true } as T;
	if (path === '/admin/jobs/config') return { paused: [], limits: {}, kinds: ['thumbnail', 'checksum', 'purge_trash'] } as T;
	if (path === '/admin/backups') return { items: [], enabled: false, keep: 14 } as T;
	if (path === '/admin/settings') return { allow_signup: false, default_quota_bytes: null, immich_url: null } as T;
	if (path.startsWith('/admin/')) return { queued: false, requeued: 0 } as T;

	return err(404, 'not_found', 'Cette fonction n’est pas disponible dans la démo.');
}

// ---- URLs and uploads ------------------------------------------------------------
export function demoFileUrl(id: string, kind: 'content' | 'thumb'): string {
	const n = init().nodes.get(id);
	if (!n) return '';
	return kind === 'thumb' ? (n.thumb ?? n.url ?? '') : (n.url ?? '');
}

export function demoUploadChunk(uploadId: string, blob: Blob): { node: NodeItem; received: number } {
	const s = init();
	const up = s.uploads.get(uploadId) ?? err(404, 'not_found', 'Envoi inconnu');
	const id = nid(s, 'n');
	const ow = s.users[0];
	const mime = blob.type || mimeOf(up.name);
	const node: DNode = {
		id, kind: 'file', name: up.name, mime, size: blob.size, parent_id: up.parent_id, owner_id: ow.id, owner_name: ow.name, owner_avatar_version: 0,
		starred: false, trashed_at: null, created_at: new Date().toISOString(), updated_at: new Date().toISOString(), last_opened_at: null, description: null,
		thumb_state: mime.startsWith('image/') ? 'ready' : 'unsupported', role: 'owner', url: URL.createObjectURL(blob)
	};
	s.nodes.set(id, node);
	s.uploads.delete(uploadId);
	return { node: strip(node), received: blob.size };
}
