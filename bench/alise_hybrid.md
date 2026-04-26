# Alise_v2 hybrid search bench (French corpus)

Repo: D:/taf/Alise_v2
Model: paraphrase-multilingual-MiniLM-L12-v2 (384d, multilingue)
Indexed at:   "indexedAt": "2026-04-17T05:58:45Z",

Generated: 2026-04-24T15:56:05Z

## Q1 — "comment un bénéficiaire est ajouté à un dossier"

### BM25 seul
```
Found 5 results for 'comment un bénéficiaire est ajouté à un dossier':

    1. [Class     ] PLAFOND_DOSSIER                 CCAS.Alise.DAL/PLAFOND_DOSSIER.cs:15-29
    2. [Class     ] DossierListPaieMasse            CCAS.Alise.Entities/Dossier/DossierListPaieMasse.cs:9-24
    3. [Method    ] TransfertDossierIndividuel      CCAS.Alise.BAL/Dossier/DossierService.cs:2126-2174
    4. [Method    ] ResultDetailsDossierNv          CCAS.Alise.BAL/Dossier/DossierService.cs:352-404
    5. [Method    ] SupprCour                       CCAS.Alise.BAL/Dossier/DossierService.cs:596-615
```

### +Hybrid (BM25 + multilingual RRF)
```
Found 5 results for 'comment un bénéficiaire est ajouté à un dossier':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Class     ] PLAFOND_DOSSIER                 CCAS.Alise.DAL/PLAFOND_DOSSIER.cs:15-29
    2. [Class     ] ResultatCreationJustif          CCAS.Alise.Entities/Aide/ResultatCreationJustif.cs:10-14
    3. [Function  ] add_severity                    corrections_courrier_masse/gen_doc.py:47-53
    4. [Method    ] GetNumeroEMail                  CCAS.Alise.BAL/Dossier/INumCommiService.cs:21-21
    5. [Class     ] IligibiliteBeneficiaire         EcheancePrestationAliseV2/Traitement/IligibiliteBeneficiaire.cs:13-63
```

## Q2 — "gestion des factures fournisseurs"

### BM25 seul
```
Found 5 results for 'gestion des factures fournisseurs':

    1. [Class     ] Resultat_Fournisseurs           CCAS.Alise.Entities/Fournisseur/Resultat_Fournisseurs.cs:8-13
    2. [Function  ] clearSelectedDemPaiement        CCAS.Alise.ihm/Views/Fournisseurs/VuePartielleDetailFournisseur/ListeDemandePaiement.cshtml:239-243
    3. [Function  ] onErrorGrid                     CCAS.Alise.ihm/Views/Fournisseurs/VuePartielleDetailFournisseur/ListeDossiersRattachés.cshtml:206-214
    4. [Function  ] Export                          CCAS.Alise.ihm/Views/Fournisseurs/VuePartielleDetailFournisseur/ListeDossiersRattachés.cshtml:220-234
    5. [Function  ] OnDataBinding                   CCAS.Alise.ihm/Views/Fournisseurs/VuePartielleDetailFournisseur/ListeDossiersRattachés.cshtml:200-204
```

### +Hybrid (BM25 + multilingual RRF)
```
Found 5 results for 'gestion des factures fournisseurs':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Class     ] Resultat_Fournisseurs           CCAS.Alise.Entities/Fournisseur/Resultat_Fournisseurs.cs:8-13
    2. [ControllerAction] CreerPaiementBen                CCAS.Alise.ihm/Controllers/FacturesController.cs:467
    3. [Function  ] clearSelectedDemPaiement        CCAS.Alise.ihm/Views/Fournisseurs/VuePartielleDetailFournisseur/ListeDemandePaiement.cshtml:239-243
    4. [Method    ] InsererBudgetAnnuel             CCAS.Alise.BAL/Administration/Budget/Regles/RegleBudget.cs:55-79
    5. [Function  ] refreshGridDemandePaiement      CCAS.Alise.ihm/Views/Fournisseurs/VuePartielleDetailFournisseur/ListeDemandePaiement.cshtml:225-237
```

## Q3 — "calcul du barème pour une aide"

### BM25 seul
```
Found 5 results for 'calcul du barème pour une aide':

    1. [Class     ] TARIFBASE_AIDE                  CCAS.Alise.DAL/TARIFBASE_AIDE.cs:15-41
    2. [Class     ] REFSTATUTS                      CCAS.Alise.Entities/Aide/REFSTATUTS.cs:9-14
    3. [Class     ] StatDossier                     CCAS.Alise.Entities/Aide/StatDossier.cs:9-19
    4. [Class     ] RefTypeGroupe                   CCAS.Alise.Entities/Aide/RefTypeGroupe.cs:9-15
    5. [Class     ] Plafond                         CCAS.Alise.Entities/Aide/Plafond.cs:11-56
```

