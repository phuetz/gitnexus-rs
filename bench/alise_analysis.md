# Alise_v2 — Validation sur corpus métier français réel

## Contexte

Alise_v2 est une application CCAS (Centre Communal d'Action Sociale) de gestion
des aides sociales. ~1000 fichiers C# (.NET MVC 5 / Entity Framework 6), docs
et UI en français, domaine métier social.

- **Graph** : 1065 files, 14016 nodes, 30251 edges
- **Embeddings** : 6225 symboles via `paraphrase-multilingual-MiniLM-L12-v2`,
  générés en 251s, fichier 10MB
- **Queries** : 12 requêtes réalistes (mix FR métier + EN technique + cross-lingual)

## Analyse 12 queries

### Démonstrations parfaites ("keyword fails, semantic wins")

| Q | Query | BM25 | Hybrid |
|---|---|---|---|
| 12 | "background jobs planification" | **0 résultat** | `BackgroundJobService` #1 |
| 8 | "where is the DossierController" | `is_numeric`, `is` Modernizr.js (bruit JS) | `ModifDossier`, `RechDossier`, `CreerDossier` (vraies actions) |
| 5 | "validation par la commission sociale" | `Commission` controller, `CommissionModel`, tests | `ValiderParCommission` #1 (la méthode principale) |

Q12 = BM25 retourne zéro parce que "planification" n'est dans aucun symbole ; le hybrid comprend que "background jobs" ≈ `BackgroundJobService`.

Q8 = question EN sur code FR. BM25 matche juste sur "is" et renvoie du JS. Hybrid fait le cross-lingual transfer et trouve les controller actions pertinentes.

Q5 = "validation" sémantique → `ValiderParCommission`. BM25 ne voit que le nom littéral "Commission".

### Gains nets (sémantique complète le keyword)

| Q | Top-1 hybrid ajouté vs BM25 | Pertinence |
|---|---|---|
| 2 | `CreerPaiementBen` ControllerAction #2 | Vraie action métier |
| 3 | `AideFinance` #2 | Domain entity pertinente |
| 6 | recall élargi de 1→5 (DbContext Entity Framework) | `CMCASClient`, `CreateViewContext` |
| 7 | `AutorisationPaiementBenef` #2, `UserCache` #4 | Plus "action" moins "entity" |
| 11 | `ExportQuantitative` method #2, `ExcelTemplExpSuiviBudget` #3 | Les vraies classes d'export Excel |

### Mitigés ou régression

- **Q1** : bruit Python injecté (`add_severity` dans `corrections_courrier_masse/gen_doc.py`). Cross-project pollution — le dossier `corrections_courrier_masse/` est un working folder avec des scripts Python qui matchent sémantiquement "ajouté".
- **Q4** : régression légère. BM25 avait déjà `MappingInfoBenef`, `GenererPdf`, etc. (très pertinent). Hybrid amène `MailBodyUtil`, `MailUtil` qui sont pertinents aussi mais moins précis.
- **Q9, Q10** : embeddings remontent des entités proches mais BM25 était déjà raisonnable.

## Scorecard

**Sur 12 queries FR métier :**
- Gros gains (⬆️⬆️⬆️) : Q5, Q8, Q12 → **3/12 (25%)**
- Gains nets (⬆️) : Q2, Q3, Q6, Q7, Q11 → **5/12 (42%)**
- Mitigés : Q1, Q9, Q10 → 3/12 (25%)
- Régression : Q4 → 1/12 (8%)

**Total strictement amélioré : 8/12 (67%)**, dont 3 where BM25 was fundamentally broken.

## Pertinence commerciale (agile-up.com)

Ce bench est la **démonstration qu'on veut pour les clients agile-up.com** :

1. **Q12 "background jobs planification"** — parfaite démo 30s : "voyez, BM25 retourne zéro, le second brain trouve le service en 1 seconde"
2. **Q8 cross-lingual** — clients internationaux / équipes mixtes : question EN sur code FR, ou l'inverse, marche
3. **Q5 sémantique** — "validation" ↔ "valider", "aide" ↔ "assistance", les variations lexicales naturelles sont absorbées

Le pipeline actuel (BM25 + embeddings multilingues + RRF) couvre exactement le cas d'usage que la vidéo Obsidian+Claude décrivait comme "le bon second brain d'entreprise". Sauf qu'on l'a pour de vrai, chiffré, sur un corpus client réel.

## Observations techniques

- **Model multilingue NÉCESSAIRE** pour Alise_v2. MiniLM-L6 anglais ne comprend pas "bénéficiaire", "barème", "règles". La cross-lingual coincidental transfer qu'on voyait sur gitnexus-rs (corpus EN) ne s'applique pas ici.
- **Embed time** : 251s pour 6225 symboles = ~40ms/symbole. Scalable pour des projets de taille raisonnable (<50k symboles) sur CPU. GPU diviserait par 5-10.
- **Fichier embeddings.bin** : 10MB pour ce corpus. Négligeable vs la taille du projet.
- **Cold start** : ~3s pour charger le modèle ONNX à chaque `gitnexus query`. Pour un usage intensif, cacher la session dans le MCP daemon / desktop app.

## Bundling suggestion pour la prestation

Package type "audit + onboarding" pour un client agile-up.com :

1. `gitnexus analyze` sur leur repo (~1min)
2. `gitnexus embed --model multilingue` (~5min pour un repo ~15k symboles)
3. Export HTML de 10-20 queries représentatives de leur domaine (like this doc)
4. Le pitch : "BM25 vs hybrid side-by-side, voyez ce que vous gagnez"

Deliverable : le .gitnexus/ folder (graph + embeddings + doc enrichie) + une courte démo live.
