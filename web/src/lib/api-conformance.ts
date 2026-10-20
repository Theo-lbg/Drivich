// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

// Compile-time check: the hand-written UI types must list exactly the fields the
// generated OpenAPI schema (api-schema.d.ts, `npm run gen:api`) says the server
// returns, so a renamed or added field on the Rust side breaks `npm run check`.
import type { components } from './api-schema';
import type { NodeItem, User } from './types';

type Schemas = components['schemas'];
type Assert<_T extends true> = never;
type SameKeys<A, B> = [keyof A] extends [keyof B] ? ([keyof B] extends [keyof A] ? true : false) : false;

export type _UserMatches = Assert<SameKeys<User, Schemas['User']>>;
export type _NodeMatches = Assert<SameKeys<NodeItem, Schemas['Node']>>;
