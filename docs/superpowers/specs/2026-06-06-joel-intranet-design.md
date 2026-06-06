# Joel — Directeur de cabinet personnel

**Date** : 2026-06-06
**Statut** : design validé section par section, en attente de relecture finale

## 1. Vision

Joel est un intranet personnel hébergé sur VPS : un « family office » numérique qui
centralise agents IA, routines automatisées, planning et revue de presse, accessible
depuis iPhone et PC à tout moment. À terme, Joel devient un véritable assistant de
cabinet : il prépare le briefing du matin, surveille les dossiers, exécute les routines,
parle (sortie audio) et accompagne les carrières entrepreneuriale et politique de son
mandant. Tout est interne, souverain et invisible depuis Internet.

## 2. Décisions structurantes

| Sujet | Décision | Motivation |
|---|---|---|
| Accès | NetBird self-hosted (mesh WireGuard) | Souveraineté totale, zéro exposition publique de l'intranet |
| Domaine | `intranet.pierrickfonquerne.com`, résolu uniquement en interne | Cert wildcard DNS-01 → aucune trace dans les CT logs |
| Front | Angular (PWA) | TypeScript strict natif, structure imposée, PWA installable iPhone |
| Backend | Rust (Axum), 2 binaires : `api` + `runner` | Performance, sûreté, plaisir d'apprentissage ; crash d'un job ≠ crash du dashboard |
| Base | PostgreSQL 17, unique état + file de jobs (`SKIP LOCKED`) | Pas de broker supplémentaire à maintenir |
| Topologie | Monolithe modulaire hexagonal | 1 dev, 1 utilisateur ; extraction de services possible plus tard |
| IA | Passerelle multi-fournisseurs (Anthropic, Mistral) | Les briques ignorent le fournisseur ; bascule et ajout (modèle local) sans refonte |
| Agents existants | Pont en mode pull (boîte de dépôt externe) | Les scheduled Claude déposent, le runner récupère ; aucun flux entrant |
| Planning | Calendrier interne maître + import Google Calendar lecture seule | Le sensible reste interne ; scope OAuth `readonly` minimise l'impact d'une compromission |
| Auth applicative | Passkeys (WebAuthn) + secours mot de passe + TOTP | Face ID / Windows Hello au quotidien, anti-phishing ; secours en cas de perte d'appareil |
| Notifications | ntfy self-hosted + app iPhone | Push souverain, sans service tiers |
| VPS | Debian/Ubuntu, Docker, ≥ 8 Go RAM | Existant, dimensionnement confortable |

## 3. Infrastructure

Tout tourne en Docker Compose sur le VPS, en deux zones étanches.

**Zone publique** (seuls ports exposés sur Internet) :
- NetBird self-hosted : management, signal, relay (coturn), dashboard d'administration
  (`netbird.pierrickfonquerne.com`, ports 443, 3478/udp, plage TURN).
- SSH : clé uniquement, port non standard.

