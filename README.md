<p align="center"><img src="assets/logo-icon-256.png" alt="Drivich" width="120"></p>

# Drivich

**Votre Drive, sur votre serveur.** Drivich est une alternative open source, légère et auto-hébergeable à Google Drive : une application web fidèle à l'esprit de Drive, un backend **Rust** performant, PostgreSQL pour les métadonnées, et vos fichiers sur votre propre disque.

> Copyright © 2026 Theo-lbg — licence [AGPL-3.0-or-later](LICENSE). Projet indépendant, sans lien avec Google ni Immich (voir [NOTICE](NOTICE)).

## Ce qui fonctionne aujourd'hui

| | |
|---|---|
| **Fichiers** | dossiers, téléversement (bouton, glisser-déposer, dossiers entiers), **reprise des téléversements interrompus**, téléchargement avec `Range`/`ETag`, renommer, déplacer (y compris par glisser-déposer), corbeille (30 j), favoris, récents |
| **Immich** | adresse de votre Immich réglable dans Paramètres (personnelle ou par défaut du serveur) : icône dans le menu des applications (en haut à droite) et dans la barre latérale droite, qui l'ouvre dans un nouvel onglet ; l'icône affichée est celle que votre propre Immich sert (rien d'Immich n'est embarqué), avec une icône générique en repli |
| **Photo de profil** | chaque utilisateur peut ajouter, changer ou supprimer sa photo (Paramètres › Compte) ; elle est recadrée en carré, redimensionnée et ré-encodée côté serveur (métadonnées EXIF supprimées), et s'affiche dans la barre du haut, les listes, le partage et l'administration |
| **Apparence** | thème **clair, sombre ou automatique** (suit les réglages de l’appareil), au choix dans le menu du compte ou dans Paramètres › Compte |
| **Visualisation** | liste et grille, tri, filtres (type, propriétaire, date), sélection multiple, menu contextuel, raccourcis clavier, chargement progressif (pagination par curseur), miniatures générées en arrière-plan (images, vidéos), visionneuse intégrée : images, vidéos, audio, PDF, texte et code |
| **Récents** | regroupés par date (Aujourd’hui, Hier, Cette semaine…) avec les filtres Type, Contacts, Date de modification et Source ; incluent les fichiers partagés avec vous |
| **Recherche** | suggestions pendant la frappe, page de résultats avec filtres, **recherche avancée**, recherche dans le **contenu** des fichiers texte |
| **Dossiers en .zip** | un dossier ou une sélection se télécharge en **.zip généré à la volée** (flux : mémoire constante, testé sur 600 Mo à ~32 Mo de RAM), y compris depuis un lien public ; au plus `DRIVICH_MAX_ZIP_STREAMS` archives simultanées |
| **Versions** | « Importer une nouvelle version » sur un fichier : l'ancien contenu est conservé (`DRIVICH_MAX_VERSIONS`, 20 par défaut, comptées dans le quota), consultable, téléchargeable, restaurable (rien n'est perdu : le contenu remplacé devient une version) ou supprimable |
| **Commentaires** | sur les fichiers et dossiers, dans le panneau Détails et la visionneuse ; rôle commentateur ou plus pour écrire, l'auteur modifie, l'auteur ou le propriétaire supprime ; jamais exposés par les liens publics |
| **Aperçu Office** | Word, Excel, PowerPoint, OpenDocument et RTF s'ouvrent dans la visionneuse : le worker les convertit en PDF avec LibreOffice à la première ouverture (un document à la fois, délai limité, sans accès réseau) ; `WITH_OFFICE=false` retire LibreOffice de l'image (~400 Mo de moins) |
| **Édition Office** | option **ONLYOFFICE** : Word, Excel, PowerPoint et OpenDocument s'ouvrent dans un éditeur complet dans le navigateur (édition à plusieurs, enregistrement automatique en **nouvelle version** du fichier) ; lecteurs en lecture seule, éditeurs en écriture. Activation : voir « Édition des documents » ci-dessous |
| **Mobile** | interface adaptée aux petits écrans et **installable** (manifeste + service worker qui ne met en cache que l'interface, jamais l'API ni vos fichiers) |
| **Partage** | avec des utilisateurs (lecteur / commentateur / éditeur, hérité par les sous-dossiers), **liens publics** (expiration, révocation), « Partagés avec moi » |
| **Langues** | interface en **français** (par défaut), English, Español et Deutsch ; choix par utilisateur dans Paramètres › Compte (ou « automatique » = langue du navigateur), aussi sur les pages de connexion ; ajouter une langue = un fichier JSON dans `web/src/lib/i18n/locales/` |
| **Administration** | comptes, quotas, inscriptions, état du serveur, de la base et des tâches ; **pause et concurrence par type de tâche** ; **sauvegardes automatiques de la base** (dump quotidien, 14 conservés, téléchargeables) |
| **Sécurité** | Argon2id, sessions révocables, CSRF, autorisations **uniquement côté serveur**, aucune synchronisation avec vos fichiers locaux |

