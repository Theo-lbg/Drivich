# Sécurité

## Signaler une vulnérabilité

Merci de **ne pas** ouvrir d'issue publique pour une faille de sécurité.
Utilisez la fonction « Report a vulnerability » (avis de sécurité privés) de ce dépôt GitHub, ou contactez le mainteneur (`Theo-lbg`) en privé.

## Ce que Drivich fait pour vous protéger

- Mots de passe hachés avec **Argon2id** ; les jetons de session et de lien de partage ne sont stockés que sous forme d'empreinte SHA-256.
- Cookie de session `HttpOnly`, `SameSite=Lax` (et `Secure` derrière HTTPS : `DRIVICH_COOKIE_SECURE=true`).
- Défense CSRF : toute requête modifiante doit porter l'en-tête `X-Drivich-CSRF`.
- **Toutes** les autorisations sont vérifiées côté serveur (`crates/drivich-core/src/perms.rs`) ; un élément auquel vous n'avez pas accès répond « introuvable », jamais « interdit ».
- Les fichiers HTML/XML ne sont jamais servis comme tels (affichés en texte) ; le SVG est servi sous une CSP `sandbox`.
- Limitation des tentatives de connexion, noms de fichiers validés, aucun chemin disque dérivé d'un nom fourni par l'utilisateur.

## À faire de votre côté

- Servez Drivich en **HTTPS** (reverse proxy) et activez `DRIVICH_COOKIE_SECURE=true`.
- Gardez `.env` secret et sauvegardez `DB_DATA_LOCATION` (base) **et** `UPLOAD_LOCATION` (fichiers) ensemble.
- Mettez à jour régulièrement les images (`docker compose pull && docker compose up -d --build`).
