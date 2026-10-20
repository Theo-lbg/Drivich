# Architecture de Drivich

Drivich s'inspire de la **philosophie** d'Immich (services séparés, métadonnées en PostgreSQL, traitements lourds en tâches de fond, interface réactive), adaptée à un gestionnaire de fichiers.

```
navigateur ──► web (nginx : fichiers statiques SvelteKit + proxy /api, streaming)
                 │
                 ▼
              server (Rust, axum) ──► PostgreSQL  (utilisateurs, arbre de fichiers, partages, jobs)
                 │                        ▲
                 ▼                        │ file de tâches (SKIP LOCKED + LISTEN/NOTIFY)
        dossier UPLOAD_LOCATION ◄──── worker (Rust) : miniatures, empreintes, indexation, nettoyage
        files/ cache/ tmp/
```

| Immich | Drivich | Pourquoi |
|---|---|---|
| PostgreSQL + Redis (BullMQ) | PostgreSQL seul : table `jobs` (`FOR UPDATE SKIP LOCKED`, `LISTEN/NOTIFY`) | un service de moins, moins de RAM ; Redis restera ajoutable |
| service ML Python | — | inutile pour des fichiers |
| serveur NestJS | `drivich-server` (Rust/axum) + `drivich-worker` | CPU/RAM faibles, un binaire statique par service |
| web SvelteKit | web SvelteKit (SPA) servi par nginx | proxy en flux pour les gros fichiers |

## Séparation des responsabilités

- **Octets des fichiers** : disque, adressés par identifiant de blob (`files/ab/cd/<uuid>`), jamais par nom → renommer/déplacer ne touche pas au disque, aucun chemin ne peut sortir du dossier de données. Écriture atomique (fichier temporaire puis `rename`).
- **Métadonnées** : PostgreSQL (`nodes`, `blobs`, `shares`, `share_links`, `upload_sessions`, `jobs`, `settings`, `users`, `sessions`).
- **Tâches de fond** : file dans PostgreSQL, verrou à bail (un worker mort libère ses tâches), reprises avec délai exponentiel, déduplication, tâches périodiques.

## Points clés

