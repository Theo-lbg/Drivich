---
description: Rejoue les contrôles de sécurité après un lot de fonctionnalités (cargo audit, npm audit, Trivy, smoke test)
---

À lancer après chaque lot de fonctionnalités. Annonce chaque étape avant de la lancer : certaines téléchargent des images ou des bases de vulnérabilités, donc demande confirmation avant de tirer quoi que ce soit.

1. **Dépendances Rust** : `cargo audit`, dans un conteneur `rust:latest` pour ne rien installer sur la machine (monter le dépôt en lecture seule).
2. **Dépendances npm** : `cd web && npm audit`.
3. **Images et fichiers** : Trivy sur le dépôt et sur les images construites, avec `--db-repository ghcr.io/aquasecurity/trivy-db:2`.
4. **Smoke test** : démarrer la pile (voir `README.md`, port 2290 et données dans un dossier temporaire, projet Compose `drivich-test`), puis `BASE=http://127.0.0.1:2290 scripts/smoke.sh` ; ensuite `docker compose -p drivich-test down` (sans `-v` : le hook le bloque, supprimer à la main le dossier de données temporaire) et le `.env` temporaire.
5. Lance aussi l'agent `security-auditor` sur les changements de la branche (`git diff main...`).

Résume : vulnérabilités trouvées (gravité, paquet, correctif disponible), contrôles non lancés et pourquoi. Ne corrige rien sans l'accord de l'utilisateur. $ARGUMENTS
