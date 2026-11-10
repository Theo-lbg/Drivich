---
paths:
  - "crates/**/tests/**"
  - "**/*.feature"
---

# Tests

- Le comportement de l'API se teste en **Cucumber** : un fichier `.feature` dans `crates/drivich-server/tests/features/`, avec les étapes génériques déjà définies dans `tests/features.rs` (`Alice sends POST "/nodes/<A>/copy" with JSON:`, `the status is 200`, `the JSON at "/name" is "…"`, `Alice owns a file "a.txt" containing "x" saved as "A"`). Un nouveau scénario n'a le plus souvent besoin d'aucun code Rust.
- `<A>` est remplacé par l'identifiant sauvegardé sous ce nom ; `<run>` est unique par scénario (pour les e-mails).
- Les étapes sont typées : `the user Alice id is saved as "X"` est une étape **Then**, elle ne peut pas suivre un `Given`/`And`.
- Un fichier mis à la corbeille reste visible de son propriétaire : copier ou modifier un tel fichier donne **400**, pas 404.
- Lancer un sous-ensemble : `cargo test -p drivich-server --test features -- --name '<regex du nom du scénario>'`.
- Base : `DATABASE_URL` vers un PostgreSQL jetable (voir `CLAUDE.local.md`). Les tests laissent des utilisateurs dans la base : ne pas l'utiliser pour essayer l'interface (l'installation n'y est plus « à faire »), créer une autre base.
- Toute nouvelle branche de code côté Rust doit être couverte (la couverture est suivie à ~98 %, SonarCloud analyse chaque PR) : `cargo llvm-cov --workspace --html`.
- Chaque flux critique pour la sécurité (droits d'un lecteur, d'un éditeur, sans accès, quota) a son scénario.
