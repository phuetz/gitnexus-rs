//! Business process documentation generator (Alise-specific).

use std::io::Write;
use std::path::Path;

use anyhow::Result;
use colored::Colorize;

use gitnexus_core::graph::KnowledgeGraph;

/// Generate business-specific process documentation (B1-B5).
pub(super) fn generate_business_docs(graph: &KnowledgeGraph, docs_dir: &Path) -> Result<usize> {
    let processes_dir = docs_dir.join("processes");
    if !processes_dir.exists() {
        std::fs::create_dir_all(&processes_dir)?;
    }

    let mut count = 0;

    // B1: System of Letters (Courriers)
    if generate_courriers_doc(graph, &processes_dir)? {
        count += 1;
    }

    // B2: Payment Cycle
    if generate_paiements_doc(graph, &processes_dir)? {
        count += 1;
    }

    // B3: Barème Calculation Engine
    if generate_baremes_doc(graph, &processes_dir)? {
        count += 1;
    }

    // B4: Financial Entities
    if generate_financial_entities_doc(graph, &processes_dir)? {
        count += 1;
    }

    // B5: Suppliers Management
    if generate_suppliers_doc(graph, &processes_dir)? {
        count += 1;
    }

    Ok(count)
}

fn generate_courriers_doc(graph: &KnowledgeGraph, dir: &Path) -> Result<bool> {
    let has_courrier = graph
        .iter_nodes()
        .any(|n| n.properties.name.contains("Courrier"));
    if !has_courrier {
        return Ok(false);
    }

    let out_path = dir.join("courriers.md");
    let mut f = std::fs::File::create(&out_path)?;

    writeln!(f, "# Système de Courriers")?;
    writeln!(f, "<!-- GNX:LEAD -->")?;
    writeln!(f)?;
    writeln!(
        f,
        "> Ce module gère la génération des 11 types de courriers officiels de l'application."
    )?;
    writeln!(f)?;

    writeln!(f, "## Types de courriers")?;
    writeln!(f, "<!-- GNX:INTRO:types-courriers -->")?;
    writeln!(f)?;
    writeln!(f, "| Type | Usage |")?;
    writeln!(f, "|------|-------|")?;
    writeln!(f, "| Accord | Lettre d'accord pour approbation d'aide |")?;
    writeln!(f, "| Refus | Lettre de refus de demande |")?;
    writeln!(f, "| Rejet | Lettre de rejet (changement de statut) |")?;
    writeln!(f, "| TarifApplique | Notification du tarif appliqué |")?;
    writeln!(
        f,
        "| DemandeJustificatif | Demande de pièces justificatives |"
    )?;
    writeln!(f, "| Renouvellement | Notification de renouvellement |")?;
    writeln!(f, "| Attestation | Lettre d'attestation |")?;
    writeln!(f, "| PvCommission | Procès-verbal de commission |")?;
    writeln!(
        f,
        "| CourrierInformation | Courrier d'information générale |"
    )?;
    writeln!(f, "| Regularisation | Notification de régularisation |")?;
    writeln!(f, "| Bordereau | Bordereau de transmission |")?;
    writeln!(f)?;

    writeln!(f, "## Processus de génération en masse")?;
    writeln!(f, "<!-- GNX:INTRO:processus-masse -->")?;
    writeln!(f)?;
    writeln!(f, "```mermaid")?;
    writeln!(f, "sequenceDiagram")?;
    writeln!(f, "    participant U as Utilisateur")?;
    writeln!(f, "    participant C as CourrierController")?;
    writeln!(f, "    participant R as RegleCourrierMasse")?;
    writeln!(f, "    participant G as CourrierGenerer")?;
    writeln!(f, "    participant PDF as Aspose.Words")?;
    writeln!(f, "    U->>C: Sélection type + modèle")?;
    writeln!(f, "    C->>R: GetTypeDestinataire()")?;
    writeln!(f, "    R-->>C: Fournisseur/Dossier/Bénéficiaire")?;
    writeln!(f, "    C-->>U: Grille des destinataires éligibles")?;
    writeln!(f, "    U->>C: Sélection + Imprimer")?;
    writeln!(f, "    C->>R: PrepareCreationCourrierMasse()")?;
    writeln!(f, "    loop Pour chaque destinataire")?;
    writeln!(f, "        R->>G: GenererInfoCourrier()")?;
    writeln!(f, "        G->>PDF: Mail merge variables ELODIE")?;
    writeln!(f, "        PDF-->>G: PDF généré")?;
    writeln!(f, "        G->>G: Sauver en base")?;
    writeln!(f, "    end")?;
    writeln!(f, "    G->>G: Fusionner tous les PDFs")?;
    writeln!(f, "    G-->>U: Téléchargement PDF unique")?;
    writeln!(f, "```")?;
    writeln!(f)?;

    writeln!(f, "## Variables de fusion ELODIE")?;
    writeln!(f, "<!-- GNX:INTRO:variables-fusion -->")?;
    writeln!(f, "L'application utilise Aspose.Words pour injecter des données dans des templates Word (.doc/.docx).")?;
    writeln!(f)?;
    writeln!(f, "| Catégorie | Variables |")?;
    writeln!(f, "|-----------|-----------|")?;
    writeln!(f, "| Identité | NumDossier, NomBeneficiaire, Prenom, NIA |")?;
    writeln!(f, "| Adresses | Adresse1, Adresse2, CodePostal, Ville |")?;
    writeln!(f, "| Financier | Taux, SommeTotalePartBen, MontantAide |")?;
    writeln!(f, "| Paiement | IBAN, BIC, LibelleBanque |")?;
    writeln!(f, "| Commission | DateCommission, LibelleCommission |")?;
    writeln!(f)?;

    writeln!(f, "<!-- GNX:CLOSING -->")?;
    println!("  {} processes/courriers.md", "OK".green());
    Ok(true)
}

