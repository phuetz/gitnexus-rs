# TODO — GitNexus Roadmap

> Propositions d'améliorations classées par priorité et effort.
> Recherche basée sur DeepWiki, Sourcegraph Cody, CAST Imaging, CodeScene, SonarQube, Cursor, Windsurf, Augment Code, CodeRabbit, Greptile.

---

## 🔴 PRIORITÉ 1 — Améliorations LLM (haute valeur, faisable ce soir)

### 1.1 Enrichissement LLM de la documentation générée
**Effort :** Moyen | **Impact :** Très élevé

Actuellement la doc est générée à partir du graphe sans LLM. Ajouter un mode `--enrich` qui appelle le LLM configuré (Gemini, Ollama, etc.) pour :
- Réécrire chaque page controller avec une description métier en prose
- Générer des résumés exécutifs pour l'overview et l'architecture
- Expliquer les flux métier complexes (cascade Domaine→GrpAide→Aide)
- Ajouter des "Developer Tips" et "Points d'attention" par module

**Approche :** Pour chaque page `.md` générée, envoyer le contenu + le contexte du graphe au LLM avec un prompt template, et remplacer le contenu par la version enrichie.

```bash
gitnexus generate --path D:\projet html --enrich
```

### 1.2 "Ask the Codebase" en CLI (Deep Search)
**Effort :** Moyen | **Impact :** Élevé

Ajouter une commande `gitnexus ask "comment fonctionne le calcul des barèmes ?"` qui :
1. Cherche dans le graphe les symboles pertinents (BM25 + graph traversal)
2. Lit les fichiers sources correspondants
3. Envoie le tout au LLM configuré
4. Affiche la réponse avec les références de fichiers

Inspiré de Sourcegraph Cody Deep Search et Greptile.

### 1.3 Génération de diagrammes Mermaid par le LLM
**Effort :** Faible | **Impact :** Moyen

Au lieu de générer les diagrammes Mermaid de façon heuristique, envoyer les données du graphe au LLM et lui demander de produire un diagramme lisible et pertinent. Le LLM comprend mieux ce qui est important à montrer.

---

## 🟡 PRIORITÉ 2 — Améliorations de la documentation HTML

### 2.1 Recherche plein texte dans le site HTML
**Effort :** Faible | **Impact :** Élevé

Actuellement le search ne filtre que les noms de pages dans la sidebar. Ajouter une vraie recherche plein texte côté client (index JSON pré-calculé) qui cherche dans le contenu de toutes les pages.

### 2.2 Table des matières globale (page index)
**Effort :** Faible | **Impact :** Moyen

Ajouter une page "Table des matières" listant toutes les pages avec un bref résumé, comme DeepWiki.

### 2.3 Scroll spy sur la TOC droite
**Effort :** Faible | **Impact :** Moyen

La TOC droite (table des matières de la page courante) devrait highlighter la section visible pendant le scroll.

### 2.4 Export PDF depuis le HTML
**Effort :** Moyen | **Impact :** Moyen

Bouton "Exporter en PDF" qui utilise `window.print()` avec un CSS d'impression optimisé.

### 2.5 Mode présentation
**Effort :** Moyen | **Impact :** Moyen

Un mode "slides" qui transforme chaque H2 en une slide pour présenter la doc à Florent/Virginie en réunion.

---

## 🟢 PRIORITÉ 3 — Analyse de code avancée

### 3.1 Détection de code mort (Dead Code Analysis)
**Effort :** Moyen | **Impact :** Élevé

Identifier les fonctions/méthodes qui ne sont jamais appelées dans le graphe. Pour un projet legacy comme Alise_v2, ça permettrait de savoir ce qui peut être supprimé en toute sécurité.

```bash
gitnexus dead-code D:\taf\Alise_v2
# Functions never called: 47
# Views never rendered: 12
# Services never injected: 3
```

### 3.2 Détection de dépendances circulaires
**Effort :** Moyen | **Impact :** Élevé

Analyser le graphe pour trouver les cycles (A → B → C → A). CAST Imaging le fait, c'est un indicateur clé de dette technique.

### 3.3 Score de complexité par module
**Effort :** Moyen | **Impact :** Moyen

Calculer un score de complexité basé sur : nombre de dépendances, profondeur d'appels, nombre de paramètres, taille des méthodes. Afficher dans le dashboard et la doc.

### 3.4 Détection de patterns dupliqués
**Effort :** Élevé | **Impact :** Moyen

Trouver les blocs de code similaires (copier-coller) pour identifier les opportunités de refactoring. CodeScene et SonarQube le font.

### 3.5 Analyse de couverture de tests
**Effort :** Moyen | **Impact :** Moyen

Croiser les fichiers de test (*.Tests.cs) avec les fichiers source pour montrer quels modules sont testés et lesquels ne le sont pas.

---

## 🔵 PRIORITÉ 4 — Desktop App

### 4.1 Intégration code review (PR GitHub)
**Effort :** Élevé | **Impact :** Très élevé

Connecter GitNexus à l'API GitHub pour analyser les PR et commenter automatiquement avec le contexte du graphe. CodeRabbit atteint 49% de précision — avec un graphe de connaissances, on peut faire mieux.

