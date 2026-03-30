# TODO — Améliorations du Rapport HTML

> Propositions pour rapprocher la documentation générée du niveau DeepWiki / Code Buddy.
> Classées par priorité et effort. On code ce soir.

---

## 🔴 PRIORITÉ 1 — Enrichissement LLM du contenu (le plus gros impact)

### 1.1 Mode `--enrich` : réécriture par LLM de chaque page
**Effort :** 3-4h | **Impact :** Transforme la doc de "dump technique" en "wiki lisible"

Ajouter un flag `--enrich` à `gitnexus generate html` qui :
1. Génère les pages markdown normalement (comme aujourd'hui)
2. Pour chaque page `.md`, envoie le contenu + le contexte du graphe au LLM
3. Le LLM réécrit avec :
   - Un résumé exécutif en prose (pas de table brute)
   - Des explications "pourquoi" (pas juste "quoi")
   - Des transitions entre sections
   - Des "Points d'attention" pour le développeur
4. Sauvegarde la version enrichie

**Prompt template par type de page :**
```
Tu es un expert ASP.NET MVC 5. Réécris cette documentation technique
pour qu'elle soit compréhensible par un responsable de service qui
reprend l'application.

RÈGLES :
- Ne jamais inventer de noms de classes/méthodes qui ne sont pas dans le contenu
- Commencer par un résumé de 2-3 phrases
- Expliquer le "pourquoi" de chaque flux
- Ajouter des "⚠️ Point d'attention" quand pertinent
- Garder les tableaux et diagrammes Mermaid existants
- Écrire en français

CONTENU À ENRICHIR :
{page_content}
```

**Fichier :** `generate.rs` — nouvelle fonction `enrich_with_llm(page_path, config)`
**Appel LLM :** Réutiliser la même mécanique que `chat.rs` (`call_llm` avec `ChatConfig`)

### 1.2 Résumés exécutifs automatiques par LLM
**Effort :** 1h | **Impact :** Élevé

Pour chaque page controller, envoyer la liste des actions + paramètres au LLM et lui demander un résumé de 3 lignes en français. Insérer en haut de la page.

**Exemple attendu :**
> Le DossiersController gère le cycle de vie des dossiers d'aide sociale.
> Il permet la recherche, création, modification et export des dossiers,
> avec un calcul automatique des montants via les barèmes et plafonds.

### 1.3 Diagrammes Mermaid générés par LLM
**Effort :** 2h | **Impact :** Moyen-Élevé

Au lieu de nos heuristiques (Recherche→Consultation→Création→Export), envoyer la liste des actions au LLM et lui demander de produire un diagramme Mermaid pertinent qui montre le vrai flux métier.

**Prompt :**
```
Voici les actions du DossiersController : [liste]
Génère UN diagramme Mermaid flowchart (max 8 nœuds) montrant
le flux principal d'un utilisateur. Pas de détails techniques,
juste le parcours métier.
```

---

## 🟡 PRIORITÉ 2 — Améliorations HTML / UX

### 2.1 Recherche plein texte dans tout le contenu
**Effort :** 2h | **Impact :** Élevé

Actuellement le search filtre les noms de pages. Ajouter une vraie recherche plein texte :
1. Au moment de la génération, construire un index JSON de tous les mots par page
2. Côté client, chercher dans cet index avec highlighting des résultats
3. Afficher les résultats dans un overlay avec contexte (snippet de 50 chars autour du match)

**Approche :** Générer un `search-index.json` embarqué dans le HTML, avec un champ `content` par page (texte nettoyé sans HTML). Utiliser un simple `indexOf` côté JS — pas besoin de librairie externe pour 40 pages.

### 2.2 Scroll spy sur la TOC droite
**Effort :** 30min | **Impact :** Moyen

La TOC droite doit highlighter la section visible pendant le scroll.

```javascript
const observer = new IntersectionObserver(entries => {
  entries.forEach(e => {
    const link = document.querySelector(`[href="#${e.target.id}"]`);
    if (link) link.classList.toggle('active', e.isIntersecting);
  });
}, { threshold: 0.5 });
document.querySelectorAll('h2, h3').forEach(h => observer.observe(h));
```

### 2.3 Temps de lecture par section
**Effort :** 30min | **Impact :** Faible

Afficher "~3 min" à côté de chaque entrée de la TOC. Calcul : nombre de mots / 200.

### 2.4 Code blocks collapsibles (> 20 lignes)
**Effort :** 1h | **Impact :** Moyen

Les blocs de code source des méthodes (jusqu'à 50 lignes) prennent beaucoup de place. Les wrapper automatiquement dans un `<details>` si > 20 lignes.

```html
<details class="code-block">
  <summary>Code source (42 lignes)</summary>
  <pre><code>...</code></pre>
</details>
```

### 2.5 Syntax highlighting avec Highlight.js
**Effort :** 1h | **Impact :** Moyen

Actuellement les blocs `<pre><code>` n'ont pas de coloration syntaxique. Ajouter Highlight.js via CDN (comme Mermaid) pour coloriser C#, JavaScript, SQL.

```html
<script src="https://cdn.jsdelivr.net/npm/highlight.js/lib/core.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/highlight.js/lib/languages/csharp.min.js"></script>
<script>hljs.highlightAll();</script>
```

### 2.6 Export PDF propre
**Effort :** 1h | **Impact :** Moyen

Ajouter un bouton "📄 Exporter en PDF" qui utilise `window.print()` avec un CSS `@media print` optimisé : masquer sidebar/TOC, utiliser des fonts serif, ajouter en-têtes/pieds de page.

### 2.7 Breadcrumbs visibles
**Effort :** 30min | **Impact :** Faible

Ajouter un fil d'Ariane en haut du contenu principal : `Documentation > Controllers > DossiersController`

---

## 🟢 PRIORITÉ 3 — Améliorations du contenu (sans LLM)

### 3.1 Page "Vue d'ensemble rapide" (one-pager)
**Effort :** 2h | **Impact :** Élevé

Une seule page qui résume TOUT le projet en 1 écran :
- Diagramme d'architecture (déjà fait)
- Tableau des controllers avec nombre d'actions (condensé)
- Top 5 entités les plus connectées
- Liste des services externes
- Stats clés (fichiers, noeuds, edges)

C'est la page que Florent ouvre en premier pour avoir une vue globale.

### 3.2 Liens entre pages (cross-references améliorées)
**Effort :** 1h | **Impact :** Moyen

Quand une page controller mentionne un service (ex: "DossierService"), le nom devrait être cliquable et naviguer vers la page services. Idem pour les vues mentionnées.

**Approche :** Après génération, scanner chaque page pour les noms connus (controllers, services, entités) et les transformer en liens.

### 3.3 Badges de complexité par controller
**Effort :** 30min | **Impact :** Faible

Ajouter un badge visuel en haut de chaque page controller :
- 🔴 **Complexe** (> 30 actions)
- 🟡 **Moyen** (10-30 actions)
- 🟢 **Simple** (< 10 actions)

### 3.4 Section "Fichiers modifiés récemment" (si git disponible)
**Effort :** 2h | **Impact :** Moyen

Si le projet est un dépôt git, analyser `git log` pour chaque fichier et ajouter :
- Date de dernière modification
- Nombre de commits
- Auteur principal

Ça permet d'identifier les fichiers "vivants" vs les fichiers abandonnés.

---

## 🔵 PRIORITÉ 4 — Améliorations visuelles

### 4.1 Icônes par type de page dans la sidebar
**Effort :** 30min | **Impact :** Faible

Ajouter des emoji/icônes devant chaque entrée de la sidebar :
- 📊 Overview
- 🏗️ Architecture
- 🎯 Guide Fonctionnel
- ⚙️ Controllers
- 💾 Data Model
- 🔌 Services
- 🌐 External Services
- 📄 Views

### 4.2 Graphe interactif miniature dans l'overview
**Effort :** 3h | **Impact :** Élevé (effet "wow")

Embarquer un petit graphe Cytoscape.js ou D3 dans la page overview montrant les 20 noeuds les plus connectés. Cliquable pour naviguer vers les pages correspondantes.

**Approche :** Générer un JSON des top noeuds + edges pendant la génération, l'embarquer dans le HTML, le rendre avec D3 force-directed.

### 4.3 Thème "paper" pour l'impression
**Effort :** 1h | **Impact :** Faible

Un troisième thème (en plus de dark/light) style "papier" avec fond crème, police serif, marges larges — pour les gens qui préfèrent lire sur fond clair avec une esthétique plus traditionnelle.

### 4.4 Animation de chargement des pages
**Effort :** 30min | **Impact :** Faible

Quand on clique sur une page dans la sidebar, ajouter un petit fade-in au lieu du remplacement instantané du contenu.

```javascript
content.style.opacity = 0;
setTimeout(() => {
  content.innerHTML = page.html;
  content.style.opacity = 1;
}, 100);
```

---

## 💡 IDÉES AVANCÉES (pour plus tard)

### 5.1 Chat intégré dans le site HTML
Embarquer le même chat que le desktop app directement dans l'index.html. L'utilisateur peut poser des questions sur le code sans quitter la doc. Nécessite un endpoint API ou un appel direct au LLM configuré.

### 5.2 Mode diff : comparer deux générations
Sauvegarder chaque génération avec un timestamp et permettre de comparer deux versions de la doc côté à côté. Utile pour voir l'évolution du projet après un refactoring.

### 5.3 Annotations collaboratives
Permettre d'ajouter des sticky notes sur les pages (stockées dans un fichier JSON local). Florent pourrait annoter "cette méthode est critique" ou "TODO: vérifier ce flux".

### 5.4 GraphRAG : questions sur le graphe via le site
Indexer le graphe pour le RAG et permettre des requêtes naturelles dans le site HTML : "quelles méthodes appellent le WebAPI Erable ?" → résultat avec liens vers les pages.

### 5.5 Génération multi-langue
Générer la doc en français ET en anglais. Le LLM peut traduire les pages enrichies.

---

## Résumé par effort (ce soir)

| Feature | Effort | Impact | Priorité |
|---------|--------|--------|----------|
| 1.1 Mode --enrich LLM | 3-4h | 🔴 Très élevé | Ce soir |
| 1.2 Résumés exécutifs LLM | 1h | 🔴 Élevé | Ce soir |
| 2.1 Recherche plein texte | 2h | 🟡 Élevé | Ce soir |
| 2.2 Scroll spy TOC | 30min | 🟡 Moyen | Ce soir |
| 2.5 Syntax highlighting | 1h | 🟡 Moyen | Ce soir |
| 3.1 One-pager overview | 2h | 🟢 Élevé | Ce soir |
| 4.1 Icônes sidebar | 30min | 🔵 Faible | Ce soir |
| 2.4 Code blocks collapsibles | 1h | 🟡 Moyen | Demain |
| 2.6 Export PDF | 1h | 🟡 Moyen | Demain |
| 1.3 Mermaid par LLM | 2h | 🔴 Moyen-Élevé | Demain |
| 3.2 Cross-references | 1h | 🟢 Moyen | Demain |
| 4.2 Graphe miniature | 3h | 🔵 Élevé | Plus tard |
| 5.1 Chat intégré HTML | 4h+ | 💡 Élevé | Plus tard |

**Total estimé ce soir : ~10h** (réaliste : choisir les 5-6 plus impactants)

---

*Basé sur l'analyse de : DeepWiki, Code Buddy (grok-cli), Docusaurus, Pagefind, Highlight.js, Shiki, Codapi, FlexSearch, CAST Imaging, Sourcegraph.*
