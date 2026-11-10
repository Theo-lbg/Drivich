---
description: Rejoue en local les contrôles de la CI (fmt, clippy, tests Rust, svelte-check, i18n, build web)
allowed-tools: Bash(cargo fmt:*), Bash(cargo clippy:*), Bash(cargo test:*), Bash(npm run:*), Bash(docker ps:*)
---

Rejoue les contrôles de `.github/workflows/ci.yml`, dans cet ordre, en t'arrêtant au premier échec (montre la sortie utile, pas tout le journal) :

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace` : exige `DATABASE_URL`. S'il n'est pas défini, ne devine pas : lis `CLAUDE.local.md` pour la base de test de cette machine, vérifie avec `docker ps` qu'elle tourne, et ne touche à aucune base qui n'est pas celle de Drivich.
4. Dans `web/` : `npm run check`, `npm run check:i18n`, `npm run build`

Termine par un bilan court : ce qui passe, ce qui échoue, ce que tu n'as pas pu lancer et pourquoi. $ARGUMENTS