### 4.2 Historique Git dans le graphe
**Effort :** Moyen | **Impact :** Élevé

Intégrer `git log` dans le graphe pour montrer :
- Quels fichiers changent le plus (hotspots à la CodeScene)
- Qui modifie quoi (ownership)
- Les fichiers qui changent toujours ensemble (couplage temporel)
- L'évolution de la complexité dans le temps

### 4.3 Diff visuel entre deux analyses
**Effort :** Moyen | **Impact :** Moyen

Comparer deux snapshots du graphe pour voir ce qui a changé : nouveaux noeuds, noeuds supprimés, relations modifiées. Utile pour le suivi de refactoring.

### 4.4 Mode collaboratif (annotations)
**Effort :** Élevé | **Impact :** Moyen

Permettre aux utilisateurs d'ajouter des notes/annotations sur les noeuds du graphe, persistées dans un fichier `.gitnexus/annotations.json`. Pour que Florent puisse annoter "cette méthode est critique, ne pas toucher".

### 4.5 Dashboard de santé du projet
**Effort :** Moyen | **Impact :** Élevé

Un dashboard style SonarQube avec :
- Score de santé global (A/B/C/D/E)
- Tendance dans le temps
- Top 5 fichiers les plus problématiques
- Couverture de traçabilité (StackLogger)
- Ratio code mort

---

## ⚪ PRIORITÉ 5 — Infrastructure & Écosystème

### 5.1 Plugin VS Code / Cursor
**Effort :** Élevé | **Impact :** Très élevé

Extension VS Code qui affiche le contexte du graphe directement dans l'éditeur : hover cards avec appelants/appelés, impact analysis dans la gutter, navigation au graphe.

### 5.2 API REST pour le graphe
**Effort :** Moyen | **Impact :** Élevé

Exposer le graphe via une API REST (en plus du MCP) pour permettre à d'autres outils de l'interroger. Endpoints : `/nodes`, `/relationships`, `/search`, `/impact/{nodeId}`.

### 5.3 Indexation incrémentale (watch mode amélioré)
**Effort :** Élevé | **Impact :** Moyen

Le `watch` mode existe mais réindexe tout. Implémenter une vraie indexation incrémentale qui ne re-parse que les fichiers modifiés et met à jour le graphe en delta.

### 5.4 Support multi-repo
**Effort :** Moyen | **Impact :** Moyen

Analyser plusieurs repos et montrer les dépendances cross-repo (ex: une lib partagée utilisée par 3 projets).

### 5.5 Export GraphML / Neo4j
**Effort :** Faible | **Impact :** Moyen

Exporter le graphe au format GraphML ou Cypher pour import dans Neo4j, Gephi, ou d'autres outils de visualisation de graphes.

---

## 💡 IDÉES EXPLORATOIRES (R&D)

### 6.1 Génération automatique de tests unitaires
Utiliser le graphe + LLM pour générer des tests unitaires pour les méthodes non couvertes. Le graphe fournit le contexte (dépendances, types, appels) et le LLM génère le code de test.

### 6.2 Refactoring assisté par IA
Proposer des refactorings basés sur les patterns détectés : extraction de méthode, inversion de dépendance, split de controller trop gros. Le graphe identifie les candidats, le LLM propose le code.

### 6.3 Documentation conversationnelle
Un chatbot intégré dans le site HTML qui répond aux questions sur le code en utilisant le graphe comme source de vérité. Déjà dans le desktop, à porter sur le web.

### 6.4 Analyse de sécurité
Scanner le graphe pour des patterns de sécurité : injection SQL (requêtes Cypher/LINQ non paramétrées), XSS (données utilisateur dans les vues), secrets hardcodés, endpoints non authentifiés.

### 6.5 Migration assistant
Pour les projets legacy : analyser le graphe et proposer un plan de migration (ex: ASP.NET MVC 5 → .NET 8 Minimal API) avec estimation d'effort par module.

---

## Résumé par effort

| Catégorie | Faible effort | Moyen effort | Gros effort |
|-----------|---------------|--------------|-------------|
| **LLM** | 1.3 Mermaid LLM | 1.1 Enrichissement, 1.2 Ask CLI | - |
| **Doc HTML** | 2.1 Search, 2.2 TOC, 2.3 Scroll spy | 2.4 PDF, 2.5 Slides | - |
| **Analyse** | - | 3.1 Dead code, 3.2 Cycles, 3.3 Complexité, 3.5 Tests | 3.4 Duplications |
| **Desktop** | - | 4.2 Git history, 4.3 Diff, 4.5 Dashboard | 4.1 Code review, 4.4 Annotations |
| **Infra** | 5.5 Export GraphML | 5.2 API REST, 5.4 Multi-repo | 5.1 VS Code, 5.3 Incrémental |

---

*Généré le 30/03/2026 — basé sur l'analyse de DeepWiki, Sourcegraph Cody, CAST Imaging, CodeScene, SonarQube, Cursor, Windsurf, Augment Code, CodeRabbit, Greptile, Mintlify.*
