---
paths:
  - "crates/drivich-server/**"
  - "crates/drivich-core/src/**"
  - "docs/**"
  - "web/src/lib/api*.ts"
  - "web/src/lib/demo/**"
---

# Conventions de l'API

- Routes sous `/api/v1`, JSON. Toute requête non-GET exige l'en-tête `X-Drivich-CSRF: 1` ; l'authentification se fait par cookie de session (extracteur `AuthUser`).
- Un handler ne contient pas de logique : il appelle une fonction de `drivich-core`, qui fait l'autorisation (`perms::authorize` / `authorize_parent`) **avant** toute lecture ou écriture.
- Codes : 400 requête invalide, 403 rôle insuffisant, **404 aucun accès** (jamais 403, pour ne pas révéler l'existence), 413 fichier trop gros, 429 trop de requêtes, 507 quota dépassé. Corps d'erreur : `{ "error": { "code", "message" } }`.
- Rôles : viewer < commenter < editor < owner. Un partage sur un dossier vaut pour tout son sous-arbre.
- **Une route nouvelle ou modifiée touche toujours** :
  1. `routes/*.rs` et `openapi.rs` (fonction `#[utoipa::path]` + liste `paths(...)` + schéma de la requête dans `components(schemas(...))`) ;
  2. `docs/openapi.json` : `UPDATE_OPENAPI=1 cargo test -p drivich-server --lib openapi` ;
  3. `web/src/lib/api-schema.d.ts` (`npm run gen:api`) et `docs/API.md` ;
  4. le client `web/src/lib/api.ts` et le faux serveur de la démo `web/src/lib/demo/mock-api.ts` ;
  5. un scénario Cucumber.
- Les types hand-écrits (`web/src/lib/types.ts`) doivent garder exactement les champs du schéma généré (`api-conformance.ts` le vérifie à la compilation).
