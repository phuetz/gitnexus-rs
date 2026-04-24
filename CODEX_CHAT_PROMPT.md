# Prompt pour Codex — Amélioration du Chat GitNexus

## Contexte

Tu travailles sur le chat de **GitNexus** — un assistant d'intelligence de code.
Le projet est dans `crates/gitnexus-desktop/src/commands/chat.rs`.

L'objectif principal : **quand un utilisateur pose une question sur un traitement ou un algorithme, la réponse doit commencer par un organigramme Mermaid détaillé**, pas juste du texte.

Exemple de question : "Comment sont générés les courriers en masse ?"
Réponse attendue : un `flowchart TD` Mermaid avec les étapes, conditions if/else, accès BDD, cas d'erreur — PUIS l'explication textuelle.

## Ce qui est déjà implémenté

Lis `chat.rs` pour comprendre l'existant :

1. **`classify_question()`** — classifie la question en 5 types (Lookup/Functional/Algorithm/Architecture/Impact)
2. **`canvas_instruction()`** — retourne un template de réponse par type
3. **`prefetch_for_type()`** — pré-charge les outils avant l'appel LLM :
   - Pour Algorithm : lit la chaîne d'appels complète (5 méthodes × 250 lignes via `read_full_method`)
   - Pour Architecture : liste les modules + génère un diagramme
   - Pour Impact : lance `get_impact`
4. **`load_enriched_doc_pages()`** — charge les pages .md enrichies du dossier `.gitnexus/docs/`
5. **`build_skeleton_flowchart()`** dans `diagram.rs` — génère un squelette topologique Mermaid
6. **Tool `read_method`** — lit une méthode complète (250 lignes, pas de troncature)

## Ce que tu dois implémenter

### TÂCHE 1 — `detect_target_symbol()` (nouvelle fonction dans chat.rs)

Extrait le symbole/module principal d'une question pour cibler le prefetch.

```rust
fn detect_target_symbol(question: &str) -> Option<String> {
    // Exemples:
    // "Comment fonctionne le module Courrier ?" → Some("Courrier")
    // "Explique DossiersController" → Some("DossiersController")
    // "Comment sont calculés les plafonds ?" → None
    // "Présente le module Elodie" → Some("Elodie")
}
```

Stratégie : chercher les mots après "module", "classe", "controller", "service", "méthode", "le", "la", "l'" suivis d'un mot commençant par une majuscule ou contenant "Controller"/"Service"/"Manager".

Utilise ce résultat dans `prefetch_for_type` pour cibler `get_symbol_context` et `read_full_method` sur le bon symbole plutôt que le top résultat FTS.

### TÂCHE 2 — `search_processes` arm dans `execute_mcp_tool()`

Ajoute un nouveau tool pour interroger les processus métier du graphe :

```rust
"search_processes" => {
    let query = args["query"].as_str().unwrap_or("");
    // Exécute : MATCH (n:Process) WHERE n.name CONTAINS '<query>'
    //           OR n.description CONTAINS '<query>'
    //           RETURN n.name, n.description, n.step_count
    //           ORDER BY n.step_count DESC LIMIT 10
}
```

Ajoute aussi la `ToolDefinition` correspondante dans la liste des outils LLM (autour de la ligne get_diagram).

Description : "Search business process flows in the graph. Use when the question involves a workflow, business process, or multi-step operation."

### TÂCHE 3 — Tests unitaires pour `build_skeleton_flowchart` dans `diagram.rs`

Ajoute des tests `#[cfg(test)]` à la fin de `diagram.rs` :

```rust
#[cfg(test)]
mod tests {
    // Test 1: build_skeleton_flowchart retourne "flowchart TD" pour un graphe vide
    // Test 2: build_skeleton_flowchart inclut le nom du symbole de départ
    // Test 3: DiagramKind::parse("sequence") retourne Sequence
    // Test 4: DiagramKind::parse("unknown") retourne Flowchart (défaut)
}
```

### TÂCHE 4 — Améliorer `prefetch_for_type` pour Functional

Pour `QuestionType::Functional`, si `detect_target_symbol()` retourne un nom de module :
1. Chercher le fichier `.gitnexus/docs/modules/{nom}.md` ou `.gitnexus/docs/modules/ctrl-{nom}.md`
2. Le lire directement (plutôt que passer par `load_enriched_doc_pages`)
3. Injecter comme "## Documentation du module [Nom]"

Cela donne au LLM la page enrichie exacte plutôt qu'une approximation FTS.

## Règles à respecter

- Ne casse pas les tests existants (57 tests passent actuellement)
- Pas de `unwrap()` sans gestion d'erreur
- Chaque nouvelle fonction : commentaire doc `///`
- Compilation : `cargo build -p gitnexus-desktop` doit passer sans erreur

## Vérification finale

```bash
cargo build -p gitnexus-desktop
cargo test -p gitnexus-desktop --lib
```

Les 57 tests existants doivent passer + les nouveaux tests de diagram.rs.