**Zone privée** (joignable uniquement via l'interface mesh `100.x`) :
- **Caddy** : terminaison TLS (wildcard `*.pierrickfonquerne.com` obtenu par défi
  DNS-01), sert le front Angular compilé, proxy `/api` vers le binaire `api`.
- **api** (Axum) : REST + SSE, authentification, façade des modules métier.
- **runner** (Rust) : scheduler cron interne + exécution des jobs ; seul conteneur
  autorisé à sortir vers les API IA et les sources externes (RSS, calendrier, dépôt).
- **PostgreSQL 17** : données + file de jobs ; jamais exposé hors du réseau Docker.
- **ntfy** : notifications push vers les appareils enrôlés. Particularité iOS : la
  livraison instantanée exige un signal de réveil relayé par le serveur ntfy officiel
  (`upstream-base-url`) ; seul un identifiant opaque transite, le contenu des
  notifications reste sur le VPS.

Cloisonnement : réseaux Docker séparés (Caddy ne voit que `api` ; `api` et `runner`
voient PostgreSQL ; seul `runner` a un accès sortant). Conteneurs durcis : multi-stage
builds, images distroless/alpine, utilisateur non-root, système de fichiers en lecture
seule, `LABEL` version et description.

DNS : l'enregistrement `intranet.…` n'existe que dans le DNS interne NetBird, résolu
vers l'IP mesh du VPS pour les seuls appareils enrôlés.

Sauvegardes : dump PostgreSQL quotidien, chiffré (age), copié hors du VPS ; la clé de
restauration est conservée hors ligne.

## 4. Architecture logicielle

Monorepo `joel` :

```
joel/
├── backend/                  # workspace Cargo
│   ├── crates/
│   │   ├── domain/           # entités + ports (traits) — zéro dépendance infra
│   │   ├── persistence/      # adapters PostgreSQL (sqlx)
│   │   ├── ai-gateway/       # ports ChatProvider / SpeechProvider ; adapters Anthropic, Mistral
│   │   └── ingest/           # adapters entrants : RSS, calendrier externe (lecture), boîte de dépôt
│   └── bins/
│       ├── api/              # Axum : REST + SSE, auth, façade des cas d'usage
│       └── runner/           # scheduler cron + exécuteur de jobs (file PostgreSQL)
├── frontend/                 # Angular PWA, strict mode, ESLint, logger structuré
│   └── src/app/
│       ├── core/             # auth, client API généré, layout, thème
│       └── features/         # 1 module lazy-loaded par brique :
│           # cockpit / agents / routines / presse / planning
└── deploy/                   # docker-compose.yml, Caddyfile, scripts d'exploitation
```

Principes :
- **Hexagonal** : `domain` contient la logique métier pure et les ports (traits) ;
  `api` et `runner` sont deux adaptateurs d'entrée qui traversent les mêmes cas
  d'usage ; `persistence`, `ai-gateway` et `ingest` sont les adaptateurs de sortie.
- **Contrat front ⇄ back** : spécification OpenAPI générée depuis le code (`utoipa`),
  client Angular (services + interfaces) régénéré à chaque build — toute rupture de
  contrat casse la compilation, pas la production.
- **Temps réel** : SSE (unidirectionnel serveur → client) pour les statuts d'agents,
  de jobs et les nouveaux digests ; reconnexion automatique native.
- **Qualité** : Clippy strict (pedantic), `cargo-deny`, TypeScript strict, ESLint,
  aucun `console.log` ; SOLID partout.

## 5. Modèle de données et flux

Chaîne d'exécution : **routine → job → agent → agent_run**.

- `routine` : déclencheur planifié (expression cron, actif/inactif) ; cible un agent
  ou une tâche technique (sauvegarde, ingestion).
- `job` : unité d'exécution dans la file PostgreSQL (`queued → running → done/failed`),
  verrouillée par `SELECT … FOR UPDATE SKIP LOCKED` ; tentatives, backoff exponentiel,
  horodatage de planification.
- `agent` : configuration IA — fournisseur, modèle, prompt système, outils autorisés,
  budget ; ou référence vers un agent externe ponté.
- `agent_run` : trace d'exécution — entrée, sortie, durée, tokens, coût, erreur.

Autres entités : `source` (flux de presse), `article`, `digest` (briefing par thème),
`event` (planning interne + événements importés marqués `origin`), `external_account`
(tokens OAuth chiffrés), `app_user` et `session` (mono-utilisateur en v1, extensible),
`audit_log` (toute action sensible : connexion, modification d'agent, déclenchement).

**Flux de référence — revue de presse** : à 06h30 le runner crée le job
`presse.digest` ; `ingest` collecte les flux RSS ; `ai-gateway` produit le résumé
thématique via le fournisseur configuré ; le digest est stocké ; SSE notifie le front ;
ntfy pousse « briefing prêt » sur l'iPhone.

**Pont agents externes — règle d'or : on ne pousse jamais vers l'intranet.** Les
scheduled Claude (infrastructure Anthropic) déposent leurs résultats dans une boîte de
dépôt externe — un dépôt git privé dédié (structuré, versionné, authentifié) ; le
runner le consulte périodiquement. Aucun flux entrant ; l'intranet reste invisible.

## 6. Sécurité

Défense en profondeur, de l'extérieur vers l'intérieur :
1. **Réseau** : rien d'exposé hormis NetBird et SSH ; l'intranet n'existe pas pour un
   scanner (pas de DNS public, pas de trace CT, pas de port).
2. **Transport** : WireGuard (mesh) + TLS interne (wildcard).
3. **Application** : passkeys WebAuthn (crate `webauthn-rs`) en usage quotidien ;
   secours mot de passe (argon2id) + TOTP ; sessions httpOnly/Secure/SameSite ;
   en-têtes durcis (CSP stricte, HSTS) posés par Caddy ; rate limiting sur `api`.
4. **Données** : tokens OAuth chiffrés au repos (AES-256-GCM, clé maîtresse dans
   l'environnement du conteneur, hors git) ; clés API IA en variables d'environnement ;
   sauvegardes chiffrées ; `audit_log` complet.
5. **Exploitation** : mises à jour automatiques de l'hôte (unattended-upgrades),
   dépendances suivies et reconstruites régulièrement ; images non-root.

Modèle de menace assumé : l'adversaire de référence est un attaquant distant opportuniste
ou ciblé sans accès physique. Le vol d'un appareil enrôlé est couvert par
l'authentification applicative ; la compromission complète du VPS est limitée par les
scopes minimaux (lecture seule Google), le chiffrement au repos et l'absence de
credentials à fort pouvoir.

## 7. Erreurs et observabilité

- Jobs : tentatives multiples avec backoff exponentiel ; échec répété → notification
  ntfy ; les erreurs sont des données (visibles au Cockpit), pas seulement des logs.
- Fournisseurs IA : timeout et disjoncteur ; bascule configurable entre fournisseurs.
- Logs structurés JSON : `tracing` côté Rust, logger structuré côté Angular.
- Le Cockpit affiche la santé du système : derniers runs, jobs en échec, coûts API
  cumulés par agent et par mois.
- Healthchecks Docker + politiques de redémarrage sur tous les services.

## 8. Tests et CI

- `domain` : tests unitaires purs, sans infra — la majorité des tests.
- `persistence` : tests d'intégration PostgreSQL (testcontainers).
- `api` : tests d'intégration HTTP (router monté en mémoire).
- `ai-gateway` : fournisseurs mockés via les traits ; tests de contrat.
- Front : tests unitaires des composants et services ; lint strict.
- CI sur chaque branche : build, Clippy pedantic, `cargo-deny`, tests, ESLint,
  build Angular. Tout doit être vert avant commit (TBD : branches courtes
  `feature/…`, `fix/…`, `docs/…`, merge `--no-ff` vers `main` après GO).

## 9. Briques et roadmap

Chaque brique suit son cycle complet : spec → plan → implémentation → review, sur sa
branche. Une brique n'est commencée que lorsque la précédente est en production.

| # | Brique | Contenu | Critère de fin observable |
|---|---|---|---|
| 0 | Socle | Monorepo, Compose (Caddy, api, runner, PostgreSQL, ntfy), NetBird self-hosted, DNS interne, cert wildcard, auth passkeys + TOTP, CI, Cockpit squelette PWA | Ouvrir la PWA depuis l'iPhone en 4G via NetBird et se connecter avec Face ID |
| 1 | Routines & jobs | File `SKIP LOCKED`, scheduler cron, CRUD routines, historique, retries, alertes ntfy | Une routine de test tourne toutes les heures ; son échec simulé notifie l'iPhone |
| 2 | Passerelle IA & Bureau des agents | Ports `ChatProvider`/`SpeechProvider`, adapters Anthropic + Mistral, entités agent/agent_run, UI complète, SSE, suivi coûts | Déclencher un agent depuis le téléphone et le voir s'exécuter en direct |
| 3 | Revue de presse | Ingestion RSS, gestion des sources, digest quotidien par thème, archive, notification | Chaque matin à 06h32, le briefing attend, généré, classé, notifié |
| 4 | Planning | Calendrier interne CRUD, import Google lecture seule (tokens chiffrés), vues jour/semaine/mois, échéances au Cockpit | Le Cockpit affiche la journée complète, interne + externe fusionnés |
| 5 | Pont agents externes | Boîte de dépôt, polling, résultats intégrés au Bureau des agents | Tous les agents, internes et externes, visibles au même endroit |

**v2 — les cabinets** : Cabinet entrepreneurial (objectifs, jalons, KPIs, veille pour
Nubster) et Cabinet politique (dossiers, contacts, agenda militant, veille locale),
construits sur le socle : agents, routines, presse et planning à leur service.

**Extensions prévues par le design** (non planifiées) : sortie audio des briefings via
le port `SpeechProvider` (offre TTS Mistral à vérifier le moment venu ; alternative
locale type Piper), modèle local via la passerelle, accès multi-utilisateurs (famille),
fallback d'accès public minimal si un besoin réel apparaît.

## 10. Hors scope v1

- Toute exposition publique de l'intranet (y compris vue lecture seule).
- Écriture vers Google Calendar.
- Les cabinets entrepreneurial et politique (v2).
- La voix de Joel (extension post-v1).
- Multi-utilisateurs.