fn generate_paiements_doc(graph: &KnowledgeGraph, dir: &Path) -> Result<bool> {
    let has_reglement = graph
        .iter_nodes()
        .any(|n| n.properties.name.contains("Reglement") || n.properties.name.contains("Facture"));
    if !has_reglement {
        return Ok(false);
    }

    let out_path = dir.join("paiements-lifecycle.md");
    let mut f = std::fs::File::create(&out_path)?;

    writeln!(f, "# Cycle de Paiement — De la Facture à ELODIE")?;
    writeln!(f, "<!-- GNX:LEAD -->")?;
    writeln!(f)?;
    writeln!(f, "> Ce document décrit le flux financier complet, de la saisie d'une facture à l'export vers ELODIE.")?;
    writeln!(f)?;

    writeln!(f, "## Statuts de paiement")?;
    writeln!(f, "<!-- GNX:INTRO:statuts-paiement -->")?;
    writeln!(f)?;
    writeln!(f, "```mermaid")?;
    writeln!(f, "stateDiagram-v2")?;
    writeln!(f, "    [*] --> DemPaiemVal : Création facture")?;
    writeln!(f, "    DemPaiemVal --> DemPaiemCtrler : Contrôle")?;
    writeln!(f, "    DemPaiemCtrler --> DemPaiemCorrig : Correction")?;
    writeln!(
        f,
        "    DemPaiemVal --> DemGrPrVal : Groupement (SetNumeroValidation)"
    )?;
    writeln!(f, "    DemPaiemCtrler --> DemGrPrVal : Groupement")?;
    writeln!(f, "    DemPaiemCorrig --> DemGrPrVal : Groupement")?;
    writeln!(f, "    DemGrPrVal --> DemTransmiseELODIE : Fonds nationaux")?;
    writeln!(f, "    DemGrPrVal --> BordereauEditeFP : Fonds propres")?;
    writeln!(
        f,
        "    DemTransmiseELODIE --> PaiementRegle : Règlement final"
    )?;
    writeln!(
        f,
        "    BordereauEditeFP --> PaiementRegle : Règlement final"
    )?;
    writeln!(f, "```")?;
    writeln!(f)?;

    writeln!(f, "## Pipeline complet")?;
    writeln!(f, "<!-- GNX:INTRO:pipeline-paiement -->")?;
    writeln!(f)?;
    writeln!(f, "```mermaid")?;
    writeln!(f, "sequenceDiagram")?;
    writeln!(f, "    participant Agent as Agent CMCAS")?;
    writeln!(f, "    participant FC as FacturesController")?;
    writeln!(f, "    participant FS as FactureService")?;
    writeln!(f, "    participant DB as Entity Framework")?;
    writeln!(f, "    participant EL as ElodieService")?;
    writeln!(f, "    Agent->>FC: Créer Facture")?;
    writeln!(f, "    FC->>FS: FactureSave()")?;
    writeln!(f, "    FS->>DB: Insert REGLEMENT (statut=DemPaiemVal)")?;
    writeln!(f, "    Agent->>FC: Validation groupée")?;
    writeln!(f, "    FC->>FS: SetNumeroValidation(liste)")?;
    writeln!(f, "    FS->>DB: Update statut → DemGrPrVal")?;
    writeln!(f, "    Agent->>FC: Transmission ELODIE")?;
    writeln!(f, "    FC->>FS: SetValidationElodie(liste)")?;
    writeln!(f, "    FS->>DB: Update statut → DemTransmiseELODIE")?;
    writeln!(f, "    Agent->>FC: Export ELODIE")?;
    writeln!(f, "    FC->>EL: ExportElodiePost()")?;
    writeln!(f, "    EL-->>Agent: Fichier Excel Flux3 ELODIE")?;
    writeln!(f, "```")?;
    writeln!(f)?;

    writeln!(f, "<!-- GNX:CLOSING -->")?;
    println!("  {} processes/paiements-lifecycle.md", "OK".green());
    Ok(true)
}

