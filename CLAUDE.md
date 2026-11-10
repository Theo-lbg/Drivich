# Drivich

Alternative auto-hébergée à Google Drive, open source (AGPL-3.0-or-later, © 2026 Theo-lbg). Interface fidèle à Drive et **en français**, backend Rust léger, PostgreSQL pour les métadonnées, fichiers sur le disque. Détails : `README.md`, `docs/ARCHITECTURE.md`, `docs/API.md`.

## Structure

- `crates/drivich-core` : logique métier (`nodes`, `perms`, `uploads`, `versions`, `search`, `jobs`, `storage`…) et migrations SQL.
- `crates/drivich-server` : API axum (`src/routes/`), description OpenAPI (`src/openapi.rs`), tests d'acceptation Cucumber (`tests/features/*.feature`).
- `crates/drivich-worker` : tâches de fond (miniatures, empreintes, indexation du texte, aperçus Office).
- `web/` : SvelteKit (Svelte 5, mode runes), SPA servie par nginx. Traductions fr / en / es / de dans `web/src/lib/i18n/locales/`. Une démo sans serveur (GitHub Pages) tourne sur `web/src/lib/demo/mock-api.ts`.
- `docs/`, `compose/` + `docker-compose.yml`, `scripts/` (`smoke.sh`, `init-env.sh`).

## Règles non négociables

- **Les autorisations se décident uniquement côté serveur** (`perms::authorize`). Aucun accès → 404, jamais 403 (pas de fuite d'existence).
- Faible RAM/CPU : aucun fichier entier en mémoire (flux). Pas de synchronisation avec les fichiers locaux.
- Deux nœuds ne partagent **jamais** le même blob : la clé de document ONLYOFFICE est l'identifiant du blob. Copier un fichier = copier les octets.
- Chaque fichier source commence par `SPDX-License-Identifier: AGPL-3.0-or-later` puis `Copyright (c) 2026 Theo-lbg`.
- **Ne jamais committer ni pousser sans demande explicite.** `main` est protégée (PR + CI verte, fusion en squash). Branche : `feat|fix|chore|docs|test|refactor|style|hotfix|release/…`. Commit : `type(scope): description` avec type dans `feat fix docs style refactor test chore` (vérifié par `.github/workflows/git-checks.yml`). Le hook `.claude/hooks/validate-bash.sh` bloque les commits/pushs sur `main` et les force pushs.

## Commandes (celles de la CI)

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
DATABASE_URL=postgres://… cargo test --workspace      # PostgreSQL requis
cd web && npm run check && npm run check:i18n && npm run build
```

`/check` les enchaîne. Après un lot de fonctionnalités, `/security` rejoue les contrôles de sécurité (cargo audit, npm audit, Trivy, smoke test).

## Pièges déjà rencontrés

- **PostgreSQL de test** : `docker-compose.dev.yml` publie le port 55432, souvent déjà pris par la base d'un autre projet local. Vérifier `docker ps` et utiliser un autre port plutôt que de toucher à cette base. Voir `CLAUDE.local.md` s'il existe.
- **Cucumber** : les étapes sont typées (`Given`/`When`/`Then`) et une étape `Then` ne se place pas après un `Given`/`And` (« Step doesn't match any function »). Pour ne lancer que certains scénarios : `cargo test -p drivich-server --test features -- --name '<regex du nom>'` (`--input` ne trouve aucun fichier ici).
- **OpenAPI** : toute route nouvelle ou modifiée se déclare dans `openapi.rs` ; régénérer `docs/openapi.json` avec `UPDATE_OPENAPI=1 cargo test -p drivich-server --lib openapi`, puis mettre à jour `web/src/lib/api-schema.d.ts` (`npm run gen:api`, qui télécharge le générateur via `npx`) et `docs/API.md`. Le test CI échoue si `openapi.json` est périmé.
- **i18n** : `web/scripts/addkeys.py` retrie **tout** le fichier ; or `fr/en/es/de.json` ont des clés (`new.*`, `editor.*`, `demo.banner`) ajoutées en fin de fichier, donc le script déplace ces lignes et pollue le diff. Insérer les nouvelles clés à leur place alphabétique sans réordonner le reste.
- **Démo** : la démo route par hash ; utiliser `goto` de `$lib/nav` (pas celui de `$app/navigation`), et refléter tout nouvel endpoint dans `mock-api.ts`.
