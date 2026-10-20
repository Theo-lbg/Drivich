# API Drivich (v1)

Base : `/api/v1`. JSON partout, sauf octets de fichiers. Authentification par cookie `drivich_session` (HttpOnly). **Toute requête modifiante** (POST/PUT/PATCH/DELETE) doit envoyer l'en-tête `X-Drivich-CSRF: 1`. Erreurs : `{"error":{"code","message"}}` avec 400/401/403/404/409/413/429/507.

## Authentification
| | |
|---|---|
| `GET /auth/status` | `{setup_required, signup_allowed, version}` |
| `POST /auth/setup` | crée l'administrateur (une seule fois) `{email,name,password}` |
| `POST /auth/login` · `/auth/logout` · `/auth/signup` | session ; limitation des tentatives (429) |
| `GET /me` · `PATCH /me` · `POST /me/password` | profil, usage/quota/disque (+ `immich_url` effective et `immich_default_url`), changement de mot de passe ; `PATCH /me {name?, immich_url?}` (http/https seulement, `null` pour effacer) |

`PUT /me/avatar` (corps = image JPEG/PNG/GIF/WebP, 5 Mo max, ré-encodée en JPEG 256×256) · `DELETE /me/avatar` · `GET /users/{id}/avatar?v=<version>` (connecté requis ; `avatar_version` = 0 si aucune photo).

## Éléments
| | |
|---|---|
| `GET /nodes?view=drive\|shared\|recent\|starred\|trash&parent=<id\|root>&sort=name\|modified\|created\|size&order=asc\|desc&type=&modified_from=&owner=me\|others&location=drive\|shared (Récents)&cursor=&limit=` | liste paginée `{items,next_cursor}` |
| `GET /nodes/{id}` · `GET /nodes/{id}/path` | élément, fil d'Ariane |
| `POST /nodes/folder` | `{parent_id,name}` |
| `PATCH /nodes/{id}` | `{name?, parent_id?, starred?, description?}` |
| `POST /nodes/{id}/trash` · `/restore` · `DELETE /nodes/{id}` · `POST /trash/empty` | corbeille |
| `POST /nodes/batch` | `{ids, action: trash\|restore\|star\|unstar\|move\|delete, parent_id?}` |
| `GET /nodes/{id}/download` · `/content` · `/thumbnail` · `POST /nodes/{id}/open` | octets (Range, ETag), aperçu inline, miniature, « récent » |
| `GET /nodes/{id}/zip` · `GET /nodes/zip?ids=a,b,c` · `GET /public/{token}/nodes/{id}/zip` | dossier ou sélection en `.zip` streamé (429 si trop d'archives en cours, 400 au-delà de 50 000 éléments) |
| `POST /nodes/{id}/preview` · `GET /nodes/{id}/preview` | aperçu PDF d'un document Office : `POST` lance la conversion une fois et renvoie `{state: pending\|ready\|failed\|unsupported}`, `GET` sert le PDF quand il est prêt (404 avant) |
| `GET /nodes/{id}/versions` · `GET …/versions/{v}/download` · `POST …/versions/{v}/restore` · `DELETE …/versions/{v}` | historique d'un fichier ; lecture : lecteur, restauration et suppression : éditeur |
| `GET\|POST /nodes/{id}/comments` · `PATCH\|DELETE /comments/{id}` | commentaires ; écrire : commentateur ou plus ; modifier : l'auteur ; supprimer : l'auteur ou le propriétaire |

## Téléversement reprenable
`POST /uploads {parent_id,name,size}` (ou `{replace_node_id,size}` pour une **nouvelle version** d'un fichier existant, rôle éditeur) → `{upload:{id,received,total_size}}` · `HEAD /uploads/{id}` (en-tête `Upload-Offset`) · `PATCH /uploads/{id}` (en-tête `Upload-Offset: N`, corps = octets ; 409 + `Upload-Offset` si décalage ; la réponse du dernier bloc contient `node`) · `DELETE /uploads/{id}`.

## Recherche
`GET /search/suggest?q=` (≤ 8) · `GET /search?q=&name=&words=&type=&owner=&modified_from=&modified_to=&location=drive\|shared\|starred\|trash&in_folder=&cursor=`.

## Partage
`GET|POST /nodes/{id}/shares` · `PATCH|DELETE /shares/{id}` · `POST /nodes/{id}/links` (le jeton n'est renvoyé qu'une fois) · `DELETE /links/{id}` · `GET /users/lookup?q=`.
Public (sans compte) : `GET /public/{token}` · `/public/{token}/nodes/{id}` · `/children` · `/download` · `/content` · `/thumbnail`.

## Administration (compte admin)
`GET /admin/overview` · `GET|POST /admin/users` · `PATCH /admin/users/{id}` · `POST /admin/users/{id}/password` · `POST /admin/jobs/retry` · `GET|PUT /admin/settings` (inscriptions, quota par défaut, `immich_url` par défaut).

`GET|PUT /admin/jobs/config` (pause, concurrence par type) · `GET|POST /admin/backups` · `GET /admin/backups/{name}/download` (dump `pg_dump` ; contient les empreintes de mots de passe, à protéger).

## Compte
`PATCH /me` accepte `name`, `immich_url` et `locale` (étiquette BCP 47, `null` = suivre le navigateur) · `PUT|DELETE /me/avatar` (image brute, 5 Mo) · `GET /users/{id}/avatar`.

## Schéma OpenAPI
La description complète et à jour est dans [openapi.json](openapi.json) (également servie sur `GET /api/v1/openapi.json`). Un test échoue si elle diverge du code : `UPDATE_OPENAPI=1 cargo test -p drivich-server openapi`. Les types du frontend se régénèrent avec `npm run gen:api` dans `web/` (`src/lib/api-conformance.ts` vérifie que les types écrits à la main listent les mêmes champs).

`GET /health` : sonde de vivacité (base comprise).
