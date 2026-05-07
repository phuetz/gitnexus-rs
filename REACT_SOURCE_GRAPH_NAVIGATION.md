# Navigation React dans les sources et le graphe

Derniere mise a jour: 2026-05-07

## Reponse courte

Oui, GitNexus peut devenir une application React capable de naviguer dans les sources et dans le graphe. Une partie importante existe deja dans l'application desktop Tauri. Le client `chat-ui` web autonome doit encore recevoir une surface dediee et des endpoints HTTP limites pour exposer ces capacites sans donner au navigateur un acces libre au disque.

## Etat actuel

### Desktop React/Tauri

Le desktop contient deja les briques principales:

| Fonction | Surface existante |
| --- | --- |
| Arborescence des fichiers | `crates/gitnexus-desktop/ui/src/components/files/FileTreeView.tsx` |
| Apercu source avec coloration | `crates/gitnexus-desktop/ui/src/components/files/FilePreview.tsx` |
| Graphe interactif | `crates/gitnexus-desktop/ui/src/components/graph/GraphExplorer.tsx` |
| Voisinage d'un noeud | `get_subgraph` cote Tauri |
| Recherche de symboles | `search_symbols` cote Tauri |
| Clic depuis le chat vers le graphe | `ChatMarkdown.tsx` et `ChatMode.tsx` |
| Mode explorateur combine | `crates/gitnexus-desktop/ui/src/components/explorer/ExplorerMode.tsx` |

Les commandes Tauri cote Rust existent aussi:

- `get_file_tree`
- `get_file_content`
- `get_graph_data`
- `get_subgraph`
- `search_symbols`
- `get_impact_analysis`

Conclusion: pour le desktop, la navigation sources + graphe est deja presente et peut etre amelioree.

### Chat React autonome (`chat-ui`)

Le client web autonome est aujourd'hui centre sur la conversation:

- selection de projet indexe;
- streaming `/api/chat`;
- rendu Markdown;
- rendu Mermaid;
- coloration syntaxique;
- exports Markdown/PDF;
- diagnostics backend.

Il ne dispose pas encore d'un vrai explorateur source/graphe. C'est la prochaine etape naturelle.

### Site HTML genere

Le site de documentation genere offre deja une navigation documentaire, des liens de sources, Mermaid et le chat de documentation. Il reste oriente "documentation exportee", pas "exploration live du graphe".

## Principe d'integration

Le navigateur ne doit jamais lire `D:\...` ou `C:\...` directement. La bonne architecture est:

```mermaid
flowchart LR
    A["React chat-ui"] --> B["HTTP API GitNexus"]
    B --> C["Depot indexe autorise"]
    B --> D["Snapshot graph.bin"]
    B --> E["Lecture source bornee"]
    D --> F["Noeuds et relations"]
    E --> G["Extraits de code"]
```

Le backend connait les depots indexes, normalise les chemins, refuse les traversals (`..`) et renvoie uniquement des donnees liees au repo selectionne.

## UX cible

Ajouter un espace de travail a onglets dans `chat-ui`:

```text
Chat | Sources | Graphe | Recherche
```

### Onglet Chat

Le chat reste l'ecran principal. Les ameliorations attendues:

- les citations de fichiers deviennent cliquables;
- un clic sur `Controllers/CourrierController.cs:120` ouvre l'onglet Sources a la bonne ligne;
- un clic sur un symbole inline ouvre le noeud correspondant dans le Graphe;
- une selection dans Sources ou Graphe peut etre envoyee au chat avec "poser une question sur cette selection".

### Onglet Sources

Disposition recommandee:

```text
Arborescence fichiers | Code source | Outline symboles / contexte graphe
```

Fonctions:

- arbre de fichiers filtre par recherche;
- preview read-only avec numeros de ligne et coloration;
- ouverture directe a une ligne ou plage de lignes;
- liens vers symboles detectes dans le graphe;
- copie du chemin relatif;
- bouton "Demander au chat".

### Onglet Graphe

Disposition recommandee:

```text
Filtres / lenses | Graphe interactif | Inspecteur de noeud
```

Fonctions:

- vue voisinage autour d'un noeud;
- profondeur 1/2/3;
- filtres par type de noeud et relation;
- recherche de symbole;
- "ouvrir le fichier";
- "analyse d'impact";
- "generer diagramme Mermaid";
- "demander au chat".

### Recherche globale

Une palette `Ctrl+K` devrait chercher:

- fichiers;
- classes;
- methodes;
- actions MVC;
- services;
- repositories;
- noeuds du graphe;
- pages de documentation generees.

## API HTTP proposee pour `chat-ui`

Les noms exacts peuvent evoluer, mais la surface devrait rester petite et read-only au depart.

| Endpoint | Role |
| --- | --- |
| `GET /api/repos` | Liste des projets indexes |
| `GET /api/repos/{repo}/files?path=` | Arborescence bornee au repo |
| `GET /api/repos/{repo}/source?path=&start=&end=` | Lecture d'un fichier ou extrait |
| `GET /api/repos/{repo}/symbols?q=&limit=` | Recherche de symboles |
| `GET /api/repos/{repo}/graph?zoom=&max_nodes=` | Graphe global limite |
| `GET /api/repos/{repo}/graph/neighborhood?node_id=&depth=` | Sous-graphe autour d'un noeud |
| `GET /api/repos/{repo}/context?node_id=` | Callers, callees, imports, communaute |
| `GET /api/repos/{repo}/impact?node_id=&direction=&depth=` | Impact amont/aval |