### +Hybrid (BM25 + multilingual RRF)
```
Found 5 results for 'calcul du barème pour une aide':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Class     ] TARIFBASE_AIDE                  CCAS.Alise.DAL/TARIFBASE_AIDE.cs:15-41
    2. [Class     ] AideFinance                     CCAS.Alise.Entities/Aide/AideFinance.cs:12-96
    3. [Class     ] RefJustificatif                 CCAS.Alise.Entities/Aide/RefJustificatif.cs:9-15
    4. [Method    ] CumulerMontantComptabilise      CCAS.Alise.BAL/Statistique/UtilStat/BudgetSuivi/BudgetSuivi.cs:94-103
    5. [Class     ] RefUnite                        CCAS.Alise.Entities/Aide/RefUnite.cs:9-19
```

## Q4 — "génération de courrier en masse"

### BM25 seul
```
Found 5 results for 'génération de courrier en masse':

    1. [Method    ] MappingInfoBenef                corrections_courrier_masse/MappingCourriers.cs:227-297
    2. [Method    ] GenererPdf                      corrections_courrier_masse/RegleCourriers.cs:278-314
    3. [Method    ] ListeAideGroupeCourriers        corrections_courrier_masse/RegleCourriers.cs:109-128
    4. [Method    ] ListeDocumentsAProduire         corrections_courrier_masse/RegleCourriers.cs:521-642
    5. [Method    ] CloneJustificatif               corrections_courrier_masse/MappingCourriers.cs:476-500
```

### +Hybrid (BM25 + multilingual RRF)
```
Found 5 results for 'génération de courrier en masse':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Class     ] MailBodyUtil                    CCAS.Alise.BAL/Commun/MailBodyUtil.cs:15-1598
    2. [Method    ] VerifierDonneeInput             corrections_courrier_masse/RegleCourriers.cs:27-82
    3. [Class     ] MailUtil                        CCAS.Alise.BAL/Commun/MailUtil.cs:19-331
    4. [Method    ] MappingAdresseDestinataire      corrections_courrier_masse/MappingCourriers.cs:186-219
    5. [Class     ] MailBodyUtilTest                CCAS.Alise.BAL.Tests/Mails/MailBodyUtilTest.cs:12-265
```

## Q5 — "validation par la commission sociale"

### BM25 seul
```
Found 5 results for 'validation par la commission sociale':

    1. [Method    ] Commission                      CCAS.Alise.ihm/Controllers/CommissionController.cs:31-57
    2. [Class     ] CommissionModel                 CCAS.Alise.ihm/Models/Commission/CommissionModel.cs:15-271
    3. [Method    ] ConstruireTablePourExportDossiers  CCAS.Alise.ihm/Models/Commission/CommissionModel.cs:215-235
    4. [Method    ] ActualiserValeurTauxNumeroCommission  CCAS.Alise.ihm/Models/Commission/CommissionModel.cs:110-150
    5. [Method    ] SelectionnerRegistre            CCAS.Alise.ihm/Models/Commission/CommissionModel.cs:185-204
```

### +Hybrid (BM25 + multilingual RRF)
```
Found 5 results for 'validation par la commission sociale':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Method    ] ValiderParCommission            CCAS.Alise.ihm/Models/Commission/CommissionModel.cs:261-268
    2. [Function  ] ValidationSummary               CCAS.Alise.ihm/Views/Administration/EditerAide.cshtml:4-4
    3. [Method    ] Commission                      CCAS.Alise.ihm/Controllers/CommissionController.cs:31-57
    4. [Class     ] CommissionModel                 CCAS.Alise.ihm/Models/Commission/CommissionModel.cs:15-271
    5. [Function  ] ValidationSummary               CCAS.Alise.ihm/Views/MCO/StatutPaiementRegle.cshtml:4-116
```

## Q6 — "DbContext Entity Framework configuration"

### BM25 seul
```
Found 1 results for 'DbContext Entity Framework configuration':

    1. [Method    ] Configuration                   CCAS.Alise.ihm/Startup.cs:12-30
```

### +Hybrid (BM25 + multilingual RRF)
```
Found 5 results for 'DbContext Entity Framework configuration':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Method    ] Configuration                   CCAS.Alise.ihm/Startup.cs:12-30
    2. [Method    ] CreateViewContext               CCAS.PdfReportGenerator/HtmlViewRenderer.cs:46-49
    3. [Class     ] CMCASClient                     ErableWebAPI/WebAPI.cs:53-563
    4. [Method    ] UseSafeContext                  CCAS.Alise.ihm/Utilitaires/Cache/Cache.cs:80-94
    5. [Method    ] CMCASAsync                      ErableWebAPI/WebAPI.cs:91-193
```

