---
name: add-endpoint
description: Ajouter ou modifier une route de l'API Drivich (nouvelle action sur un fichier ou un dossier, nouveau paramètre, nouvelle ressource). À utiliser dès qu'une tâche touche `crates/drivich-server/src/routes/`, `openapi.rs` ou le client `web/src/lib/api.ts`.
---

# Ajouter ou modifier une route de l'API

Les conventions sont dans `.claude/rules/api-conventions.md` ; voici l'ordre de travail, qui évite d'oublier une des cinq places à tenir synchronisées.

1. **Core d'abord** : écrire la fonction dans `crates/drivich-core/src/` (`nodes.rs`, `uploads.rs`…). Elle fait l'autorisation avant tout accès (`perms::authorize` ou `authorize_parent`), vérifie le quota si elle crée des octets, et copie les octets plutôt que de partager un blob.
2. **Route** : handler fin dans `crates/drivich-server/src/routes/`, enregistré dans le `Router` du fichier.
3. **OpenAPI** : dans `crates/drivich-server/src/openapi.rs`, un schéma de requête (`#[derive(Serialize, ToSchema)]`), une fonction `#[utoipa::path(...)]` avec ses réponses (400/403/404/507 selon le cas), puis les deux listes `paths(...)` et `components(schemas(...))`.
4. **Régénérer** : `UPDATE_OPENAPI=1 cargo test -p drivich-server --lib openapi`, puis `docs/API.md` (tableau des routes) et `web/src/lib/api-schema.d.ts` (`cd web && npm run gen:api`).
5. **Tests** : un fichier `.feature` dans `crates/drivich-server/tests/features/` : cas nominal, rôles (lecteur, éditeur, sans accès), cas d'erreur (400), quota si des octets sont créés.
6. **Web** : méthode dans `web/src/lib/api.ts`, action dans `web/src/lib/actions.ts`, mêmes chemins dans `web/src/lib/demo/mock-api.ts`, textes dans les quatre langues.
7. **Vérifier** : `/check`.
