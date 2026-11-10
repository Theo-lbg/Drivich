# Drivich

Alternative auto-hébergée à Google Drive, open source (AGPL-3.0-or-later, © 2026 Theo-lbg) : interface fidèle à Drive et **en français**, backend Rust léger, PostgreSQL, fichiers sur le disque. Voir `README.md`, `docs/ARCHITECTURE.md` (limites connues et pistes) et `docs/API.md`.

## Structure

- `crates/drivich-core` (logique métier, autorisations, migrations SQL), `drivich-server` (API axum, `openapi.rs`, tests Cucumber), `drivich-worker` (tâches de fond).
- `web/` : SvelteKit, Svelte 5 en mode runes ; traductions fr / en / es / de ; la démo GitHub Pages tourne sur `web/src/lib/demo/mock-api.ts`.
- `docs/`, `compose/` + `docker-compose.yml`, `scripts/` (`smoke.sh`, `init-env.sh`).

## Règles non négociables

- **Les autorisations se décident uniquement côté serveur** (`perms::authorize`). Aucun accès → 404, jamais 403.
- Faible RAM/CPU : aucun fichier entier en mémoire. Pas de synchronisation avec les fichiers locaux.
- Deux nœuds ne partagent **jamais** un blob (la clé de document ONLYOFFICE est son identifiant) : copier un fichier = copier les octets.
- Tout fichier source commence par `SPDX-License-Identifier: AGPL-3.0-or-later` puis `Copyright (c) 2026 Theo-lbg`.
- **Ne jamais committer ni pousser sans demande explicite.** `main` est protégée (PR + CI verte, squash). Branche : `feat|fix|chore|docs|test|refactor|style|hotfix|release/…`. Commit : `type(scope): description`, type parmi `feat fix docs style refactor test chore` (vérifié par `git-checks.yml`). Le hook `.claude/hooks/validate-bash.sh` bloque commits et pushs sur `main` ainsi que les force pushs.

## Commandes (celles de la CI)

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
DATABASE_URL=postgres://… cargo test --workspace      # PostgreSQL jetable : voir .claude/rules/testing.md
cd web && npm run check && npm run check:i18n && npm run build
```

`/check` les enchaîne ; `/security` rejoue les contrôles de sécurité après un lot de fonctionnalités.

## Où sont les détails

- `.claude/rules/` (chargées selon les fichiers touchés) : `code-style`, `api-conventions`, `testing`.
- Ajouter ou modifier une route : skill `add-endpoint`. Relecture : agents `code-reviewer` et `security-auditor`.
- Notes propres à une machine : `CLAUDE.local.md` (non versionné).
