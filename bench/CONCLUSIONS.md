# Conclusions du sprint semantic search (2026-04-24)

Après 15 commits, 4 benchs, 2 modèles testés :

## Best config pour gitnexus-rs

**Graph propre (target-codex gitignored) + MiniLM-L6-v2 + hybrid RRF.**

Commandes :
```bash
# Une seule fois
echo "/target-codex" >> .gitignore
echo "/.codex-target" >> .gitignore
gitnexus analyze . --force                     # rebuild graph propre (452 files, 6533 nodes)
gitnexus embed --model ~/.gitnexus/models/all-MiniLM-L6-v2/model.onnx

# Ensuite
gitnexus query "ta recherche" --hybrid          # BM25 + semantic RRF
gitnexus query "ta recherche" --hybrid --rerank # + LLM re-ranking
```

## Leçons apprises

### 1. Le nettoyage du corpus > l'upgrade de modèle
48% du graph gitnexus-rs était des build artifacts. Un meilleur modèle (multilingue 470MB, 3× plus lent) améliore marginalement un corpus sale. Un corpus propre améliore tout, quel que soit le modèle.

**Ordre des priorités si tu refais l'exercice :**
1. Filter build artifacts à la source (gitignore + re-analyze)
2. Activer hybrid (BM25 + semantic RRF)
3. Activer LLM reranker si budget tokens OK
4. Upgrade modèle seulement si la langue du corpus change (FR/multilingue)

### 2. Les régressions Phase 2 étaient des artefacts de corpus sale
Les 4 régressions Q4/Q6/Q8/Q13 qu'on observait pré-cleanup ne venaient PAS de la dilution du signal content (Phase 2.4 a invalidé cette hypothèse). Elles venaient du bruit target-codex qui remontait parce que les embeddings sont "plus agressifs" sémantiquement que BM25.

**Morale** : quand un système retrieval régresse, inspect le corpus avant le modèle.

### 3. MiniLM-L6 suffit souvent, y compris sur FR
"chat_cancel" #2 en français ("où est gérée l'annulation du chat streaming") avec MiniLM anglais sur un corpus propre. Les mots partagés (Latin/tech English) suffisent pour la cross-lingual coincidental transfer. Le multilingue devient obligatoire seulement quand la query est en langue lointaine (chinois, arabe) ou que le corpus est 90%+ français non-technique.

### 4. Le reranker LLM pose son propre problème : la dépendance API
Gemini Flash a 503'd 4/15 queries dans le bench Phase 1. Fallback gracieux sur BM25 fonctionne mais nuit à la consistance. Pour un produit user-facing, un cross-encoder ONNX local (ms-marco-MiniLM) serait plus robuste. Mais ~80MB de plus sur disk.

## Comparaison 4-way finale (top-1, corpus propre)

| Q | BM25 pur | +Rerank | +Hybrid | +Hybrid+Rerank |
|---|---|---|---|---|
| 1  | merge_with_rrf ✓ | = (503) | ✓ | ✓ |
| 2  | rank ❌ | = (503) | rank (mais reranker module dans top-5) ⬆️ | avec rerank : à tester |
| 4  | generate_embeddings ✓ | ⬆️⬆️ | ⬆️⬆️⬆️ save/load_embeddings top-3 | idem |
| 8  | bruit | enrich_with_llm ⬆️⬆️ | **load_llm_config #1** ⬆️⬆️⬆️ | idem |
| 14 | chat_ask | chat_cancel #1 ⬆️ | **chat_cancel #1** ✓ | ✓ |
| 17 FR | n/a (pas dans baseline) | n/a | **chat_cancel #2** ⬆️ | à tester |

## Phase F toujours en suspens

Le vrai point de la vidéo (sous-agents isolés dans le chat desktop) n'a pas été adressé dans ce sprint. C'est un autre chantier, sur autre branche. Estimé 3-5j.

Ce que ce sprint a livré = la moitié "retrieval" du pipeline. La moitié "orchestration" (Phase F) reste à faire pour que le chat desktop utilise ce retrieval dans des sous-contextes isolés plutôt que de tout balancer dans le contexte principal.

## Chiffres finaux du sprint

- 14 commits, branche `feat/semantic-search`
- ~1200 lignes de Rust nouvelles (+ ~100 TS, + ~250 docs bench)
- 627 tests au vert, 0 régression sur master
- 2 binaires ajoutés : `gitnexus embed`, flags `--hybrid` et `--rerank` sur `gitnexus query`
- 2 features ajoutées : `embeddings`, `reranker-llm` sur gitnexus-search
- 3 surfaces wired : CLI, MCP tool search_code, Desktop Tauri search_symbols
- 2 modèles testés : MiniLM-L6 (default), paraphrase-multilingual (optionnel FR)
- 4 runs de bench complets sur 17 queries
