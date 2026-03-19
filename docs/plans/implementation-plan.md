# Plan d'Implémentation — Klima

## Table des matières

1. [Contexte et état des lieux](#1-contexte-et-état-des-lieux)
2. [Principes directeurs](#2-principes-directeurs)
3. [Phases d'implémentation](#3-phases-dimplémentation)
   - [Phase 0 — Fondations & Outillage DX](#phase-0--fondations--outillage-dx)
   - [Phase 1 — Acquisition & Préparation des données géospatiales](#phase-1--acquisition--préparation-des-données-géospatiales)
   - [Phase 2 — Génération du dataset CFD](#phase-2--génération-du-dataset-cfd)
   - [Phase 3 — Entraînement du Surrogate Model](#phase-3--entraînement-du-surrogate-model)
   - [Phase 4 — Backend API complet](#phase-4--backend-api-complet)
   - [Phase 5 — Frontend interactif complet](#phase-5--frontend-interactif-complet)
   - [Phase 6 — Intégration ONNX end-to-end](#phase-6--intégration-onnx-end-to-end)
   - [Phase 7 — Validation scientifique & Production](#phase-7--validation-scientifique--production)
4. [Dimensionnement scientifique](#4-dimensionnement-scientifique)
5. [Registre des risques](#5-registre-des-risques)
6. [Matrice des tâches & parallélisation](#6-matrice-des-tâches--parallélisation)

---

## 1. Contexte et état des lieux

### Ce qui existe (scaffolding livré)

| Composant | État | Fichiers |
|-----------|------|----------|
| Backend Rust/Axum | Squelette compilable, routes placeholder | `back/src/{main,routes/*,db/*}.rs` |
| Frontend Vue/Quasar/CesiumJS | Squelette, viewer 3D avec OSM Buildings | `front/src/{components,layouts,pages}/*` |
| Docker dev | Dockerfile + compose pour back et front | `{back,front}/docker/*` |
| Script orchestrateur | `run-dev.sh` (up/down/logs/restart) | `scripts/run-dev.sh` |
| Documentation | Spec, architecture, setup | `docs/*.md` |
| Base de données | Schema SQLite (projects, simulations, scenarios) | `back/src/db/mod.rs` |

### Ce qui manque pour un produit fonctionnel

1. **Pipeline de données Python** — acquisition géospatiale, voxelisation, CFD
2. **Modèle IA Surrogate** — entraînement U-Net 3D / GNN, export ONNX
3. **Backend fonctionnel** — CRUD, inférence ONNX réelle, compression résultats
4. **Frontend fonctionnel** — édition de scène, appel API, visualisation thermique + vent
5. **Validation scientifique** — comparaison prédictions vs vérité terrain
6. **Infrastructure de production** — CI/CD, monitoring, déploiement

---

## 2. Principes directeurs

1. **Vertical slice first** : livrer une tranche fonctionnelle end-to-end sur un périmètre réduit (un quartier, un scénario) avant d'élargir.
2. **Contract-driven** : les interfaces API (OpenAPI) et les formats de tenseurs sont définis *avant* l'implémentation.
3. **Reproductibilité scientifique** : tout script de données et d'entraînement est versionné, déterministe (seeds fixés), et documenté.
4. **Containerisation systématique** : tout tourne dans Docker, y compris le pipeline Python et OpenFOAM.
5. **Parallélisation maximale** : les 3 piliers (data/ML, backend, frontend) avancent en parallèle via des contrats d'interface stables.

---

## 3. Phases d'implémentation

---

### Phase 0 — Fondations & Outillage DX

**Objectif** : Rendre le monorepo robuste, testable et automatisé avant tout développement fonctionnel.

| ID | Tâche | Livrable | Dépendances |
|----|-------|----------|-------------|
| `P0.1` | CI/CD GitHub Actions : lint (clippy + eslint), test, build | `.github/workflows/ci.yml` | — |
| `P0.2` | Infrastructure de test backend (cargo test, tests d'intégration avec SQLite en mémoire) | `back/tests/` | — |
| `P0.3` | Infrastructure de test frontend (Vitest, tests composants Vue) | `front/vitest.config.ts`, `front/src/**/*.test.ts` | — |
| `P0.4` | Gestion d'erreurs structurée backend (type `AppError`, codes HTTP cohérents) | `back/src/error.rs` | — |
| `P0.5` | Spécification OpenAPI du contrat d'API (JSON Schema des requêtes/réponses) | `docs/api/openapi.yml` | — |
| `P0.6` | Pipeline Python : Dockerfile + docker-compose (Python 3.12, PyTorch, GeoPandas) | `training/docker/` | — |
| `P0.7` | Structure du dossier `training/` dans le monorepo | `training/{src,data,models,configs}/` | — |

**Critère de sortie** : `cargo test`, `npm run test`, et le workflow CI passent au vert.

---

### Phase 1 — Acquisition & Préparation des données géospatiales

**Objectif** : Constituer la géométrie 3D et les métadonnées de surface d'un quartier test exploitable par le solveur CFD.

#### 1a. Définition du domaine d'étude

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P1.1` | Sélection du quartier test | Zone dense, ~500 m × 500 m. Candidats : Paris 11e (rue Oberkampf), Lyon Part-Dieu, Toulouse Capitole. Critères : disponibilité BD TOPO + LiDAR HD + Urban Atlas. | `training/configs/domain.yaml` (bbox WGS84, CRS, résolution) |
| `P1.2` | Téléchargement BD TOPO IGN | Emprise au sol + hauteur des bâtiments. Format shapefile/GeoJSON. Script reproductible. | `training/src/data/download_bdtopo.py` |
| `P1.3` | Téléchargement LiDAR HD IGN | Nuages de points (fichiers LAZ). Extraction du MNS (Modèle Numérique de Surface) pour la canopée. | `training/src/data/download_lidar.py` |
| `P1.4` | Téléchargement Urban Atlas (Copernicus) | Classification d'occupation des sols. Attribution des coefficients d'albédo (α) et d'émissivité (ε) par type de surface. | `training/src/data/download_urban_atlas.py` |
| `P1.5` | Téléchargement ERA5 / Météo-France | Données météo horaires : T_air, vitesse/direction vent, rayonnement solaire, humidité. | `training/src/data/download_meteo.py` |

#### 1b. Traitement géométrique

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P1.6` | Fusion des sources géométriques | Combiner BD TOPO (bâtiments) + LiDAR (canopée) + Urban Atlas (sols). Projection vers un CRS métrique local (ex: Lambert-93 / EPSG:2154). | `training/src/preprocessing/geometry_fusion.py` |
| `P1.7` | Voxelisation 3D | Transformer la géométrie continue en grille régulière 3D. Résolution cible : Δx = Δy = 2 m, Δz = 2 m. Domaine 500×500×100 m → grille 250×250×50. Chaque voxel contient un label : {air, bâtiment, sol, végétation}. | `training/src/preprocessing/voxelizer.py` |
| `P1.8` | Attribution des propriétés physiques | Chaque voxel de surface reçoit : albédo (α), émissivité (ε), conductivité thermique (k), rugosité aérodynamique (z₀). Lookup table depuis Urban Atlas. | `training/src/preprocessing/surface_properties.py` |
| `P1.9` | Validation et visualisation | Visualisation 3D de la grille voxelisée (Matplotlib 3D ou PyVista). Vérification des volumes, détection des artefacts. | `training/notebooks/validate_geometry.ipynb` |

**Critère de sortie** : Une grille `.npy` de shape `(250, 250, 50, C)` avec C canaux (occupancy + propriétés) sauvegardée et versionnée.

---

### Phase 2 — Génération du dataset CFD

**Objectif** : Produire les paires [entrée → sortie] qui serviront de vérité terrain pour l'entraînement.

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P2.1` | Conteneurisation OpenFOAM | Dockerfile basé sur `openfoam/openfoam2406-default`. Scripts de setup. | `training/docker/Dockerfile.openfoam` |
| `P2.2` | Convertisseur voxels → maillage OpenFOAM | Transformer la grille voxelisée en maillage blockMesh/snappyHexMesh compatible OpenFOAM. Gestion des boundary conditions (inlet, outlet, ground, top). | `training/src/cfd/voxel_to_mesh.py` |
| `P2.3` | Template de cas OpenFOAM | Cas RANS (k-ε ou k-ω SST) avec équations de transfert thermique couplées. Fichiers `0/`, `constant/`, `system/`. | `training/src/cfd/case_template/` |
| `P2.4` | Générateur paramétrique de scénarios | Échantillonnage Latin Hypercube (LHS) sur l'espace des paramètres : wind_speed ∈ [0.5, 15] m/s, wind_direction ∈ [0°, 360°), sun_elevation ∈ [10°, 80°], albédo_toits ∈ [0.1, 0.8]. Cible : **N = 300 simulations**. | `training/src/cfd/scenario_generator.py` |
| `P2.5` | Runner batch de simulations | Orchestrateur qui lance les N simulations OpenFOAM en parallèle (via job queue ou xargs). Capture des résultats et gestion des échecs. | `training/src/cfd/batch_runner.py` |
| `P2.6` | Parseur de résultats CFD | Extraction des champs T(x,y,z) et v(x,y,z) depuis les fichiers OpenFOAM, ré-échantillonnage sur la grille voxel originale. | `training/src/cfd/result_parser.py` |
| `P2.7` | Constitution du dataset final | Concaténation des N paires. Format : fichier HDF5 ou collection de fichiers `.npz`. Séparation train/val/test (70/15/15). Statistiques de distribution. | `training/src/cfd/dataset_builder.py` |
| `P2.8` | Validation statistique du dataset | Distribution des températures, histogrammes de vitesse de vent, vérification de la conservation de masse (∇·v ≈ 0), absence de valeurs aberrantes. | `training/notebooks/validate_dataset.ipynb` |

**Critère de sortie** : Dataset HDF5 de ~300 paires, documenté, avec splits train/val/test et métriques de qualité.

**Dimensionnement** :
- 300 simulations × ~2h/sim (RANS sur maillage 3M cells) = ~600 h CPU
- Parallélisé sur 16 cœurs : ~38h wall-clock
- Taille dataset : ~300 × (250×250×50×4) × 4 bytes ≈ **18 Go**

---

### Phase 3 — Entraînement du Surrogate Model

**Objectif** : Entraîner un réseau de neurones capable de prédire T(x,y,z) et v(x,y,z) en <100 ms pour une configuration donnée.

#### 3a. Architecture du modèle

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P3.1` | Encodage des entrées | Canaux d'entrée du tenseur : occupancy binaire (1), type de surface one-hot (4-5), propriétés physiques continues (α, ε, z₀ → 3), conditions limites (wind_speed, wind_dir, sun_elev → 3 canaux broadcastés). **Total : ~12 canaux d'entrée.** | `training/src/model/encoding.py` |
| `P3.2` | Architecture U-Net 3D | Encoder : 4 blocs (Conv3D → BatchNorm → ReLU → MaxPool). Bottleneck. Decoder symétrique avec skip connections. Canaux de sortie : 4 (T, vx, vy, vz). Paramétrable (profondeur, canaux). | `training/src/model/unet3d.py` |
| `P3.3` | Fonction de perte PINN | Composante data-fidelity + contrainte physique : $\mathcal{L} = \lambda_1 \cdot MSE(T_{pred}, T_{cfd}) + \lambda_2 \cdot MSE(\mathbf{v}_{pred}, \mathbf{v}_{cfd}) + \lambda_3 \cdot \|\nabla \cdot \mathbf{v}_{pred}\|^2 + \lambda_4 \cdot \|\mathbf{v}_{pred} \cdot \mathbf{1}_{solide}\|^2$. Le dernier terme force la vitesse à zéro dans les obstacles. | `training/src/model/loss.py` |
| `P3.4` | Architecture GNN alternative | Graph construit à partir des voxels actifs (air uniquement). Message-passing avec attention. Plus léger pour les géométries creuses. | `training/src/model/gnn.py` |

#### 3b. Pipeline d'entraînement

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P3.5` | DataLoader PyTorch | Chargement HDF5, normalisation (z-score par canal), augmentation par rotation 90° et flip. Patchification si mémoire insuffisante (patches 64³ avec overlap). | `training/src/model/dataloader.py` |
| `P3.6` | Boucle d'entraînement | Optimizer AdamW, scheduler cosine annealing, gradient clipping. Logging via TensorBoard ou W&B. Early stopping sur val_loss. | `training/src/model/train.py` |
| `P3.7` | Recherche d'hyperparamètres | Grid/random search sur : learning_rate ∈ [1e-4, 1e-3], λ₁..λ₄, profondeur U-Net (3-5), canaux initiaux (32-128). | `training/src/model/hparam_search.py` |
| `P3.8` | Évaluation quantitative | Métriques sur le test set : MAE, RMSE, R² pour T et pour |v|. Ventilation par zone (rue, toit, parc). | `training/src/model/evaluate.py` |

#### 3c. Export et validation du modèle

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P3.9` | Export ONNX | `torch.onnx.export()` avec dynamic axes pour batch_size. Validation via `onnxruntime.InferenceSession` en Python. | `training/src/model/export_onnx.py` |
| `P3.10` | Benchmark de latence | Mesurer le temps d'inférence sur CPU et GPU pour une entrée unique (250×250×50). Cible : < 100 ms sur CPU, < 20 ms sur GPU. | `training/src/model/benchmark.py` |
| `P3.11` | Optimisation du graphe ONNX | Quantization INT8 si nécessaire. ORT graph optimizations (fusion de couches, constant folding). | `training/src/model/optimize_onnx.py` |

**Critère de sortie** : Fichier `.onnx` validé, MAE(T) < 0.5°C, latence < 100 ms CPU.

---

### Phase 4 — Backend API complet

**Objectif** : Transformer le squelette Axum en API robuste, avec CRUD, inférence, et gestion des projets.

#### 4a. API CRUD et gestion des données

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P4.1` | CRUD Projects | `POST/GET/PUT/DELETE /api/projects`. Pagination sur le listing. Tests d'intégration. | `back/src/routes/projects.rs` |
| `P4.2` | CRUD Scenarios | `POST/GET/PUT/DELETE /api/projects/:id/scenarios`. Un scénario contient : géométrie modifiée (diff par rapport à la baseline), paramètres météo. | `back/src/routes/scenarios.rs` |
| `P4.3` | Gestion des résultats de simulation | `GET /api/simulations/:id`. Stockage des résultats en BLOB compressé (zstd) dans SQLite. | `back/src/routes/simulations.rs` |
| `P4.4` | Migrations versionnées | Système de migration incrémentale (numérotées) au lieu d'un `CREATE IF NOT EXISTS` monolithique. | `back/src/db/migrations/` |
| `P4.5` | Validation des entrées | Validation structurelle des requêtes (bornes, types, tailles max). Utiliser `validator` ou validation manuelle. | `back/src/validation.rs` |

#### 4b. Inférence ONNX

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P4.6` | Service ONNX (`OnnxService`) | Chargement du modèle `.onnx` au démarrage via `ort::Session`. Partagé via `Arc<OnnxService>` dans l'AppState. Gestion de l'absence de modèle (mode dégradé). | `back/src/inference/mod.rs` |
| `P4.7` | Préprocesseur géométrie → tenseur | Transformer le JSON de géométrie (blocs, surfaces) en tenseur d'entrée `ndarray` compatible ONNX. Appliquer la même normalisation que l'entraînement. | `back/src/inference/preprocessor.rs` |
| `P4.8` | Postprocesseur tenseur → résultat | Dénormaliser les sorties, extraire T et v par coordonnée, compresser en JSON + zstd. | `back/src/inference/postprocessor.rs` |
| `P4.9` | Endpoint `/api/simulate` réel | Assembler P4.6 + P4.7 + P4.8. Mesurer et logguer le temps d'inférence. Sauvegarder le résultat en base. | Mise à jour de `back/src/routes/simulate.rs` |

#### 4c. Fonctionnalités avancées

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P4.10` | Compression des réponses | Middleware de compression gzip/zstd pour les réponses volumineuses (champs 3D). Header `Accept-Encoding`. | Middleware tower-http |
| `P4.11` | Cache de simulation | LRU cache en mémoire (hash des paramètres + géométrie). Évite de relancer une inférence pour des paramètres identiques. | `back/src/cache.rs` |
| `P4.12` | Endpoint de données géographiques | `GET /api/geodata/buildings?bbox=...` — servir les bâtiments baseline depuis la BD TOPO importée. | `back/src/routes/geodata.rs` |
| `P4.13` | WebSocket pour simulations longues | Channel pour notifier le frontend de la progression. Utile si on ajoute un mode "batch" ou des simulations en file d'attente. | `back/src/routes/ws.rs` |

**Critère de sortie** : API complète, tous les endpoints documentés (OpenAPI), tests d'intégration passent, inférence ONNX fonctionnelle.

---

### Phase 5 — Frontend interactif complet

**Objectif** : Construire l'interface complète permettant de visualiser, éditer et simuler.

#### 5a. Infrastructure frontend

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P5.1` | Pinia stores | `useProjectStore`, `useSimulationStore`, `useScenarioStore`. Gestion de l'état applicatif. | `front/src/stores/` |
| `P5.2` | Service API (composable) | `useApi()` : wrapper typé autour de fetch/axios pour tous les endpoints. Gestion erreurs, retry, loading states. | `front/src/composables/useApi.ts` |
| `P5.3` | Types partagés | Interfaces TypeScript miroir du contrat OpenAPI : `Project`, `Scenario`, `SimulateRequest`, `SimulateResponse`, `GeometryBlock`. | `front/src/types/` |

#### 5b. Gestion de projets

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P5.4` | Page de listing des projets | Grille de cards Quasar avec les projets existants. Bouton "Nouveau projet". | `front/src/pages/ProjectsPage.vue` |
| `P5.5` | Dialog de création/édition de projet | Formulaire Quasar (nom, description, zone géographique). Sélection de la bbox sur une mini-carte Cesium. | `front/src/components/ProjectDialog.vue` |
| `P5.6` | Routing multi-projets | `/projects`, `/projects/:id` (vue simulateur). | Mise à jour `front/src/router/routes.ts` |

#### 5c. Outils d'édition de scène 3D

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P5.7` | Toolbar d'outils d'édition | Barre flottante sur la carte : sélection, pinceau de surface, placement d'objet, gomme. | `front/src/components/EditorToolbar.vue` |
| `P5.8` | Outil de modification de surface | Clic ou pinceau sur le sol : changer le type (bitume → herbe → eau → gravier). Feedback visuel immédiat (colorisation de la zone). | `front/src/composables/useSurfaceEditor.ts` |
| `P5.9` | Outil de placement d'objets | Placer des arbres (modèle 3D simplifié), du mobilier urbain, des panneaux solaires. Chaque objet a des propriétés physiques (ombre, évapotranspiration). | `front/src/composables/useObjectPlacer.ts` |
| `P5.10` | Sérialisation des modifications | Encoder les modifications de scène en format `GeometryDiff` (liste de voxels modifiés + nouveau type). C'est ce qui est envoyé à l'API. | `front/src/utils/geometrySerializer.ts` |

#### 5d. Visualisation des résultats

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P5.11` | Overlay thermique (heatmap 3D) | Colorer les surfaces (sol, toits, façades) selon T(x,y,z) : échelle bleu (frais) → rouge (chaud). Implémenté via `Cesium.Primitive` avec des attributs de couleur par vertex. | `front/src/composables/useThermalOverlay.ts` |
| `P5.12` | Système de particules de vent | Animer des particules (sprites) suivant le champ v(x,y,z). Densité proportionnelle à |v|, couleur proportionnelle à T. Utiliser le `ParticleSystem` de Cesium ou un shader custom. | `front/src/composables/useWindParticles.ts` |
| `P5.13` | Slider temporel (heure du jour) | Contrôle l'élévation solaire et les conditions météo. Interpolation entre deux simulations pré-calculées ou re-calcul en temps réel. | `front/src/components/TimeSlider.vue` |
| `P5.14` | Mode comparaison avant/après | Split-screen ou slider pour comparer l'état actuel vs le scénario modifié. | `front/src/components/ComparisonView.vue` |
| `P5.15` | Légende et métriques | Affichage : T_min, T_max, T_moyen, écart-type. Échelle de couleur. UHI index (Urban Heat Island intensity). | `front/src/components/ResultLegend.vue` |

#### 5e. Export et partage

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P5.16` | Export des résultats | PDF/PNG du rendu 3D actuel. CSV des données de simulation. GeoJSON des zones modifiées. | `front/src/composables/useExport.ts` |
| `P5.17` | Partage de scénario | URL partageable pointant vers un scénario sauvegardé. | Routing + backend endpoint |

**Critère de sortie** : L'utilisateur peut créer un projet, modifier la scène, lancer une simulation, et visualiser les résultats thermiques et de vent en 3D.

---

### Phase 6 — Intégration ONNX end-to-end

**Objectif** : Connecter le modèle entraîné au backend, et le backend au frontend, pour une boucle complète.

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P6.1` | Intégration du .onnx dans le backend | Copier le modèle exporté. Configurer le chemin via variable d'env `KLIMA_MODEL_PATH`. Vérifier l'inférence sur les cas de test. | Config + tests d'intégration |
| `P6.2` | Pipeline front → back → front | Test end-to-end : modifier la scène dans le frontend → appel API → inférence → affichage résultat. Mesurer la latence totale. | Test E2E (Playwright ou Cypress) |
| `P6.3` | Optimisation du transfert de données | Évaluer : JSON vs MessagePack vs Protocol Buffers pour le transfert des champs 3D. Compression zstd côté serveur. Cible : payload < 500 Ko pour un champ 250×250×50. | Benchmark + implémentation |
| `P6.4` | Mode dégradé (sans modèle) | Si aucun `.onnx` n'est chargé : afficher un message clair dans le frontend, proposer un jeu de données de démonstration pré-calculé. | UI + mock data |

**Critère de sortie** : La boucle complète fonctionne en < 500 ms (réseau local). Le mode dégradé est fonctionnel.

---

### Phase 7 — Validation scientifique & Production

**Objectif** : Valider les prédictions contre des observations réelles et préparer le déploiement.

#### 7a. Validation

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P7.1` | Validation satellite | Comparer les prédictions de température de surface (LST) avec les données Landsat 8 / Sentinel-3 sur le quartier test. Calcul du biais moyen et du RMSE spatial. | `training/notebooks/validate_satellite.ipynb` |
| `P7.2` | Validation in-situ (si disponible) | Comparer avec des mesures de stations météo urbaines (réseau APUR à Paris, par ex.). | Rapport de validation |
| `P7.3` | Tests de sensibilité | Vérifier que le modèle répond correctement aux perturbations physiques attendues : augmenter l'albédo d'un toit → T diminue ; ajouter des arbres → T diminue sous la canopée. | `training/notebooks/sensitivity_tests.ipynb` |
| `P7.4` | Documentation scientifique | Rapport méthodologique complet : architecture du modèle, hyperparamètres, métriques, limitations connues. | `docs/scientific_report.md` |

#### 7b. Production

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P7.5` | Dockerfiles de production | Multi-stage builds optimisés (Rust release, frontend build statique, Nginx). | `{back,front}/docker/Dockerfile.prod` |
| `P7.6` | Docker Compose production | Compose avec Nginx reverse proxy, HTTPS (Let's Encrypt), health checks. | `docker-compose.prod.yml` |
| `P7.7` | Monitoring et observabilité | Métriques Prometheus (latence inférence, requêtes/s). Dashboards Grafana. | `monitoring/` |
| `P7.8` | Documentation utilisateur | Guide pour les urbanistes : comment utiliser l'outil, interpréter les résultats, limites du modèle. | `docs/user_guide.md` |

---

## 4. Dimensionnement scientifique

### 4.1 Résolution spatiale

| Paramètre | Valeur | Justification |
|-----------|--------|---------------|
| Δx = Δy | 2 m | Résolution suffisante pour distinguer une rue d'un trottoir. Cohérent avec la BD TOPO (précision ~1m). |
| Δz | 2 m | Capture les variations verticales de température et vent dans les rues (canyon urbain). |
| Domaine horizontal | 500 × 500 m | Un quartier complet, suffisant pour capturer les effets de sillage des bâtiments. |
| Domaine vertical | 100 m | ~3× la hauteur max des bâtiments standard (Hmax ≈ 30 m). Nécessaire pour les conditions limites supérieures. |
| Taille grille | 250 × 250 × 50 | = 3 125 000 voxels |

### 4.2 Modèle CFD de référence

| Paramètre | Choix | Justification |
|-----------|-------|---------------|
| Solveur | OpenFOAM `buoyantSimpleFoam` | RANS stationnaire avec flottabilité thermique. Bon compromis précision/coût pour la génération de données. |
| Modèle de turbulence | k-ω SST | Meilleure performance que k-ε dans les couches limites et les zones de recirculation urbaines. |
| Rayonnement | `viewFactor` + modèle simplifié | Prise en compte des réflexions multi-surfaces et de l'ombrage. |
| Conditions limites inlet | Profil logarithmique de vent | $U(z) = \frac{u_*}{\kappa} \ln\left(\frac{z + z_0}{z_0}\right)$ avec $z_0 = 0.5$ m (terrain urbain). |

### 4.3 Architecture du réseau neuronal

| Paramètre | U-Net 3D | GNN (alternative) |
|-----------|----------|---------------------|
| Input | Tenseur (B, 12, 250, 250, 50) | Graphe ~500K nœuds (voxels air) |
| Paramètres | ~8-15M | ~2-5M |
| Mémoire entraînement | ~12-24 Go GPU | ~6-12 Go GPU |
| Latence inférence (CPU) | ~50-200 ms | ~100-500 ms |
| Avantage | Exploite la structure régulière | Épars, généralise mieux à des géométries variées |
| Inconvénient | Gaspille du calcul dans les zones pleines (bâtiments) | Plus lent, implémentation plus complexe |

**Recommandation** : Commencer avec le U-Net 3D (Phase 3.2), évaluer les performances, puis explorer le GNN (Phase 3.4) si les résultats sont insuffisants ou si la généralisation pose problème.

### 4.4 Fonction de perte détaillée

$$\mathcal{L}_{total} = \underbrace{\lambda_1 \frac{1}{N_{air}} \sum_{i \in \text{air}} (T_i^{pred} - T_i^{cfd})^2}_{\text{MSE température}} + \underbrace{\lambda_2 \frac{1}{N_{air}} \sum_{i \in \text{air}} \|\mathbf{v}_i^{pred} - \mathbf{v}_i^{cfd}\|^2}_{\text{MSE vent}} + \underbrace{\lambda_3 \frac{1}{N_{air}} \sum_{i \in \text{air}} (\nabla \cdot \mathbf{v}_i^{pred})^2}_{\text{Contrainte physique : divergence}} + \underbrace{\lambda_4 \frac{1}{N_{sol}} \sum_{i \in \text{solide}} \|\mathbf{v}_i^{pred}\|^2}_{\text{Contrainte physique : no-slip}}$$

Valeurs initiales recommandées : λ₁ = 1.0, λ₂ = 1.0, λ₃ = 0.1, λ₄ = 10.0 (fort pour forcer le no-slip).

---

## 5. Registre des risques

| # | Risque | Impact | Probabilité | Mitigation |
|---|--------|--------|-------------|------------|
| R1 | Simulations OpenFOAM trop lentes → dataset insuffisant | Modèle peu précis | Moyenne | Commencer avec un domaine réduit (250×250 m). Utiliser des solveurs simplifiés (ENVI-met) en complément. |
| R2 | U-Net 3D trop gourmand en mémoire pour 250³ | Impossibilité d'entraîner | Moyenne | Patchification (patches 64³ avec overlap). Ou réduire la résolution à 4 m. |
| R3 | Le modèle surrogate ne généralise pas à des géométries non vues | Prédictions aberrantes quand l'utilisateur modifie la scène | Haute | Augmentation de données (rotations, flips). Entraîner sur plusieurs quartiers. Mécanisme de détection d'outlier. |
| R4 | Latence d'inférence ONNX > 100 ms | Expérience utilisateur dégradée | Faible | Quantization INT8, modèle plus léger, inférence GPU côté serveur. |
| R5 | Cesium Ion rate limiting ou changement de pricing | Frontend 3D cassé | Faible | Supporter des tuiles 3D auto-hébergées (serveur de tuiles local). |
| R6 | Pas de données de validation in-situ | Impossible de vérifier la qualité des prédictions | Moyenne | Utiliser la validation satellite (Landsat) + tests de cohérence physique (sensibilité). |

---

## 6. Matrice des tâches & parallélisation

### 6.1 Graphe de dépendances

```
P0 (Fondations)
 │
 ├──────────────────────────┬─────────────────────────────┐
 │                          │                             │
 ▼                          ▼                             ▼
P1 (Données géo)      P4a (CRUD API)              P5a-b (Stores, UI projets)
 │                          │                             │
 ▼                          │                             ▼
P2 (CFD dataset)            │                     P5c (Éditeur de scène)
 │                          │                             │
 ▼                          ▼                             ▼
P3 (Entraînement ML)   P4b (Service ONNX)         P5d (Visualisation)
 │                          │                             │
 └────────────┬─────────────┘                             │
              │                                           │
              ▼                                           │
         P6 (Intégration E2E) ◄───────────────────────────┘
              │
              ▼
         P7 (Validation & Prod)
```

### 6.2 Matrice de parallélisation par agent

Chaque colonne représente un **agent autonome** (sous-agent) pouvant travailler en parallèle. Les lignes représentent des **slots temporels** (itérations). Une cellule vide signifie que l'agent attend une dépendance.

```
╔════════════╦═══════════════════════╦══════════════════════╦═══════════════════════╦══════════════════════╗
║   Slot     ║  Agent A              ║  Agent B             ║  Agent C              ║  Agent D             ║
║            ║  DATA / ML PIPELINE   ║  BACKEND RUST        ║  FRONTEND VUE         ║  DEVOPS / DOCS       ║
╠════════════╬═══════════════════════╬══════════════════════╬═══════════════════════╬══════════════════════╣
║            ║                       ║                      ║                       ║                      ║
║  Slot 1    ║  P0.6  Docker Python  ║  P0.2  Tests back    ║  P0.3  Tests front    ║  P0.1  CI/CD         ║
║            ║  P0.7  Structure      ║  P0.4  Error types   ║                       ║  P0.5  OpenAPI spec  ║
║            ║       training/       ║                      ║                       ║                      ║
╠════════════╬═══════════════════════╬══════════════════════╬═══════════════════════╬══════════════════════╣
║            ║                       ║                      ║                       ║                      ║
║  Slot 2    ║  P1.1  Quartier test  ║  P4.1  CRUD Projects ║  P5.1  Pinia stores   ║  docs/api/           ║
║            ║  P1.2  BD TOPO        ║  P4.2  CRUD Scenar.  ║  P5.2  useApi()       ║  openapi.yml         ║
║            ║  P1.3  LiDAR          ║  P4.3  Simulations   ║  P5.3  Types partagés ║                      ║
║            ║  P1.4  Urban Atlas    ║  P4.4  Migrations     ║                       ║                      ║
║            ║  P1.5  ERA5/Météo     ║  P4.5  Validation    ║                       ║                      ║
╠════════════╬═══════════════════════╬══════════════════════╬═══════════════════════╬══════════════════════╣
║            ║                       ║                      ║                       ║                      ║
║  Slot 3    ║  P1.6  Fusion géo     ║  P4.10 Compression   ║  P5.4  Page projets   ║                      ║
║            ║  P1.7  Voxelisation   ║  P4.11 Cache LRU     ║  P5.5  Dialog projet  ║                      ║
║            ║  P1.8  Props phys.    ║  P4.12 Geodata API   ║  P5.6  Routing        ║                      ║
║            ║  P1.9  Validation     ║                      ║                       ║                      ║
╠════════════╬═══════════════════════╬══════════════════════╬═══════════════════════╬══════════════════════╣
║            ║                       ║                      ║                       ║                      ║
║  Slot 4    ║  P2.1  OpenFOAM dock  ║  P4.6  OnnxService   ║  P5.7  Toolbar        ║                      ║
║            ║  P2.2  Voxel→mesh     ║  P4.7  Préprocesseur ║  P5.8  Surface editor ║                      ║
║            ║  P2.3  Case template  ║  P4.8  Postprocess.  ║  P5.9  Object placer  ║                      ║
║            ║  P2.4  Scénarios LHS  ║  P4.9  Simulate real ║  P5.10 Sérialisation  ║                      ║
╠════════════╬═══════════════════════╬══════════════════════╬═══════════════════════╬══════════════════════╣
║            ║                       ║                      ║                       ║                      ║
║  Slot 5    ║  P2.5  Batch runner   ║  P4.13 WebSocket     ║  P5.11 Heatmap 3D     ║                      ║
║            ║  P2.6  CFD parser     ║       (optionnel)    ║  P5.12 Wind particles ║                      ║
║            ║  P2.7  Dataset build  ║                      ║  P5.13 Time slider    ║                      ║
║            ║  P2.8  Validation     ║                      ║  P5.14 Comparaison    ║                      ║
║            ║                       ║                      ║  P5.15 Légende        ║                      ║
╠════════════╬═══════════════════════╬══════════════════════╬═══════════════════════╬══════════════════════╣
║            ║                       ║                      ║                       ║                      ║
║  Slot 6    ║  P3.1  Encoding       ║       ← attend      ║  P5.16 Export         ║                      ║
║            ║  P3.2  U-Net 3D       ║         P3.9         ║  P5.17 Partage        ║                      ║
║            ║  P3.3  Loss PINN      ║                      ║                       ║                      ║
║            ║  P3.5  DataLoader     ║                      ║                       ║                      ║
╠════════════╬═══════════════════════╬══════════════════════╬═══════════════════════╬══════════════════════╣
║            ║                       ║                      ║                       ║                      ║
║  Slot 7    ║  P3.6  Train          ║       ← attend      ║       ← attend       ║                      ║
║            ║  P3.7  HPO            ║         P3.9         ║         P6            ║                      ║
║            ║  P3.8  Evaluate       ║                      ║                       ║                      ║
╠════════════╬═══════════════════════╬══════════════════════╬═══════════════════════╬══════════════════════╣
║            ║                       ║                      ║                       ║                      ║
║  Slot 8    ║  P3.9  Export ONNX    ║                      ║                       ║                      ║
║            ║  P3.10 Benchmark      ║                      ║                       ║                      ║
║            ║  P3.11 Optimize ONNX  ║                      ║                       ║                      ║
╠════════════╬═══════════════════════╬══════════════════════╬═══════════════════════╬══════════════════════╣
║            ║                       ║                      ║                       ║                      ║
║  Slot 9    ║                       ║  P6.1  Intégration   ║  P6.2  Test E2E       ║  P6.4  Mode dégradé  ║
║  (join)    ║                       ║        .onnx         ║  P6.3  Optim transfer ║                      ║
╠════════════╬═══════════════════════╬══════════════════════╬═══════════════════════╬══════════════════════╣
║            ║                       ║                      ║                       ║                      ║
║  Slot 10   ║  P7.1  Valid. sat.    ║  P7.5  Docker prod   ║  P7.8  User guide     ║  P7.7  Monitoring    ║
║            ║  P7.2  Valid. in-situ ║  P7.6  Compose prod  ║                       ║                      ║
║            ║  P7.3  Sensibilité    ║                      ║                       ║                      ║
║            ║  P7.4  Rapport sci.   ║                      ║                       ║                      ║
╚════════════╩═══════════════════════╩══════════════════════╩═══════════════════════╩══════════════════════╝
```

### 6.3 Résumé des dépendances critiques

| Tâche bloquée | Attend | Raison |
|---------------|--------|--------|
| P1.6-P1.9 | P1.1-P1.5 | Besoin des données brutes pour fusionner et voxeliser |
| P2.1-P2.4 | P1.7 | Le maillage CFD est généré depuis la grille voxel |
| P2.5 | P2.1-P2.4 | Besoin du template de cas et du générateur de scénarios |
| P3.5-P3.8 | P2.7 | Besoin du dataset pour entraîner |
| P3.9 | P3.6-P3.8 | Besoin d'un modèle entraîné pour l'exporter |
| P4.6-P4.9 | P0.5 (contrat) | Le format tenseur doit être spécifié. Peut utiliser un mock .onnx en attendant P3.9. |
| P5.11-P5.12 | P5.2, P5.10 | Besoin du service API et du format de sérialisation |
| P6.* | P3.9 + P4.9 + P5.11 | Intégration = jonction de tous les flux |
| P7.* | P6.* | Validation = le système fonctionne de bout en bout |

### 6.4 Chemin critique

Le chemin critique (séquence la plus longue) est :

**P0 → P1.1-P1.5 → P1.6-P1.9 → P2.1-P2.4 → P2.5-P2.8 → P3.1-P3.8 → P3.9 → P6 → P7**

Ce chemin est dicté par le **pipeline de données et ML**. C'est lui qui détermine le temps total du projet. Les Agents B (backend) et C (frontend) avancent en parallèle et sont prêts *avant* que le modèle ONNX ne soit disponible — ils travaillent avec des mocks en attendant.

### 6.5 Stratégie de mocks pour débloquer les agents

| Agent | Ce qu'il mocke | Quand le mock est remplacé |
|-------|----------------|---------------------------|
| Agent B (Backend) | Fichier `.onnx` factice (2 couches linéaires, mêmes dimensions I/O) | Quand P3.9 livre le vrai modèle |
| Agent C (Frontend) | Réponses API simulées (données aléatoires mais structurellement correctes) | Quand P4.9 est fonctionnel |
| Agent D (DevOps) | Aucun mock nécessaire | — |