La premiere implementation peut reutiliser les memes types que le desktop:

- `FileTreeNode`
- `FileContent`
- `CytoNode`
- `CytoEdge`
- `GraphPayload`
- `SearchResult`
- `SymbolContext`

## Securite minimale requise

Avant d'exposer les sources au navigateur:

- accepter uniquement un `repo` connu par `/api/repos`;
- convertir les ids publics en chemins internes cote serveur;
- canonicaliser `repo_root + path`;
- refuser tout chemin qui sort du repo;
- ne jamais renvoyer de chemin absolu si `repoPathsExposed` est desactive;
- limiter la taille des fichiers lus;
- limiter `max_nodes` et la profondeur des graphes;
- garder CORS strictement local;
- journaliser sans secrets ni tokens.

## Plan d'implementation

### Phase 1: API read-only source/graphe

Ajouter les endpoints HTTP read-only dans `gitnexus serve` en reutilisant les fonctions deja presentes cote Tauri quand c'est possible.

Validation:

- tests unitaires path traversal;
- tests sur repo indexe minimal;
- tests `GET /api/repos`, `files`, `source`, `symbols`, `neighborhood`.

### Phase 2: citations cliquables dans le chat

Transformer les sources et chemins de fichier dans les reponses en actions:

- ouvrir fichier;
- ouvrir symbole dans graphe;
- copier chemin;
- envoyer au chat.

Validation:

- tests React sur `Markdown`;
- test d'une reponse contenant `foo/bar.cs:42`;
- pas de boucle de render.

### Phase 3: onglet Sources

Construire une version web de `FileTreeView` + `FilePreview`:

- API HTTP au lieu de Tauri IPC;
- virtualisation si gros arbre;
- preservation de l'etat par repo.

Validation:

- arbre vide;
- gros arbre;
- fichier introuvable;
- fichier trop volumineux;
- affichage mobile.

### Phase 4: onglet Graphe

Porter le coeur de `GraphExplorer` en version web:

- commencer par le voisinage d'un noeud plutot que le graphe complet;
- limiter a 200 noeuds par defaut;
- lazy expand sur double-clic;
- inspecteur de noeud a droite.

Validation:

- graphe vide;
- noeud sans relations;
- limite `max_nodes`;
- focus depuis un lien de chat.

### Phase 5: experience unifiee

Relier les trois surfaces:

```mermaid
flowchart TD
    A["Question chat"] --> B["Reponse avec sources"]
    B --> C["Ouvrir source a la ligne"]
    B --> D["Ouvrir noeud graphe"]
    C --> E["Demander au chat sur ce fichier"]
    D --> E
    D --> F["Analyse d'impact"]
    F --> G["Diagramme Mermaid"]
```

Le resultat attendu ressemble davantage a DeepWiki: on lit une reponse, on clique dans les preuves, on navigue dans le graphe, puis on relance une question contextualisee.

## Reutilisation recommandee

Priorite de reutilisation:

1. Reprendre les types de `tauri-commands.ts` dans un module web API separe.
2. Extraire les composants purs quand ils ne dependent pas de Tauri.
3. Garder deux adaptateurs:
   - `desktopAdapter` pour Tauri IPC;
   - `webAdapter` pour HTTP.
4. Ne pas dupliquer la logique de securite cote frontend: le backend reste l'autorite.

Fichiers candidats:

- `crates/gitnexus-desktop/ui/src/components/files/FileTreeView.tsx`
- `crates/gitnexus-desktop/ui/src/components/files/FilePreview.tsx`
- `crates/gitnexus-desktop/ui/src/components/graph/GraphExplorer.tsx`
- `crates/gitnexus-desktop/ui/src/components/graph/NodeHoverCard.tsx`
- `crates/gitnexus-desktop/ui/src/lib/graph-adapter.ts`
- `crates/gitnexus-desktop/ui/src/components/chat/ChatMarkdown.tsx`
- `chat-ui/src/components/ui/Markdown.tsx`

## Decision conseillee

Commencer par `chat-ui`, pas par une fusion complete avec le desktop.

Raison:

- le chat web est deja l'interface que Patrice teste sur `localhost:5174/5176`;
- le besoin immediat est de cliquer les sources d'une reponse;
- un explorateur read-only HTTP est plus simple a securiser qu'un portage complet de Tauri;
- le desktop peut rester la surface avancee pendant que le web gagne les fonctions essentielles.

## Critere de succes

Une premiere version est satisfaisante quand:

- une question chat cite des fichiers;
- un clic ouvre le fichier a la bonne ligne;
- un clic sur un symbole ouvre son voisinage graphe;
- le graphe permet de revenir au source;
- la selection source/graphe peut etre envoyee au chat;
- le backend refuse les chemins hors repo;
- le tout fonctionne sur un projet ASP.NET MVC indexe comme Alise_v2.
