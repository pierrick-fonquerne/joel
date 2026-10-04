# Coffre patrimoine (Egide) : exploitation

Egide garde la clé qui enveloppe la clé de données du module patrimoine. Tant qu'Egide est
scellé, Joel fonctionne mais les routes `/api/wealth/*` répondent `503 wealth_vault_sealed`.

## Contraintes

- Joel api doit tourner en une seule instance : le déverrouillage du coffre n'est protégé
  contre la concurrence que dans le processus. Avec deux réplicas, chacun pourrait créer une
  clé de données différente sur une table vide.

## Mise en place (une seule fois)

1. Démarrer Egide : `docker compose -f deploy/compose.prod.yml up -d egide`
2. Initialiser : `docker compose -f deploy/compose.prod.yml exec egide egide operator init`.
   Ranger les parts Shamir et le token root hors du VPS (gestionnaire de mots de passe, une
   part par emplacement). Le token root n'est affiché qu'une fois.
3. Desceller : `docker compose -f deploy/compose.prod.yml exec egide egide operator unseal`,
   autant de fois que le seuil de parts l'exige.
4. Créer la clé Transit, avec le token root :
   `docker compose -f deploy/compose.prod.yml exec -e EGIDE_TOKEN=<root> egide sh -c 'curl -s -X POST http://localhost:8200/v1/transit/keys -H "Authorization: Bearer $EGIDE_TOKEN" -H "Content-Type: application/json" -d "{\"name\":\"joel-wealth\"}"'`
5. Créer le token de service de Joel, avec le token root :
   `... -d '{"service_name": "joel-api"}'` sur `POST /v1/auth/service-tokens`.
6. Écrire le token `egst_...` dans `deploy/secrets/egide_token` (droits `600`, jamais commité).
7. Redémarrer l'api : `docker compose -f deploy/compose.prod.yml up -d api`. Au premier
   démarrage, Joel génère la clé de données et enregistre sa version enveloppée.

## Après chaque redémarrage du VPS

Egide redémarre scellé. Lancer l'étape 3. Les routes patrimoine reviennent seules dans les
30 secondes, sans redémarrer Joel.

## Rotation de la clé

1. Faire tourner la clé maîtresse avec le token root :
   `POST /v1/transit/keys/joel-wealth/rotate`.
2. Ré-envelopper la clé de données :
   `docker compose -f deploy/compose.prod.yml run --rm api rewrap-wealth-key`.
   Les données chiffrées ne bougent pas.

## Limites connues (Egide 0.1.0)

- Pas de moteur de politiques : le token `joel-api` peut utiliser toutes les clés Transit et
  tous les secrets. Cette instance ne sert que Joel. À restreindre dès que les politiques
  existeront.
- Egide parle HTTP en clair sur le réseau Docker interne `data`, non routé.
- Un attaquant root sur le VPS pendant que Joel tourne peut lire la clé en mémoire. Le
  chiffrement protège les dumps, les sauvegardes et les disques, pas un hôte compromis.