- **Arbre** : `nodes.parent_id` ; `NULL` = racine de « Mon Drive ». Corbeille : le dossier supprimé et tout ce qu'il contient reçoivent le même `trash_root_id` ; seule la racine de corbeille est listée, la restauration remet exactement ce qui a été supprimé.
- **Permissions** (`perms.rs`) : propriétaire, ou partage sur le nœud ou un de ses ancêtres (CTE récursive). Rôle insuffisant → 403 ; aucun accès → 404 (pas de fuite d'existence). Éléments en corbeille visibles du seul propriétaire.
- **Téléversement reprenable** : `POST /uploads` → `PATCH` par blocs avec `Upload-Offset` → `HEAD` pour reprendre. La longueur du fichier `.part` sur disque fait foi ; finalisation atomique (rename + insertion `blobs`/`nodes` dans une transaction). Quota et taille maximale vérifiés avant d'écrire.
- **Téléchargement** : flux depuis le disque, `Range` (une plage), `ETag`, `If-None-Match`, `If-Range`. Les types dangereux (HTML, XML) sont servis en `text/plain`, le SVG sous CSP `sandbox`.
- **Pagination** : par curseur *keyset* (dossiers d'abord, puis tri, puis id) → coût constant même avec des centaines de milliers de fichiers ; recherche par décalage borné.
- **Recherche** (`search.rs`) : noms (sous-chaîne, index trigramme `pg_trgm`), contenu des fichiers texte (`tsvector`), filtres type/propriétaire/date/emplacement. Le périmètre de droits est appliqué **en SQL** (propriétaire ∪ partagés), jamais après coup.
- **Miniatures** : décodage borné (pixels et mémoire), exécuté sur des threads bloquants ; vidéos via `ffmpeg` si présent ; états `pending/ready/failed/unsupported` pour ne jamais réessayer indéfiniment.
- **Mémoire** : aucun fichier ni aucune miniature entière n'est chargé en mémoire (flux, blocs de 8 Mo côté client, 256 Ko côté serveur, listes paginées côté web).

## Limites connues et pistes

- Application mobile native (Flutter) : l'interface web est responsive et installable (PWA) en attendant. Extraction du texte des PDF et des documents Office pour la recherche. Édition de documents.
- Commentaires : pas de fils de réponses, de mentions ni de notifications ; lisibles par tout lecteur de l'élément.
- Versions : pas de nom ni d'auteur par version (seulement la date) ; l'aperçu Office n'est pas proposé via les liens publics.
- L'étiquette de rôle des résultats de recherche pour les éléments partagés est prudente (`viewer`) ; les actions sont de toute façon revérifiées côté serveur.
- Les favoris sont personnels au propriétaire ; « Récents » ne couvre que ses propres fichiers.
- Le déplacement n'est proposé que pour ses propres éléments.
- CSP du web : `script-src` autorise l'inline de SvelteKit ; un mode à empreintes est possible.
- Le schéma OpenAPI est généré par `utoipa` à partir de descriptions de chemins écrites à côté des routes (`openapi.rs`) ; un test garantit que `docs/openapi.json` est à jour, mais il ne vérifie pas que chaque route réelle y figure.
- Rejouer la recherche sur plusieurs millions de lignes mérite un `EXPLAIN` et éventuellement `SearchBackend` (Meilisearch/Tantivy/pgvector) : l'accès est isolé dans `search.rs`.

## Comparaison avec le dépôt d'Immich (audit)

Drivich **n'est pas une copie** de l'architecture d'Immich : il en reprend les principes et les leçons, adaptés à un gestionnaire de fichiers. Sources lues : le dépôt `immich-app/immich` (arborescence, `docker/docker-compose.yml`, `web/static`) et la documentation `docs.immich.app` (architecture, prérequis, proxy inverse, sauvegarde, intégrité du système, variables d'environnement).

### Structure du dépôt

| Immich | Drivich | Écart |
|---|---|---|
| monorepo : `server/`, `web/`, `mobile/`, `machine-learning/`, `cli`, `open-api/`, `docker/`, `docs/`, `e2e/`, `i18n/`, `design/`, `deployment/`, `.github/` | `crates/` (core, server, worker), `web/`, `server/` et `worker/` (Dockerfile), `compose/`, `docs/`, `scripts/`, `assets/`, `.github/` | pas de mobile, de CLI, de service ML (hors périmètre) ; `scripts/smoke.sh` tient lieu d'`e2e/` |
| un Dockerfile par service dans le dossier du service | idem (`server/`, `worker/`, `web/`) | aligné |
| `docker/docker-compose.yml` + `example.env`, un seul fichier | `docker-compose.yml` central qui `include` deux stacks + `.env.example` | **choix de l'utilisateur** (deux stacks) ; impose Compose ≥ 2.20 (documenté) |
| images publiées sur ghcr, compose « release » | `release.yml` publie `ghcr.io/theo-lbg/drivich-{server,worker,web}` (amd64 + arm64) sur étiquette `v*`, avec `deployment/docker-compose.release.yml` | **ajouté** (rien n'est publié tant qu'aucune étiquette n'est poussée) |
| `renovate.json`, `CODEOWNERS`, modèles d'issues/PR, `.editorconfig`, `.gitattributes` | `.github/dependabot.yml`, `CODEOWNERS`, modèles, `.editorconfig`, `.gitattributes` | **ajoutés par cet audit** |

### Services et données

| Immich | Drivich |
|---|---|
| `immich-server` (API) + `immich-microservices` (tâches, ne traite pas l'API ni les crons) | `drivich-server` (API seule) + `drivich-worker` (tâches **et** planification des tâches périodiques) |
| PostgreSQL + Redis/BullMQ | PostgreSQL seul (file `jobs`, `SKIP LOCKED`, `LISTEN/NOTIFY`) |
| NestJS + Kysely (constructeur de requêtes), architecture « hexagonale » : `repositories` (technique) / `services` (métier) | Rust/axum + sqlx ; **écart** : le SQL est dans les mêmes modules que la logique (`drivich-core`), sans couche `repositories` séparée |
| schéma OpenAPI généré depuis les DTO, clients générés | **repris** : `utoipa` → `docs/openapi.json` (test d'actualité) → `openapi-typescript` ; les types du frontend restent écrits à la main mais un contrôle de compilation compare leurs champs au schéma |
| migrations de schéma versionnées | idem (`sqlx`, verrou consultatif ; `build.rs` force la recompilation quand une migration est ajoutée) |
| photos stockées sous `upload/`, `library/`, `thumbs/`, `encoded-video/`, `profile/`, `backups/` | `files/` (identifiants de blobs), `cache/thumbs/`, `tmp/uploads/`, `avatars/` |
| sauvegardes automatiques de la base (quotidiennes, 14 conservées) | **repris** : le worker lance `pg_dump` (format custom) chaque jour dans `backups/`, conserve les 14 derniers, liste et téléchargement dans l'administration ; la restauration reste manuelle (`pg_restore`) |
| page « Job Queues » (pause, concurrence par file) | **repris** : pause et limite de concurrence par type de tâche (administration), en plus de la relance des échecs |
| interface traduite (`i18n/`) | **repris** : fr (défaut), en, es, de ; JSON par langue, repli sur l'anglais, préférence par utilisateur, contrôle `npm run check:i18n` |

### Pièges connus d'Immich et traitement dans Drivich

| Piège (documentation Immich) | Traitement |
|---|---|
| volume non monté ou droits incorrects : une bibliothèque vide est créée en silence → **vérification de montage** avec fichiers `.immich` (`IMMICH_IGNORE_MOUNT_CHECK_ERRORS`) | **repris** : marqueurs `.drivich`, refus de démarrer si la base connaît des fichiers mais que `files/` est vide, `DRIVICH_IGNORE_MOUNT_CHECK_ERRORS` ; testé |
| base de données sur un partage réseau ou NTFS/FAT : interdit | avertissement dans le README et `.env.example` |
| Postgres exige beaucoup de mémoire partagée | `shm_size: 128mb` (comme Immich) ; limite mémoire de la base documentée |
| proxy inverse : corps de requête illimité, pas de tamponnage des envois, délais longs, en-têtes `X-Forwarded-*` | `client_max_body_size 0`, `proxy_request_buffering off`, délais d'une heure, en-têtes transmis (nginx du conteneur `web`) |
| les sauvegardes de base ne contiennent pas les fichiers ; ordre base puis fichiers ; vignettes reconstruisibles | documenté (README) |
| changement de version majeure de PostgreSQL | avertissement ; image épinglée par empreinte |
| mot de passe de base avec caractères spéciaux (URL de connexion) | `init-env.sh` génère du `A-Za-z0-9` ; consigne dans `.env.example` |
| fuseau horaire des journaux et des tâches | variable `TZ` transmise à tous les conteneurs |
| anciens `docker-compose` v1 | prérequis explicite : `docker compose` (plugin) ≥ 2.20 |
| WebSocket pour les notifications temps réel | non utilisé (l'interface se rafraîchit par requêtes) |

### Ce qui reste, et ce qui a été écarté volontairement

- **Séparation `repositories` / `services`** : *non faite, par choix*. Elle sert à Immich pour tester sans base et pour changer de moteur ; Drivich n'a qu'un moteur (PostgreSQL, dont `pg_trgm` et `tsvector` sont indispensables) et ses tests d'intégration passent par une vraie base. Découper aujourd'hui ajouterait du code et un risque de régression sans bénéfice mesurable ; à reconsidérer si un second moteur ou des tests unitaires isolés deviennent nécessaires.
- Restauration guidée d'un dump depuis l'interface (aujourd'hui `pg_restore` à la main).
- Vérifier par un test que chaque route réelle est décrite dans le schéma OpenAPI.

## Contrôle de sécurité (2026-10-20)

Fait : `npm audit` (0), RustSec via `cargo audit` (aucune vulnérabilité compilée : `rsa` n'est que dans `Cargo.lock` via sqlx-mysql, non utilisé ; `paste` est un macro de build non maintenu), Trivy sur les images (0 HIGH/CRITICAL corrigeable après mise à jour de la base nginx et `apt-get upgrade`), et confrontation avec des failles publiées d'Immich (OAuth `state`, mot de passe en URL, élévation de clé d'API, XSS via métadonnées, accès hors périmètre via lien partagé) : Drivich n'a ni OAuth ni clés d'API ; les noms sont échappés par Svelte ; les liens publics sont limités au sous-arbre et refusent révoqué/expiré/corbeille ; les jetons de lien ne sont plus écrits dans les journaux.
Pas fait : audit indépendant, test d'intrusion, fuzzing. À refaire à chaque mise à jour (`dependabot` + CI).

### Zip, versions, commentaires, aperçu Office : choix de conception et de sécurité

- **Zip** : `drivich_core::archive` liste l'arbre (récursion SQL bornée à 64 niveaux et 50 000 entrées, éléments à la corbeille exclus) après autorisation de chaque racine ; les noms sont assainis (`/`, `\`, contrôles → `_`) et dédoublonnés sans tenir compte de la casse. Un fil bloquant écrit l'archive (deflate niveau 1, zip64 au-delà de 4 Gio) dans un canal borné : mémoire constante et contre-pression si le client est lent ; si le client part, l'écriture s'arrête. Nombre d'archives simultanées limité par un sémaphore.
- **Versions** : le nœud pointe toujours sur le blob courant ; remplacer un fichier archive l'ancien blob dans `node_versions` (transaction, verrou de ligne). Les blobs ne sont supprimés que s'il n'est référencé ni par un nœud ni par une version ; la suppression définitive, le vidage de la corbeille, la purge et le ramasse-miettes en tiennent compte (testé). Le quota compte les versions.
- **Commentaires** : texte brut (échappé par Svelte, jamais interprété), 4 000 caractères, 1 000 par élément ; accès décidé par `perms::authorize`, donc un tiers reçoit 404.
- **Aperçu Office** : LibreOffice tourne **uniquement dans le worker** (utilisateur non root, réseau `internal` dans les compose fournis, mémoire limitée, un document à la fois, 180 s maximum, entrée ≤ 100 Mio, sortie ≤ 300 Mio, résultat vérifié `%PDF-`). L'extension du fichier d'entrée vient du type MIME, jamais du nom fourni. Un échec est définitif pour ce contenu (pas de nouvelle tentative automatique). Les formats Office restent un format d'analyse risqué : ne branchez pas le worker sur un réseau sortant et gardez `WITH_OFFICE=false` si vous n'en avez pas besoin.
- **PWA** : le service worker ne met en cache que la sortie de build ; `/api/` n'est jamais intercepté.

Contrôle du 2026-10-20 (après ces fonctions) : `npm audit` 0 ; RustSec : `zip` 7.4 avait des avis (extraction, non utilisée ici) → passé en 9.x ; reste `rsa` (présent uniquement dans `Cargo.lock` via sqlx-mysql, non compilé) et `paste` (macro de build non maintenue) ; Trivy sur les trois images : 0 HIGH/CRITICAL corrigeable ; test réel d'un .docx converti en PDF dans le worker, d'un dossier de 600 Mo en .zip (≈ 32 Mio de RAM côté serveur), et smoke test 99/99.
