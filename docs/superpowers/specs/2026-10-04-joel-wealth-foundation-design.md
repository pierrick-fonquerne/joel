# Socle patrimoine de Joel, brique W0 (design)

- Date : 2026-10-04
- Slug : joel-wealth-foundation
- Statut : validé en brainstorming, en attente de revue de la spec écrite
- Dépôts touchés : `Perso/joel` (nouveau module `domain/wealth`, nouveaux crates `cipher-egide` et `exchange-rates-ecb`, `persistence`, `bins/api`, `bins/runner`, `frontend/features/wealth`, `deploy`)
- Dépendance externe : Egide (`nubster-opensources/egide`, moteur Transit), déployé à côté de Joel
- Successeurs prévus (hors périmètre) : W1 Tools agent, W2 Imports, W3 Routines, W4 Lignes et performance, W5 Pilotage SASU

## 1. Objet

Pierrick veut suivre l'ensemble de son patrimoine (courtiers PEA et CTO, assurance-vie, PER,
comptes bancaires, crypto, immobilier, parts de société), perso et SASU, depuis son iPhone,
et à terme le piloter avec l'agent de Joel.

Une recherche préalable (2026-10-01) a comparé Ghostfolio, Wealthfolio, Portfolio Performance,
Sure, Firefly III et Actual Budget. Aucun ne gère nativement PEA, assurance-vie, fonds euros ni
la consolidation perso et SASU. Le choix retenu est un module natif dans Joel, qui réutilise
son socle (PWA, passkeys, NetBird, substrat de capacités) au lieu d'ajouter un outil tiers à
côté.

W0 livre le socle : le modèle, la saisie manuelle des relevés, le calcul du patrimoine net, le
chiffrement des données sensibles via Egide, l'API et un tableau de bord PWA. Critère de fin :
Pierrick saisit ses comptes depuis l'iPhone et lit son patrimoine net et son évolution.

## 2. Décisions verrouillées (brainstorming 01-04/10/2026)

| # | Sujet | Décision |
|---|-------|----------|
| 1 | Outil tiers ou natif | **Natif dans Joel**. Wealthfolio et Ghostfolio écartés comme moteur : modèle sans SASU ni AV, double authentification, dépendance à une API tierce |
| 2 | Application mobile | **La PWA de Joel**, déjà installable sur iPhone avec Face ID et joignable via NetBird. Pas d'application native |
| 3 | Unité de base | **Le relevé** (`Valuation`) : la valeur d'un compte à une date. Toutes les sources savent en produire un. Les lignes et la performance TWR arrivent en W4 |
| 4 | Historique | Les relevés sont **ajoutés, jamais modifiés**. Une correction est un nouveau relevé. L'historique vient gratuitement |
| 5 | Passifs | Un prêt est un `Account` de type `Loan` dont les relevés sont négatifs. Le patrimoine net immobilier se lit sans cas particulier |
| 6 | Protection au repos | **Chiffrement en colonne par enveloppe via Egide** : une clé de données (`datakey`) obtenue d'Egide, stockée enveloppée, déchiffrée au démarrage, gardée en mémoire effaçable |
| 7 | Indisponibilité d'Egide | Joel démarre quand même. Seules les routes patrimoine répondent `503` (coffre scellé). Le reste de l'intranet n'est pas touché |
| 8 | Découpage | W0 Socle, W1 Agent, W2 Imports, W3 Routines, W4 Lignes, W5 SASU. Chaque brique a sa spec |
| 9 | EidosDB | **Hors du chemin des soldes** (pas d'ACID). Candidat en W1 pour retrouver des documents (relevés PDF, avis d'impôt) via le knowledge store existant |

## 3. Langage du domaine

Un concept, un nom, partout (code, tests, documentation, commits).