Limites connues (voir [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)) : pas d'application mobile native (l'interface web est adaptée aux téléphones et **installable** comme application, PWA), l'édition de documents demande le conteneur ONLYOFFICE (~2-3 Go de RAM), pas encore d'extraction du texte des PDF et des documents Office pour la recherche ; l'API est décrite par un schéma **OpenAPI** généré ([docs/openapi.json](docs/openapi.json), servi aussi sur `/api/v1/openapi.json`) en plus de [docs/API.md](docs/API.md).

## Prérequis

- **Docker Engine avec le plugin Compose ≥ 2.20** (le fichier central utilise `include`) ; la commande `docker compose` (pas l'ancien `docker-compose`).
- Un système de fichiers Unix (EXT4, ZFS, APFS…) avec droits utilisateur/groupe pour `UPLOAD_LOCATION` et `DB_DATA_LOCATION`. **Jamais la base sur un partage réseau** (NFS/SMB), ni sur NTFS/FAT (sous Windows, utilisez un volume Docker pour la base).
- Environ 1 Go de RAM libre pour la stack au repos ; la base est plafonnée à 512 Mo par défaut (`DRIVICH_DB_MEM`), à augmenter pour de gros volumes.

## Installation (Docker)

Deux façons : depuis une **release** (images déjà construites, rien à compiler : recommandé pour un serveur ou un mini PC) ou depuis les **sources** (la machine construit elle-même les images). Ouvrez ensuite <http://localhost:2283> (ou `http://<adresse-du-serveur>:2283` depuis une autre machine) : la première visite crée le compte **administrateur**.

- Seul le port du web est publié ; la base n'est joignable que par l'API et le worker.
- Données, avec les mêmes variables qu'Immich : `UPLOAD_LOCATION` (défaut `./library` : fichiers et miniatures) et `DB_DATA_LOCATION` (défaut `./postgres` : base). **Sauvegardez les deux ensemble.** `init-env.sh` crée ces dossiers et règle `DRIVICH_UID/GID` sur votre utilisateur pour que les conteneurs puissent y écrire.
- Derrière HTTPS : mettez `DRIVICH_COOKIE_SECURE=true` dans `.env`.
- Mémoire visée au repos : quelques dizaines de Mo pour le web et l'API, la base selon son usage ; limites réglables dans `.env`.

### Depuis une release (sans compilation)

```bash
mkdir drivich && cd drivich
curl -fL -O https://github.com/Theo-lbg/Drivich/releases/latest/download/docker-compose.yml \
         -O https://github.com/Theo-lbg/Drivich/releases/latest/download/example.env \
         -O https://github.com/Theo-lbg/Drivich/releases/latest/download/init-env.sh
bash init-env.sh       # crée .env (mot de passe aléatoire, votre UID/GID), library/ et postgres/
docker compose up -d
```

- Les images `ghcr.io/theo-lbg/drivich-{server,worker,web}` existent pour amd64 et arm64 ; le `docker-compose.yml` d'une release est épinglé à sa version.
- Pour une version précise, remplacez `latest/download` par `download/v0.1.0` (par exemple) dans les adresses.
- Vos réglages vivent dans `.env` : ne modifiez pas `docker-compose.yml`, il est remplacé à chaque mise à jour.

### Depuis les sources

```bash
git clone https://github.com/Theo-lbg/Drivich.git && cd Drivich   # adaptez l'URL de votre dépôt
scripts/init-env.sh              # crée .env avec un mot de passe de base aléatoire
docker compose up -d --build
```

- Une image par service : `server/Dockerfile`, `worker/Dockerfile`, `web/Dockerfile` ; PostgreSQL utilise l'image officielle.
- `docker-compose.yml` assemble les deux stacks : [`compose/backend.yml`](compose/backend.yml) (base, API, worker) et [`compose/frontend.yml`](compose/frontend.yml) (web).
- La compilation (Rust et interface web) demande du temps et de la mémoire : sur une petite machine, préférez une release.

### Édition des documents (ONLYOFFICE, optionnel)

Pour modifier les .docx / .xlsx / .pptx dans le navigateur, Drivich pilote un
[ONLYOFFICE Document Server](https://github.com/ONLYOFFICE/DocumentServer)
(AGPL, ~2-3 Go de RAM) lancé dans le même compose. Dans `.env` :

```bash
DRIVICH_ONLYOFFICE_JWT_SECRET=<40 caractères aléatoires>   # généré par init-env.sh pour un nouveau .env
COMPOSE_PROFILES=office
```

puis `docker compose up -d`. L'éditeur est servi par le conteneur web sous
`/onlyoffice/` (même origine que Drivich). Derrière votre propre reverse proxy,
autorisez les **WebSockets** et transmettez `X-Forwarded-Proto` / `X-Forwarded-Host`.
Sans ce conteneur, les documents Office restent affichés en PDF (LibreOffice).
Pour générer un secret dans un `.env` existant :

```bash
echo "DRIVICH_ONLYOFFICE_JWT_SECRET=$(head -c 48 /dev/urandom | base64 | tr -dc 'A-Za-z0-9' | head -c 40)" >> .env
```

### Sécurité du stockage (vérification des volumes)

Au démarrage, le serveur et le worker écrivent puis relisent un fichier `.drivich` dans chaque dossier de données (comme la vérification de montage d'Immich) : cela détecte les droits incorrects. Surtout, si la base connaît des fichiers mais que `library/files` est vide et sans marqueur, c'est presque sûrement qu'un disque n'est **pas monté** : Drivich refuse alors de démarrer au lieu de créer une seconde bibliothèque vide. Contournement (à vos risques) : `DRIVICH_IGNORE_MOUNT_CHECK_ERRORS=true`.

### Mise à jour et sauvegarde

```bash
# Les migrations s'appliquent au démarrage, dans les deux cas.
# Release : récupérez le docker-compose.yml de la nouvelle version (vos réglages restent dans .env)
curl -fLO https://github.com/Theo-lbg/Drivich/releases/latest/download/docker-compose.yml && docker compose pull && docker compose up -d
# Sources
git pull && docker compose up -d --build
# Attention : changer de version MAJEURE de PostgreSQL (17 → 18…) exige un dump puis une restauration, pas un simple changement d'image.
docker compose exec database pg_dump -U drivich drivich > drivich.sql   # base
tar czf drivich-library.tgz library                                         # fichiers
```

À sauvegarder **ensemble** : la base (`pg_dump`) et `library/files` + `library/avatars`. `library/cache` (miniatures) se reconstruit seul ; `library/tmp` (téléversements en cours) peut être ignoré. Sauvegardez la base d'abord, puis les fichiers, ou arrêtez le serveur pendant la sauvegarde : sinon la base restaurée peut référencer des fichiers absents de la copie. La base ne contient **que** les métadonnées, jamais le contenu des fichiers, et les fichiers sont stockés sous des identifiants (`files/ab/cd/<uuid>`) : sans la base, ils ne sont pas lisibles par nom.

### Images de base épinglées

Comme Immich, les images de base sont épinglées par empreinte (`image:tag@sha256:…`) : un build est reproductible et ne change pas sans que vous le décidiez. Pour les mettre à jour :

```bash
docker buildx imagetools inspect postgres:17-alpine | grep -m1 Digest   # nouvelle empreinte
# remplacez-la dans compose/backend.yml, server/Dockerfile, worker/Dockerfile, web/Dockerfile et .github/workflows/ci.yml
```

## Développement

Prérequis : Rust ≥ 1.85, Node ≥ 22, Docker (pour PostgreSQL).

```bash
docker compose -f docker-compose.dev.yml up -d        # PostgreSQL seul, sur 127.0.0.1:55432 (jetable)
export DATABASE_URL=postgres://drivich:dev-only@127.0.0.1:55432/drivich
cargo run -p drivich-server &      # API sur :2283
cargo run -p drivich-worker &      # tâches de fond
cd web && npm ci && npm run dev    # interface sur :5173 (proxy /api → :2283)
```

Tests : `cargo test --workspace` (35 tests dont des tests d'intégration sur PostgreSQL), `scripts/smoke.sh` (61 vérifications de bout en bout sur une API lancée), `cd web && npm run check`.

## Organisation du dépôt

```
crates/drivich-core     logique métier : permissions, nœuds, upload, recherche, jobs, miniatures
crates/drivich-server   API HTTP (axum)
crates/drivich-worker   file de tâches (miniatures, empreintes, indexation, nettoyage)
web/                    interface SvelteKit (SPA) + nginx
server/ worker/ web/    Dockerfile de chaque service
compose/                stacks backend et frontend
docs/                   architecture et API
scripts/                init-env.sh, smoke.sh
```

## Licence

AGPL-3.0-or-later. Si vous modifiez Drivich et le proposez comme service réseau, vous devez publier vos modifications sous la même licence. Voir [LICENSE](LICENSE) et [NOTICE](NOTICE).