## Q7 — "authentification utilisateur"

### BM25 seul
```
Found 5 results for 'authentification utilisateur':

    1. [Class     ] Resultat_Utilisateur            CCAS.Alise.Entities/Utilisateur/Resultat_Utilisateur.cs:7-12
    2. [Class     ] UTILISATEUR                     CCAS.Alise.DAL/UTILISATEUR.cs:15-35
    3. [Class     ] UTILISATEUR_SPE                 CCAS.Alise.DAL/UTILISATEUR_SPE.cs:15-18
    4. [Class     ] FonctionAutoriseUrl             CCAS.Alise.Entities/Utilisateur/FonctionAutoriseUrl.cs:12-64
    5. [Class     ] ControleurAction                CCAS.Alise.Entities/Utilisateur/ControleurAction.cs:9-19
```

### +Hybrid (BM25 + multilingual RRF)
```
Found 5 results for 'authentification utilisateur':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Class     ] Resultat_Utilisateur            CCAS.Alise.Entities/Utilisateur/Resultat_Utilisateur.cs:7-12
    2. [Method    ] AutorisationPaiementBenef       CCAS.Alise.ihm/Models/Utilisateur/AutorisationPaiement.cs:138-151
    3. [Class     ] UTILISATEUR                     CCAS.Alise.DAL/UTILISATEUR.cs:15-35
    4. [Class     ] UserCache                       CCAS.Alise.ihm/Utilitaires/Cache/Cache.cs:73-127
    5. [Class     ] UTILISATEUR_SPE                 CCAS.Alise.DAL/UTILISATEUR_SPE.cs:15-18
```

## Q8 — "where is the DossierController"

### BM25 seul
```
Found 3 results for 'where is the DossierController':

    1. [Function  ] is_numeric                      CCAS.Alise.ihm/Views/Dossiers/RechercheDossiersTest.cshtml#script-0:110-113
    2. [Function  ] is                              CCAS.Alise.ihm/Scripts/modernizr-2.5.3.js:273-275
    3. [Function  ] is                              packages/Modernizr.2.5.3/Content/Scripts/modernizr-2.5.3.js:273-275
```

### +Hybrid (BM25 + multilingual RRF)
```
Found 5 results for 'where is the DossierController':
  (hybrid BM25+semantic RRF, pool=20)

    1. [ControllerAction] ModifDossier                    CCAS.Alise.ihm/Controllers/DossiersController.cs:398
    2. [Function  ] is_numeric                      CCAS.Alise.ihm/Views/Dossiers/RechercheDossiersTest.cshtml#script-0:110-113
    3. [ControllerAction] RechDossier                     CCAS.Alise.ihm/Controllers/DossiersController.cs:40
    4. [Function  ] is                              CCAS.Alise.ihm/Scripts/modernizr-2.5.3.js:273-275
    5. [ControllerAction] CreerDossier                    CCAS.Alise.ihm/Controllers/DossiersController.cs:307
```

## Q9 — "import de données administratives"

### BM25 seul
```
Found 5 results for 'import de données administratives':

    1. [Method    ] Test_Import_Elodie_ExecuteImportProcedure  CCAS.Alise.BAL.Tests/Statistique/StatistiqueTest.cs:197-217
    2. [Method    ] Test_Import_Elodie_ImportCsvInSql  CCAS.Alise.BAL.Tests/Statistique/StatistiqueTest.cs:122-137
    3. [Method    ] Test_Import_Elodie_GenerateMultiTabExcelFile  CCAS.Alise.BAL.Tests/Statistique/StatistiqueTest.cs:223-243
    4. [Method    ] Test_Import_Elodie_NombreLignesCsvVsSql  CCAS.Alise.BAL.Tests/Statistique/StatistiqueTest.cs:181-191
    5. [Class     ] CategDossLib                    CCAS.Alise.Entities/Statistique/Tableau de bord/CategDossLib.cs:3-9
```

### +Hybrid (BM25 + multilingual RRF)
```
Found 5 results for 'import de données administratives':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Method    ] Test_Import_Elodie_GenerateMultiTabExcelFile  CCAS.Alise.BAL.Tests/Statistique/StatistiqueTest.cs:223-243
    2. [Method    ] ExportPaiement                  CCAS.Alise.BAL/Statistique/StatistiqueService.cs:191-386
    3. [Class     ] Export                          CCAS.Alise.Entities/Statistique/Commun/Export.cs:10-26
    4. [Method    ] Test_Import_Elodie_ImportCsvInSql  CCAS.Alise.BAL.Tests/Statistique/StatistiqueTest.cs:122-137
    5. [Method    ] Test_Import_Elodie_NombreLignesCsvVsSql  CCAS.Alise.BAL.Tests/Statistique/StatistiqueTest.cs:181-191
```

