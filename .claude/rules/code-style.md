---
paths:
  - "crates/**/*.rs"
  - "web/src/**/*.{ts,svelte}"
  - "web/src/lib/i18n/**"
---

# Style du code

- **En-tête de licence** sur tout nouveau fichier source, deux lignes : `SPDX-License-Identifier: AGPL-3.0-or-later` puis `Copyright (c) 2026 Theo-lbg` (en commentaire `//`, `<!-- -->`, `--` ou `#` selon le langage).
- **Rust** : `cargo fmt` et `cargo clippy -- -D warnings` doivent passer. Les erreurs métier passent par `CoreError` (`CoreError::bad(..)`, `NotFound`, `Forbidden`, `QuotaExceeded`…), jamais par un `unwrap` sur une donnée venue du client.
- **SQL** : toujours des paramètres liés (`.bind`). `format!` n'est admis que pour assembler des constantes (colonnes, jointures), jamais une valeur utilisateur.
- **Mémoire** : un fichier ou une miniature ne se charge jamais en entier ; on lit et on écrit en flux.
- **Web** : Svelte 5 en mode runes (`$state`, `$derived`, `$props`), indentation à la tabulation, aucun texte d'interface en dur : `t('clé')` avec la clé présente dans les quatre langues (`npm run check:i18n`). Le français sert de référence et le texte doit rester fidèle au vocabulaire de Drive.
- **Icônes** : uniquement celles listées dans la `glob` de `web/src/lib/icons.ts` (ajouter le nom à la liste si besoin).
- **Nouvelles clés de traduction** : `web/scripts/addkeys.py` retrie **tout** le fichier ; or `fr/en/es/de.json` ont des clés (`new.*`, `editor.*`, `demo.banner`) ajoutées en fin de fichier, que le script déplacerait en polluant le diff. Insérer les nouvelles clés à leur place alphabétique sans réordonner le reste, puis `npm run check:i18n`.
- **Navigation** : `goto` de `$lib/nav`, pas celui de `$app/navigation` (la démo route par hash). Pour remplacer l'entrée d'historique : `{ replace: true }` (`replaceState` est obsolète).
