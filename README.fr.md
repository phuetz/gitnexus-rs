# GitNexus

Intelligence de code basée sur un graphe de connaissances pour agents IA. GitNexus construit un graphe à partir de votre code source et l'expose via [MCP](https://modelcontextprotocol.io/) (Model Context Protocol) pour l'analyse de code assistée par IA.

Écrit en Rust. Supporte 14 langages de programmation. Livré avec une application desktop et un générateur de documentation HTML.

[English version](README.md)

## Fonctionnalités

- **Graphe de connaissances** — Parse le code source en un graphe riche de symboles (fonctions, classes, modules, imports, appels, héritage) avec 50+ types de nœuds et relations typées
- **14 Langages** — JavaScript, TypeScript, Python, Java, C, C++, C#, Go, Rust, Ruby, PHP, Kotlin, Swift, Razor via tree-sitter
- **Support avancé ASP.NET MVC 5** — Controllers, actions, vues Razor, Entity Framework 6 EDMX, grilles Telerik/Kendo, mapping jQuery/AJAX, détection couche service/repository (voir ci-dessous)
- **Générateur de documentation HTML** — Site HTML mono-page style DeepWiki avec recherche plein texte (Ctrl+K), coloration syntaxique, boutons copier, callouts, breadcrumbs, navigation Précédent/Suivant, scroll spy TOC, responsive mobile
- **Enrichissement LLM** — Mode `--enrich` optionnel qui augmente la documentation avec de la prose LLM grounded, des payloads JSON structurés, des citations avec provenance, et une validation anti-hallucination
- **Interroger le code** — Commande CLI `gitnexus ask "question"` pour du Q&A basé sur le graphe avec réponses en streaming
- **Application Desktop** — Application Tauri v2 avec visualisation interactive du graphe, vue treemap, chat intelligent et palette de commandes (Ctrl+K)
- **Chat Intelligent** — Q&A de code assisté par IA avec réponses en streaming, analyse de complexité des requêtes, plans de recherche multi-étapes et mode recherche approfondie. Supporte Ollama, OpenAI, Anthropic, OpenRouter et Gemini (avec mode raisonnement)
- **Serveur MCP** — 7 outils accessibles à tout agent IA compatible MCP (Claude, Cursor, VS Code, etc.)
- **Recherche Hybride** — Recherche lexicale BM25 + embeddings sémantiques ONNX optionnels, fusionnés par Reciprocal Rank Fusion
- **Analyse d'Impact** — Trace les appelants amont, les appelés aval et l'impact transitif de tout symbole
- **Modes Interactifs** — Shell REPL, dashboard TUI, surveillance de fichiers avec réindexation automatique
- **Stockage Modulaire** — Backend en mémoire (par défaut) ou base de données graphe KuzuDB

## Support ASP.NET MVC 5 / Legacy .NET

GitNexus offre un support approfondi des projets ASP.NET MVC 5 legacy, idéal pour documenter et comprendre des applications d'entreprise complexes.

### Ce qu'il détecte

| Fonctionnalité | Détection |
|----------------|-----------|
| **Controllers & Actions** | Héritage de classes, `[HttpGet/Post]`, `[GridAction]`, templates de routes, signatures de paramètres |
| **Vues Razor** (.cshtml) | `@model`, `@layout`, `@Html.Partial`, `@Html.RenderAction`, `@Html.BeginForm` |
| **Entity Framework 6** | DbContext, DbSet, entités EDMX, associations, propriétés de navigation, héritage (TPH/TPT) |
| **Telerik / Kendo UI** | `Html.Telerik().Grid<T>()`, `Html.Kendo().Grid<T>()`, bindings DataSource (`.Select()`, `.Read()`), colonnes de grille, `ClientEvents`, `DatePickerFor`, `DropDownListFor` |
| **jQuery / AJAX** | `$.ajax()`, `$.getJSON()`, `$.post()`, `$.get()`, `$.load()`, `fetch()`, `@Url.Action()` — liés aux actions des controllers |
| **Couche Service** | Classes `*Service`, `*Repository`, `*Manager`, `*Provider`, `*UnitOfWork` avec détection d'interfaces |
| **Injection de Dépendances** | Autofac (`RegisterType<T>().As<I>()`), Unity, Ninject, MS DI |
| **Attributs Personnalisés** | `[AuthorizeADAttribute]`, `[VerifActionFilter]`, tout `[*Attribute]`, `[*Filter]`, `[*Action]` |
| **Services Externes** | Détection de clients WebAPI (`new CMCASClient(httpClient)`), références WCF, traçage d'appels HTTP |
| **Traçabilité StackLogger** | Analyse de couverture — identifie les méthodes instrumentées avec `BeginMethodScope()` |
| **Controllers de Base** | Héritage de controllers personnalisés (`RootController` → `Controller`) |
| **Web.config** | Détection des fichiers de configuration |

### Documentation générée

La commande `generate html` produit un site de documentation HTML style DeepWiki :