| Terme (code) | Sens |
|---|---|
| `Owner` | À qui appartient le compte : `Personal` ou `Company` (Labade Conseil). Énumération, pas de table en W0 |
| `Account` | Une enveloppe patrimoniale : nom, `AccountKind`, `Owner`, devise, archivé ou non |
| `AccountKind` | `Brokerage { envelope: BrokerageEnvelope }`, `LifeInsurance`, `RetirementPlan`, `BankAccount`, `Savings`, `CryptoWallet`, `RealEstate`, `CompanyShares`, `Loan` |
| `BrokerageEnvelope` | `Pea`, `Cto` |
| `Valuation` | Un relevé : `account_id`, `as_of` (date), `amount` (`Money`), `source`, `recorded_at` |
| `ValuationSource` | `Manual`, `CsvImport`, `BankAggregation`, `PriceFeed`. Seul `Manual` est produit en W0 |
| `Money` | Montant décimal exact (`rust_decimal::Decimal`) et devise ISO 4217. Jamais de flottant |
| `NetWorth` | À une date : somme, convertie en EUR, du dernier relevé de chaque compte non archivé, avec ventilation par `Owner` et par `AccountKind` |
| `ExchangeRate` | Taux BCE à une date : nombre d'unités de la devise pour 1 EUR (`units_per_eur`). Montant en EUR = montant / `units_per_eur` |
| `FieldCipher` | Port de chiffrement d'un champ, avec contexte lié (AAD) |
| `StaleAccount` | Compte dont le dernier relevé a plus de 45 jours |

## 4. Architecture

```
backend/
├─ crates/
│  ├─ domain/
│  │  └─ src/wealth/              NOUVEAU : pur, zéro infra
│  │       ├─ mod.rs
│  │       ├─ money.rs            Money, Currency
│  │       ├─ account.rs          Account, AccountId, AccountKind, BrokerageEnvelope, Owner
│  │       ├─ valuation.rs        Valuation, ValuationId, ValuationSource
│  │       ├─ net_worth.rs        NetWorth, NetWorthBreakdown, calcul pur
│  │       ├─ ports.rs            AccountRepository, ValuationRepository, ExchangeRateSource, FieldCipher
│  │       ├─ use_cases.rs        CreateAccount, ArchiveAccount, RecordValuation, ComputeNetWorth, NetWorthHistory
│  │       └─ test_support.rs     faux pour chaque port
│  │
│  ├─ cipher-egide/               NOUVEAU crate : adaptateur FieldCipher
│  │  └─ src/
│  │       ├─ lib.rs
│  │       ├─ egide_client.rs     client HTTP Transit (datakey, decrypt, rewrap)
│  │       └─ envelope_cipher.rs  EgideEnvelopeCipher : AES-256-GCM local avec la datakey
│  │
│  ├─ persistence/
│  │  └─ src/wealth.rs            NOUVEAU : dépôts Postgres + lecture et écriture des taux BCE en cache
│  │
│  └─ exchange-rates-ecb/         NOUVEAU crate : client du flux quotidien BCE (parse XML)
│
├─ migrations/0003_wealth.sql     NOUVEAU
├─ bins/api/                      câblage, routes /api/wealth/*, état « coffre scellé »
└─ bins/runner/                   tâche quotidienne : flux BCE -> table des taux

frontend/src/app/features/wealth/ NOUVEAU : tableau de bord, comptes, saisie d'un relevé
deploy/                           service Egide sur le réseau interne, secret du token
```

Règle de dépendance : `domain/wealth` ne connaît ni Postgres, ni HTTP, ni Egide. Le chiffrement
passe par le port `FieldCipher`. Les dépôts reçoivent des valeurs déjà chiffrées ou les
déchiffrent via ce port, jamais en appelant Egide directement.

Le conteneur `api` vit sur des réseaux Docker internes, sans accès à Internet. Il ne lit donc
les taux que depuis la table `wealth_exchange_rates`. C'est le `runner`, seul à avoir une
sortie Internet, qui récupère chaque jour le flux BCE et alimente cette table. Côté `api`,
l'implémentation de `ExchangeRateSource` est donc la lecture Postgres.