## Q10 — "règles de traitement des dossiers"

### BM25 seul
```
Found 5 results for 'règles de traitement des dossiers':

    1. [Class     ] Traitement                      TestWsFournisseur/Traitement.cs:12-54
    2. [Method    ] ListTousfournisseur             TestWsFournisseur/Traitement.cs:14-41
    3. [Method    ] Unfournisseur                   TestWsFournisseur/Traitement.cs:43-53
    4. [Method    ] IligibiliteBenef                EcheancePrestationAliseV2/Traitement/IligibiliteBeneficiaire.cs:16-61
    5. [Class     ] IligibiliteBeneficiaire         EcheancePrestationAliseV2/Traitement/IligibiliteBeneficiaire.cs:13-63
```

### +Hybrid (BM25 + multilingual RRF)
```
Found 5 results for 'règles de traitement des dossiers':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Class     ] Traitement                      TestWsFournisseur/Traitement.cs:12-54
    2. [Method    ] AjoutReglementComptabilise      CCAS.Alise.BAL/Facture/GestionPlafonds/ResumerPlafondMensuelle.cs:22-37
    3. [Class     ] REGLEMENT                       CCAS.Alise.DAL/REGLEMENT.cs:15-68
    4. [Method    ] ListTousfournisseur             TestWsFournisseur/Traitement.cs:14-41
    5. [Method    ] ListDestinaireReglement         CCAS.Alise.BAL/Beneficiaire/BenefServiceDummy.cs:76-79
```

## Q11 — "export Excel des statistiques"

### BM25 seul
```
Found 5 results for 'export Excel des statistiques':

    1. [Class     ] EXPORT                          CCAS.Alise.DAL/AliseV2DocModel/EXPORT.cs:15-26
    2. [Class     ] Export                          CCAS.Alise.Entities/Statistique/Commun/Export.cs:10-26
    3. [Class     ] ElodieSearch                    CCAS.Alise.Entities/Statistique/Export/ELODIE/ElodieSearch.cs:7-35
    4. [Class     ] PaiementInfoBenef               CCAS.Alise.Entities/Statistique/Export/Paiement/PaiementInfoBenef.cs:9-34
    5. [Function  ] Export                          CCAS.Alise.ihm/Views/Fournisseurs/VuePartielleDetailFournisseur/ListeDossiersRattachés.cshtml:220-234
```

### +Hybrid (BM25 + multilingual RRF)
```
Found 5 results for 'export Excel des statistiques':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Class     ] EXPORT                          CCAS.Alise.DAL/AliseV2DocModel/EXPORT.cs:15-26
    2. [Method    ] ExportQuantitative              CCAS.Alise.BAL/Statistique/UtilStat/ExportExcel/ExcelTemplateExportQuantitative.cs:12-83
    3. [Class     ] ExcelTemplExpSuiviBudget        CCAS.Alise.BAL/Statistique/UtilStat/ExportExcel/ExcelTemplExpSuiviBudget.cs:10-144
    4. [Class     ] Export                          CCAS.Alise.Entities/Statistique/Commun/Export.cs:10-26
    5. [Class     ] ExcelTemplateExportQuantitative  CCAS.Alise.BAL/Statistique/UtilStat/ExportExcel/ExcelTemplateExportQuantitative.cs:9-84
```

## Q12 — "background jobs planification"

### BM25 seul
```
No results for 'background jobs planification'.
```

### +Hybrid (BM25 + multilingual RRF)
```
Found 5 results for 'background jobs planification':
  (hybrid BM25+semantic RRF, pool=20)

    1. [Service   ] BackgroundJobService            CCAS.Alise.BAL/BackgroundJob/BackgroundJobService.cs
    2. [Class     ] BackgroundJobService            CCAS.Alise.BAL/BackgroundJob/BackgroundJobService.cs:16-786
    3. [Method    ] WorkflowEtatPaiement            CCAS.Alise.BAL/Facture/Regles/ReglePaiementBenef.cs:26-50
    4. [Method    ] BuildSafeFileName               CCAS.Alise.BAL/BackgroundJob/BackgroundJobService.cs:207-217
    5. [Method    ] Initializer                     CCAS.Alise.BAL.Tests/BackgroundJob/BackgroundJobServiceTests.cs:231-234
```

