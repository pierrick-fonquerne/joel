# Coffre patrimoine (Egide) : exploitation

Egide garde la clé qui enveloppe la clé de données du module patrimoine. Tant qu'Egide est
scellé, Joel fonctionne mais les routes `/api/wealth/*` répondent `503 wealth_vault_sealed`.

## Contraintes

- Joel api doit tourner en une seule instance : le déverrouillage du coffre n'est protégé
  contre la concurrence que dans le processus. Avec deux réplicas, chacun pourrait créer une
  clé de données différente sur une table vide.

## Mise en place (une seule fois)

Compose refuse de démarrer la pile entière tant que `deploy/secrets/egide_token` n'existe pas.
Avant l'étape 6, ne démarrer que `egide` et `postgres`, jamais un `up -d` global.

1. Démarrer Egide : `docker compose -f deploy/compose.prod.yml up -d egide postgres`
2. Initialiser : `docker compose -f deploy/compose.prod.yml exec egide egide operator init`.
   Ranger les parts Shamir et le token root hors du VPS (gestionnaire de mots de passe, une
   part par emplacement). Le token root n'est affiché qu'une fois.
3. Desceller : `docker compose -f deploy/compose.prod.yml exec egide egide operator unseal`,
   autant de fois que le seuil de parts l'exige.
4. Lire le token root sans le laisser dans l'historique du shell : `read -rs EGIDE_ROOT_TOKEN`
   (coller le token, Entrée). L'image Egide n'a pas curl : les appels HTTP passent par un
   conteneur curl jetable sur le réseau interne `data`. Trouver le vrai nom du réseau (le
   préfixe est le nom du projet compose) : `docker network ls | grep data`, puis le mettre
   dans `DATA_NETWORK` (par exemple `export DATA_NETWORK=joel_data`). Créer la clé Transit :
   `docker run --rm --network "$DATA_NETWORK" curlimages/curl:8.10.1 -s -X POST http://egide:8200/v1/transit/keys -H "Authorization: Bearer $EGIDE_ROOT_TOKEN" -H "Content-Type: application/json" -d '{"name":"joel-wealth"}'`
   Le shell de l'hôte développe `$EGIDE_ROOT_TOKEN` avant de lancer le conteneur.
5. Créer le token de service de Joel, avec le même token root :
   `docker run --rm --network "$DATA_NETWORK" curlimages/curl:8.10.1 -s -X POST http://egide:8200/v1/auth/service-tokens -H "Authorization: Bearer $EGIDE_ROOT_TOKEN" -H "Content-Type: application/json" -d '{"service_name":"joel-api"}'`
   Le token `egst_...` n'est affiché qu'une fois : le copier tout de suite. Ensuite, effacer
   le token root de l'environnement : `unset EGIDE_ROOT_TOKEN`.
6. Écrire le token dans `deploy/secrets/egide_token`, puis régler les droits :
   `mkdir -p deploy/secrets`, écrire le fichier, puis
   `sudo chown 65532:65532 deploy/secrets/egide_token && chmod 400 deploy/secrets/egide_token`.
   Pourquoi : l'api tourne en `USER nonroot` (uid 65532). Un fichier en `600` appartenant à
   l'utilisateur de déploiement serait illisible dans le conteneur. Le fichier ne doit jamais
   être commité.
7. Démarrer toute la pile : `docker compose -f deploy/compose.prod.yml up -d`. Au premier
   démarrage, Joel génère la clé de données et enregistre sa version enveloppée.

## Après chaque redémarrage du VPS

Egide redémarre scellé. Lancer l'étape 3. Les routes patrimoine reviennent seules dans les
30 secondes, sans redémarrer Joel.

## Rotation de la clé

1. Faire tourner la clé maîtresse avec le token root (relu avec `read -rs EGIDE_ROOT_TOKEN`,
   `DATA_NETWORK` comme à l'étape 4) :
   `docker run --rm --network "$DATA_NETWORK" curlimages/curl:8.10.1 -s -X POST http://egide:8200/v1/transit/keys/joel-wealth/rotate -H "Authorization: Bearer $EGIDE_ROOT_TOKEN"`,
   puis `unset EGIDE_ROOT_TOKEN`.
2. Ré-envelopper la clé de données :
   `docker compose -f deploy/compose.prod.yml run --rm api rewrap-wealth-key`.
   Les données chiffrées ne bougent pas. `run --rm api rewrap-wealth-key` lance un conteneur
   éphémère qui ré-enveloppe seulement la clé stockée puis s'arrête : il ne sert aucun
   trafic, la contrainte d'instance unique reste respectée.

## Limites connues (Egide 0.1.0)

- Pas de moteur de politiques : le token `joel-api` peut utiliser toutes les clés Transit et
  tous les secrets. Cette instance ne sert que Joel. À restreindre dès que les politiques
  existeront.
- Egide parle HTTP en clair sur le réseau Docker interne `data`, non routé.
- Un attaquant root sur le VPS pendant que Joel tourne peut lire la clé en mémoire. Le
  chiffrement protège les dumps, les sauvegardes et les disques, pas un hôte compromis.