Le crate `capability-wealth` (Tools pour l'agent) n'est pas créé en W0 : il appartient à W1 et
dépend du substrat de capacités, pas encore intégré sur `main`.

## 5. Contrat du domaine (signatures)

```rust
pub struct Money { amount: Decimal, currency: Currency }

pub enum Owner { Personal, Company }

pub enum BrokerageEnvelope { Pea, Cto }

pub enum AccountKind {
    Brokerage { envelope: BrokerageEnvelope },
    LifeInsurance, RetirementPlan, BankAccount, Savings,
    CryptoWallet, RealEstate, CompanyShares, Loan,
}

pub struct Account {
    id: AccountId, name: String, kind: AccountKind, owner: Owner,
    currency: Currency, is_archived: bool, notes: Option<String>,
}

pub struct Valuation {
    id: ValuationId, account_id: AccountId, as_of: Date,
    amount: Money, source: ValuationSource, recorded_at: OffsetDateTime,
}

pub struct NetWorth {
    as_of: Date, total: Money,
    by_owner: BTreeMap<Owner, Money>,
    by_kind: BTreeMap<AccountKind, Money>,
    stale_accounts: Vec<AccountId>,
}

pub struct CipherContext { table: &'static str, column: &'static str, row_id: Uuid }

pub trait FieldCipher: Send + Sync {
    fn encrypt(&self, plaintext: &[u8], context: &CipherContext) -> Result<Vec<u8>, CipherError>;
    fn decrypt(&self, ciphertext: &[u8], context: &CipherContext) -> Result<Vec<u8>, CipherError>;
}

pub trait ExchangeRateSource: Send + Sync {
    /// Units of `currency` for one euro, on `on` or the closest earlier day (7 days max).
    async fn units_per_eur(&self, currency: Currency, on: Date) -> Result<Option<Decimal>, WealthError>;
    /// True when at least one rate is known for `currency` (EUR is always known).
    async fn is_supported(&self, currency: Currency) -> Result<bool, WealthError>;
}

pub trait WrappedKeyStore: Send + Sync {
    async fn current(&self) -> Result<Option<String>, WealthError>;
    async fn insert(&self, wrapped_key: &str, egide_key_name: &str) -> Result<(), WealthError>;
}
```

Invariants vérifiés par le domaine :

- Un relevé d'un compte `Loan` est négatif ou nul. Un relevé d'un autre type est positif ou nul.
- La devise d'un relevé est celle de son compte.
- La devise d'un compte est l'EUR ou une devise publiée par la BCE. Une crypto se suit dans un compte en EUR, valorisé à la main.
- `as_of` n'est pas dans le futur.
- On ne saisit pas de relevé sur un compte archivé.
- `ComputeNetWorth(at)` retient, pour chaque compte, le relevé de plus grand `as_of` inférieur
  ou égal à `at`. À `as_of` égal, le plus récent `recorded_at` gagne (correction).
- La conversion utilise le taux BCE du jour `as_of` du relevé, ou du dernier jour ouvré
  précédent. Un taux introuvable fait échouer le calcul avec une erreur explicite, jamais une
  conversion à 1.

`NetWorthHistory(from, to)` produit un point par fin de mois, plus le point du jour.

## 6. Stockage et chiffrement

### Migration `0003_wealth.sql`

| Table | Colonnes en clair | Colonnes chiffrées (`BYTEA`) |
|---|---|---|
| `wealth_accounts` | `id`, `kind` (le code porte l'enveloppe : `brokerage_pea`, `brokerage_cto`), `owner`, `currency`, `is_archived`, `created_at` | `name`, `notes` |
| `wealth_valuations` | `id`, `account_id`, `as_of`, `currency`, `source`, `recorded_at` | `amount` (décimal sérialisé en texte avant chiffrement) |
| `wealth_exchange_rates` | `currency`, `on_date`, `units_per_eur` (unités de devise pour 1 EUR, convention BCE) | aucune (donnée publique) |
| `wealth_keys` | `version`, `egide_key_name`, `created_at` | `wrapped_key` (déjà chiffrée par Egide) |

Pas de `UPDATE` ni de `DELETE` sur `wealth_valuations` côté application.

### Enveloppe Egide

1. Premier démarrage, `wealth_keys` vide : `POST /v1/transit/datakey/joel-wealth`. Joel garde
   la clé en clair en mémoire et insère uniquement sa version enveloppée.
2. Démarrages suivants : `POST /v1/transit/decrypt/joel-wealth` sur `wrapped_key`. La clé en
   clair vit dans un `Zeroizing<[u8; 32]>`, jamais journalisée, jamais écrite sur disque.
3. Chaque champ : AES-256-GCM, nonce aléatoire de 96 bits préfixé au chiffré. L'AAD est
   `CipherContext` sérialisé (`table`, `column`, `row_id`) : un chiffré copié vers une autre
   ligne ou une autre colonne ne se déchiffre pas.
4. Rotation : rotation de la clé maîtresse dans Egide (opération root, hors Joel), puis
   `POST /v1/transit/rewrap/joel-wealth` sur `wrapped_key`. Les données ne sont pas touchées.

### Indisponibilité d'Egide

Egide scellé, injoignable, ou token refusé au démarrage : Joel démarre, journalise l'erreur
sans secret, et l'état « coffre scellé » rend `503` avec un code `wealth_vault_sealed` sur
toutes les routes `/api/wealth/*`. Une nouvelle tentative a lieu à chaque requête patrimoine,
au plus une fois toutes les 30 secondes, pour reprendre sans redémarrer Joel après le
descellement.

### Ce que ça protège, et ce que ça ne protège pas

Protégé : dump de base, sauvegarde volée ou mal stockée, disque récupéré. Non protégé : un
attaquant root sur le VPS pendant que Joel tourne (clé lisible en mémoire). Egide est en
alpha : à suivre de près, sans y mettre plus que la clé `joel-wealth`.

## 7. Surface HTTP (`bins/api`)

Toutes les routes exigent une session authentifiée, comme le reste de l'API.

| Méthode | Route | Effet |
|---|---|---|
| `GET` | `/api/wealth/accounts` | Liste des comptes, avec dernier relevé et drapeau `is_stale` |
| `POST` | `/api/wealth/accounts` | Crée un compte |
| `POST` | `/api/wealth/accounts/{id}/archive` | Archive un compte |
| `POST` | `/api/wealth/accounts/{id}/valuations` | Ajoute un relevé |
| `GET` | `/api/wealth/accounts/{id}/valuations` | Historique des relevés d'un compte |
| `GET` | `/api/wealth/net-worth?at=YYYY-MM-DD` | `NetWorth` à une date (aujourd'hui par défaut) |
| `GET` | `/api/wealth/net-worth/history?from=&to=` | Série mensuelle |

Les montants circulent en JSON sous forme de chaîne décimale (`"12345.67"`), jamais en nombre
flottant. Erreurs : `400` invariant violé (code métier explicite), `404` compte inconnu,
`422` taux de change introuvable, `503` coffre scellé.

## 8. PWA (`features/wealth`)

- **Tableau de bord** : patrimoine net en grand, écart avec la fin du mois précédent (montant et
  pourcentage), sélecteur Perso / SASU / Total, répartition par type, courbe de l'historique.
- **Comptes** : liste groupée par propriétaire, badge « à mettre à jour » sur les comptes
  `StaleAccount`, création et archivage.
- **Nouveau relevé** : pensé pour le pouce, deux champs (montant, date pré-remplie à
  aujourd'hui), validation en un geste.
- **Tuile cockpit** : patrimoine net et nombre de comptes à mettre à jour.
- **Mode discret** : un appui masque tous les montants. L'état est conservé dans le
  `localStorage` du téléphone.
- **Coffre scellé** : sur un `503 wealth_vault_sealed`, un écran explique que le coffre doit
  être descellé, sans afficher d'erreur technique.

## 9. Déploiement

- Service `egide` (image `nubster/egide`, version épinglée) dans la composition Docker de
  Joel, sur le réseau interne `data` uniquement, sans port publié, volume de données
  persistant, `EGIDE_ENV=production`. Egide parle HTTP en clair : c'est acceptable parce que
  le réseau `data` est interne et non routé, et que seuls `api`, `runner` et `postgres` y sont.
- Clé Transit `joel-wealth` créée une fois par l'opérateur (Pierrick, token root, CLI Egide).
- Token de service Egide dédié à Joel (`service_name: joel-api`), transmis en secret Docker
  (variable `EGIDE_TOKEN_FILE`), jamais en clair dans la composition. URL en `EGIDE_URL`.
- Limite d'Egide 0.1.0 : il n'y a pas encore de moteur de politiques. Un token de service peut
  utiliser toutes les clés Transit et tous les secrets. On l'accepte parce que cette instance
  Egide ne sert que Joel. À restreindre dès que les politiques existeront.
- Après chaque redémarrage du VPS, Pierrick descelle Egide avec ses parts Shamir. Une
  procédure courte est ajoutée au runbook de `deploy/`.

## 10. Découpage en briques (cycle Planning -> Testing -> Building -> Review)

| Étape | Contenu |
|---|---|
| W0.1 | Domaine `wealth` : `Money`, `Account`, `Valuation`, invariants, calcul `NetWorth`, use cases, faux |
| W0.2 | Crate `cipher-egide` : client Transit, `EgideEnvelopeCipher`, tests `wiremock` |
| W0.3 | Persistance : migration `0003`, dépôts chiffrés, cache des taux ; crate `exchange-rates-ecb` et tâche quotidienne du `runner` |
| W0.4 | API : routes, état « coffre scellé », tests de routes |
| W0.5 | PWA : tableau de bord, comptes, saisie, tuile cockpit, mode discret |
| W0.6 | Déploiement : service Egide, secret, runbook de descellement, vérification sur iPhone en 4G |

## 11. Hors périmètre (specs ultérieures)

- W1 : crate `capability-wealth`, Tools passifs `net_worth`, `allocation`, `account_history`, et
  éventuellement indexation de documents dans le knowledge store (EidosDB).
- W2 : imports CSV des courtiers, agrégation bancaire Enable Banking (identifiants dans Egide).
  GoCardless est fermé aux nouvelles inscriptions depuis juillet 2025.
- W3 : alertes de seuil, revue mensuelle, rappel de mise à jour de l'assurance-vie via ntfy.
- W4 : lignes, cours (Yahoo, BCE), performance TWR et IRR sur les comptes courtiers.
- W5 : pilotage SASU (trésorerie, dividendes, fiscalité).
- Plusieurs utilisateurs ou détenteurs supplémentaires (conjoint, enfants).

## 12. Gestion des erreurs et tests

- Domaine (TDD avec les faux) : chaque invariant de la section 5, sélection du dernier relevé,
  correction à date égale, ventilation par propriétaire et par type, prêt négatif, taux
  introuvable, comptes archivés exclus, détection `StaleAccount`.
- `cipher-egide` : aller-retour chiffrement et déchiffrement, échec avec un mauvais contexte,
  échec avec un chiffré altéré, Egide scellé, token refusé, `datakey` au premier démarrage,
  `decrypt` aux suivants, `rewrap`. Egide simulé par `wiremock`.
- Persistance : tests d'intégration Postgres, vérification que `name`, `notes` et `amount` ne
  sont jamais en clair dans la base.
- API : un test par route et par code d'erreur, dont le `503` coffre scellé.
- PWA : tests des composants (tableau de bord, saisie, mode discret, écran coffre scellé).
- Journalisation : aucun montant, nom de compte ni clé dans les journaux.
- CI : `cargo fmt`, Clippy strict, `cargo test`, `cargo deny`, lint et tests Angular au vert
  avant chaque commit.