```bash
gitnexus analyze D:\chemin\vers\mon-projet-mvc5
gitnexus generate --path D:\chemin\vers\mon-projet-mvc5 html
# Ouvrir .gitnexus/docs/index.html dans le navigateur
```

Le site HTML inclut :
- **Vue d'ensemble** avec stack technique, structure des projets et métriques
- **Diagramme d'architecture** (Mermaid) montrant les couches Présentation → Logique Métier → Accès aux Données
- **Pages par controller** avec signatures des actions, paramètres (liés au modèle de données), appelants et code source
- **Pages modèle de données** avec diagrammes de relations par entité et par domaine métier
- **Guide fonctionnel** avec descriptions métier en français, niveaux de criticité et diagrammes de flux Mermaid
- **Page services externes** avec signatures complètes des méthodes WebAPI incluant les surcharges
- **Vues & Templates** groupées par écran, filtrées par type (grilles, formulaires, vues partielles)
- **Couche service** avec descriptions et liens "Utilisé par" vers les controllers
- **Diagrammes de séquence** pour les flux critiques (recherche bénéficiaire, création dossier, export comptable)
- **Thème sombre/clair** avec recherche dans la sidebar et navigation Précédent/Suivant

## Démarrage Rapide

### Prérequis

- Rust 1.75+ (installer via [rustup](https://rustup.rs/))
- Un compilateur C (nécessaire pour la compilation des grammaires tree-sitter)
- Node.js 18+ (pour le frontend de l'application desktop uniquement)

### Compilation

```bash
git clone https://github.com/phuetz/gitnexus-rs.git
cd gitnexus-rs

# Compiler la CLI (mode release, optimisé)
cargo build --release -p gitnexus-cli

# Le binaire se trouve à :
# Windows : target\release\gitnexus.exe
# Linux/macOS : target/release/gitnexus
```

Des scripts de build sont aussi fournis :

```bash
# Windows
build-release.bat           # Compiler CLI + Desktop
build-release.bat cli       # CLI uniquement
build-release.bat desktop   # Desktop uniquement

# Linux/macOS
./build-release.sh          # Compiler CLI + Desktop
./build-release.sh cli      # CLI uniquement
```

### Compiler l'Application Desktop

```bash
cd crates/gitnexus-desktop/ui
npm install
npm run build
cd ../../..
cargo build -p gitnexus-desktop --release
```

Ou lancer en mode développement avec rechargement à chaud :

```bash
cd crates/gitnexus-desktop
cargo tauri dev
```

## Utilisation de la CLI

### Analyser un projet

```bash
# Indexer le répertoire courant
gitnexus analyze

# Indexer un chemin spécifique (ex: un projet ASP.NET MVC legacy)
gitnexus analyze D:\chemin\vers\projet

# Forcer la réindexation (réinitialise le graphe)
gitnexus analyze D:\chemin\vers\projet --force
```

Cela crée un répertoire `.gitnexus/` contenant le graphe de connaissances sérialisé.

### Générer la documentation

```bash
# Générer le site HTML (recommandé)
gitnexus generate --path D:\chemin\vers\projet html
# → Ouvrir .gitnexus/docs/index.html dans le navigateur

# Générer avec enrichissement LLM (nécessite un LLM configuré)
gitnexus generate --path D:\chemin\vers\projet html --enrich
gitnexus generate --path D:\chemin\vers\projet html --enrich --enrich-profile strict
gitnexus generate --path D:\chemin\vers\projet html --enrich --enrich-lang en

# Tout générer (AGENTS.md, wiki, skills, docs, DOCX, HTML)
gitnexus generate --path D:\chemin\vers\projet all

# Générer des formats spécifiques
gitnexus generate --path D:\chemin\vers\projet docs     # Pages Markdown
gitnexus generate --path D:\chemin\vers\projet docx     # Document Word
gitnexus generate --path D:\chemin\vers\projet context   # AGENTS.md uniquement
gitnexus generate --path D:\chemin\vers\projet wiki      # Pages wiki
gitnexus generate --path D:\chemin\vers\projet skills    # Fichiers skills
```

### Interroger le code

```bash
# Poser une question en utilisant le graphe + LLM (réponse en streaming)
gitnexus ask "comment fonctionne le calcul des barèmes ?"
gitnexus ask "quels controllers appellent le WebAPI Erable ?" --path D:\taf\Alise_v2
```

### Rechercher & Explorer

```bash
# Recherche en langage naturel
gitnexus query "middleware d'authentification"

# Contexte 360° d'un symbole (appelants, appelés, imports, hiérarchie)
gitnexus context UserService

# Analyse de rayon d'impact
gitnexus impact handleRequest --direction both

# Requête Cypher brute
gitnexus cypher "MATCH (n:Function) RETURN n.name LIMIT 10"
```

### Modes interactifs

```bash
gitnexus shell         # REPL interactif avec auto-complétion
gitnexus dashboard     # Dashboard TUI avec navigation dans le graphe
gitnexus watch         # Surveillance & réindexation automatique
```

### Serveur MCP (pour agents IA)

```bash
# Transport stdio (pour Claude, Cursor, VS Code, etc.)
gitnexus mcp

# Configuration automatique MCP dans votre éditeur
gitnexus setup

# Serveur HTTP
gitnexus serve         # Port 3000 par défaut
```

### Autres commandes

```bash
gitnexus list          # Lister les dépôts indexés avec statistiques
gitnexus status        # Afficher le statut de l'index du dépôt courant
gitnexus clean         # Supprimer l'index
gitnexus clean --all   # Supprimer tous les dépôts indexés
gitnexus report        # Rapport combiné de santé du code (hotspots + couplage + ownership)
gitnexus report --json # Idem, en JSON
```

### Exemple complet (projet ASP.NET MVC)

```bash
# 1. Compiler la CLI
cargo build --release -p gitnexus-cli

# 2. Analyser le projet
.\target\release\gitnexus.exe analyze D:\taf\MonAppLegacy

# 3. Générer la documentation HTML
.\target\release\gitnexus.exe generate --path D:\taf\MonAppLegacy html

# 4. Ouvrir dans le navigateur
start D:\taf\MonAppLegacy\.gitnexus\docs\index.html

# 5. Ou lancer l'application desktop pour une exploration interactive
.\target\release\gitnexus-desktop.exe
```

## Application Desktop

L'application desktop GitNexus est une application Tauri v2 avec un frontend React 19. Elle fournit une interface visuelle pour explorer le graphe de connaissances de votre code et un système de chat intelligent pour l'analyse de code assistée par IA.

### Explorateur de Graphe

Visualisation interactive du graphe propulsée par Cytoscape.js avec trois niveaux de zoom (package, module, symbole), plusieurs algorithmes de disposition et navigation au clic. Sélectionnez n'importe quel nœud pour voir son contexte complet : appelants, appelés, imports, exports et chaîne d'héritage.

### Chat Intelligent

Le système de chat est la fonctionnalité principale de l'application desktop. Il va au-delà du simple Q&A en analysant la complexité des requêtes et en exécutant des plans de recherche multi-étapes quand nécessaire.

**Analyse de Complexité** — Chaque question est classifiée en Simple (recherche directe), Moyenne (2-3 opérations), ou Complexe (DAG multi-étapes). Le système détecte les intentions en français et en anglais.

**Plans de Recherche Multi-Étapes** — Pour les requêtes moyennes et complexes, le planificateur génère un DAG de recherche avec suivi de dépendances.

**Mode Recherche Approfondie** (Ctrl+Shift+D) — Force une analyse multi-étapes complète quelle que soit la complexité de la requête.

**Filtrage Style IDE** — Ciblez vos questions sur des parties spécifiques du code :
- **Sélecteur de Fichiers** (Ctrl+P) — Recherche floue de fichiers
- **Sélecteur de Symboles** (Ctrl+Shift+O) — Recherche de symboles par type
- **Sélecteur de Modules** — Sélection par communautés

### Raccourcis Clavier

| Raccourci | Action |
|-----------|--------|
| Ctrl+K | Palette de commandes |
| Ctrl+B | Afficher/masquer la sidebar |
| Ctrl+1-5 | Changer d'onglet (Dépôts, Fichiers, Graphe, Impact, Docs) |
| Ctrl+\\ | Fermer le panneau de détails |
| Ctrl+Shift+D | Basculer le mode recherche approfondie |
| F | Ajuster le graphe à l'écran |
| L | Changer d'algorithme de disposition |
| 1 / 2 / 3 | Changer de niveau de zoom (package / module / symbole) |
| Escape | Fermer les modales, désélectionner |

## Outils MCP

En mode serveur MCP, GitNexus expose ces outils :

| Outil | Description |
|-------|-------------|
| `list_repos` | Lister les dépôts indexés avec statistiques |
| `query` | Recherche en langage naturel dans le graphe |
| `context` | Vue 360° d'un symbole : appelants, appelés, imports, exports, hiérarchie |
| `impact` | Analyse de rayon d'impact — amont, aval ou les deux |
| `detect_changes` | Analyser les changements non committés et leur impact |
| `rename` | Trouver toutes les références à mettre à jour pour un renommage |
| `cypher` | Exécuter une requête Cypher en lecture seule |

## Langages Supportés

| Langage | Extensions |
|---------|------------|
| JavaScript | `.js` `.jsx` `.mjs` `.cjs` |
| TypeScript | `.ts` `.tsx` `.mts` `.cts` |
| Python | `.py` `.pyi` |
| Java | `.java` |
| C | `.c` `.h` |
| C++ | `.cpp` `.hpp` `.cc` `.hh` `.cxx` `.hxx` |
| C# | `.cs` `.cshtml` `.edmx` `.config` |
| Go | `.go` |
| Rust | `.rs` |
| Ruby | `.rb` |
| PHP | `.php` |
| Kotlin | `.kt` `.kts` |
| Swift | `.swift` |
| Razor | `.cshtml` `.razor` |

## Licence

PolyForm Noncommercial 1.0.0
