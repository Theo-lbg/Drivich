# Contribuer à Drivich

Drivich est un projet open source (AGPL-3.0-or-later). Les contributions sont bienvenues.

1. Ouvrez une issue pour discuter d'un changement important avant de coder.
2. Une branche par sujet ; la branche principale est protégée (pull request + CI verte).
3. Avant de proposer une PR :
   ```bash
   cargo fmt --all && cargo clippy --workspace --all-targets
   DATABASE_URL=postgres://… cargo test --workspace
   cd web && npm run check && npm run build
   BASE=http://127.0.0.1:2283 scripts/smoke.sh   # serveur lancé
   ```
4. Chaque nouveau fichier source commence par l'en-tête :
   `SPDX-License-Identifier: AGPL-3.0-or-later` et `Copyright (c) <année> <auteur>`.
5. En soumettant une PR, vous acceptez que votre contribution soit publiée sous la licence du projet.

Règles d'or : aucune synchronisation avec les fichiers locaux, peu de RAM/CPU, aucune autorisation décidée côtée client.