fn generate_baremes_doc(graph: &KnowledgeGraph, dir: &Path) -> Result<bool> {
    let has_bareme = graph
        .iter_nodes()
        .any(|n| n.properties.name.contains("Bareme"));
    if !has_bareme {
        return Ok(false);
    }

    let out_path = dir.join("baremes-calcul.md");
    let mut f = std::fs::File::create(&out_path)?;

    writeln!(f, "# Moteur de Calcul des Barèmes")?;
    writeln!(f, "<!-- GNX:LEAD -->")?;
    writeln!(f)?;
    writeln!(
        f,
        "> Le barème détermine le taux de participation (TauxFASS) en fonction des ressources."
    )?;
    writeln!(f)?;

    writeln!(f, "## Processus de calcul")?;
    writeln!(f, "<!-- GNX:INTRO:calcul-bareme -->")?;
    writeln!(f)?;
    writeln!(f, "```mermaid")?;
    writeln!(f, "flowchart TD")?;
    writeln!(
        f,
        "    A[\"Ressources annuelles\"] --> B[\"÷ Nombre de parts\"]"
    )?;
    writeln!(f, "    B --> C[\"Ressource comparable\"]")?;
    writeln!(f, "    C --> D{{Match Tranche ?}}")?;
    writeln!(f, "    D -->|Oui| J[\"TauxFASS = TRA_TAUX_SERVI\"]")?;
    writeln!(f, "    D -->|Non| K[\"Hors barème / Taux min\"]")?;
    writeln!(f, "    J --> M[\"Plafond de la tranche\"]")?;
    writeln!(f, "    M --> N{{Majorations ?}}")?;
    writeln!(f, "    N -->|Oui| O[\"Appliquer Majo\"]")?;
    writeln!(f, "    O --> T[\"Plafond final\"]")?;
    writeln!(f, "    N -->|Non| T")?;
    writeln!(f, "```")?;
    writeln!(f)?;

    writeln!(f, "## Automatique vs Manuel")?;
    writeln!(f, "<!-- GNX:INTRO:auto-vs-manuel -->")?;
    writeln!(f)?;
    writeln!(f, "| Aspect | Barème Automatique | Barème Manuel |")?;
    writeln!(f, "|--------|-------------------|---------------|")?;
    writeln!(
        f,
        "| Création | Tranches calculées (min/max) | Tranches saisies une par une |"
    )?;
    writeln!(f, "| BAR_TYPE | 1 | 2 ou 3 |")?;
    writeln!(f, "| Flexibilité | Fixe | Totale |")?;
    writeln!(f)?;

    writeln!(f, "<!-- GNX:CLOSING -->")?;
    println!("  {} processes/baremes-calcul.md", "OK".green());
    Ok(true)
}

fn generate_financial_entities_doc(graph: &KnowledgeGraph, dir: &Path) -> Result<bool> {
    let has_reglement = graph
        .iter_nodes()
        .any(|n| n.properties.name.contains("Reglement"));
    if !has_reglement {
        return Ok(false);
    }

    let out_path = dir.join("entites-financieres.md");
    let mut f = std::fs::File::create(&out_path)?;

    writeln!(f, "# Entités Financières et leur Cycle de Vie")?;
    writeln!(f, "<!-- GNX:LEAD -->")?;
    writeln!(f)?;
    writeln!(f, "## Structure des données")?;
    writeln!(f, "<!-- GNX:INTRO:structure-donnees-financieres -->")?;
    writeln!(f)?;
    writeln!(f, "```mermaid")?;
    writeln!(f, "graph TD")?;
    writeln!(f, "    DOSSIERPRESTA -->|1:N| REGLEMENT")?;
    writeln!(f, "    REGLEMENT -->|1:N| REGLEMENTLIGNE")?;
    writeln!(f, "    REGLEMENT -->|1:1| STATREG")?;
    writeln!(f, "    REGLEMENTLIGNE -->|1:N| REGULLIGNE")?;
    writeln!(f, "    REGLEMENT -->|N:1| BORDEREAU")?;
    writeln!(f, "    BORDEREAU -->|1:1| EXPORT")?;
    writeln!(f, "```")?;
    writeln!(f)?;

    writeln!(f, "<!-- GNX:CLOSING -->")?;
    println!("  {} processes/entites-financieres.md", "OK".green());
    Ok(true)
}

fn generate_suppliers_doc(graph: &KnowledgeGraph, dir: &Path) -> Result<bool> {
    let has_fournisseur = graph
        .iter_nodes()
        .any(|n| n.properties.name.contains("Fournisseur"));
    if !has_fournisseur {
        return Ok(false);
    }

    let out_path = dir.join("fournisseurs.md");
    let mut f = std::fs::File::create(&out_path)?;

    writeln!(f, "# Gestion des Fournisseurs")?;
    writeln!(f, "<!-- GNX:LEAD -->")?;
    writeln!(f)?;
    writeln!(
        f,
        "> Les fournisseurs sont les prestataires payés par la CMCAS pour les aides sociales."
    )?;
    writeln!(f)?;

    writeln!(f, "## Fonctionnalités clés")?;
    writeln!(f, "<!-- GNX:INTRO:fonctionnalites-fournisseurs -->")?;
    writeln!(f, "- Recherche multi-critères (Nom, CP, Ville)")?;
    writeln!(f, "- Gestion des coordonnées bancaires (IBAN/BIC)")?;
    writeln!(f, "- Historique des paiements par fournisseur")?;
    writeln!(f, "- Association aux types d'aides éligibles")?;
    writeln!(f)?;

    writeln!(f, "<!-- GNX:CLOSING -->")?;
    println!("  {} processes/fournisseurs.md", "OK".green());
    Ok(true)
}
