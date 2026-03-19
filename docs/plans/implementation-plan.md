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
| Base de données | Schema PostgreSQL 16 (projects, simulations, scenarios) via sqlx | `back/src/db/mod.rs` |

### Ce qui manque pour un produit fonctionnel

1. **Pipeline de données Python** — acquisition géospatiale, classification LiDAR, voxelisation, CFD
2. **Modèle IA Surrogate** — entraînement Local-FNO PINN, export ONNX
3. **Backend fonctionnel** — CRUD, inférence ONNX réelle, compression résultats
4. **Frontend fonctionnel** — édition de scène, appel API, visualisation thermique + vent
5. **Validation scientifique** — comparaison prédictions vs vérité terrain (satellite + cohérence physique)
6. **Infrastructure de production** — CI/CD, monitoring, déploiement

---

## 2. Principes directeurs

1. **Vertical slice first** : livrer une tranche fonctionnelle end-to-end sur un périmètre réduit (un quartier, un scénario) avant d'élargir.
2. **Contract-driven** : les interfaces API (OpenAPI) et les formats de tenseurs sont définis *avant* l'implémentation.
3. **Reproductibilité scientifique** : tout script de données et d'entraînement est versionné, déterministe (seeds fixés), et documenté.
4. **Containerisation systématique** : tout tourne dans Docker, y compris le pipeline Python et OpenFOAM.
5. **Parallélisation maximale** : les 3 piliers (data/ML, backend, frontend) avancent en parallèle via des contrats d'interface stables.
6. **Séparation domaines CFD / ML** : le domaine CFD est plus grand que le domaine d'intérêt (avec marges pour les conditions limites). Le réseau neuronal ne voit que le domaine intérieur extrait.

---

## 3. Phases d'implémentation

---

### Phase 0 — Fondations & Outillage DX

**Objectif** : Rendre le monorepo robuste, testable et automatisé avant tout développement fonctionnel.

| ID | Tâche | Livrable | Dépendances |
|----|-------|----------|-------------|
| `P0.1` | CI/CD GitHub Actions : lint (clippy + eslint), test, build | `.github/workflows/ci.yml` | — |
| `P0.2` | Infrastructure de test backend (cargo test, tests d'intégration avec PostgreSQL de test via testcontainers ou base dédiée) | `back/tests/` | — |
| `P0.3` | Infrastructure de test frontend (Vitest, tests composants Vue) | `front/vitest.config.ts`, `front/src/**/*.test.ts` | — |
| `P0.4` | Gestion d'erreurs structurée backend (type `AppError`, codes HTTP cohérents) | `back/src/error.rs` | — |
| `P0.5` | Spécification OpenAPI du contrat d'API (JSON Schema des requêtes/réponses) + spécification du format tenseur I/O pour le modèle ONNX | `docs/api/openapi.yml`, `docs/api/tensor-spec.md` | — |
| `P0.6` | Pipeline Python : Dockerfile + docker-compose (Python 3.12, PyTorch, NVIDIA Modulus / neuraloperator, GeoPandas, laspy, PyVista) | `training/docker/` | — |
| `P0.7` | Structure du dossier `training/` dans le monorepo | `training/{src,data,models,configs}/` | — |
| `P0.8` | Mock ONNX model (2 couches linéaires, mêmes dimensions I/O que la spec tenseur) pour débloquer le backend et le frontend | `training/src/model/mock_onnx.py` → `back/models/mock.onnx` | P0.5 |

**Critère de sortie** : `cargo test`, `npm run test`, et le workflow CI passent au vert. Le mock .onnx est généré et fonctionnel.

---

### Phase 1 — Acquisition & Préparation des données géospatiales

**Objectif** : Constituer la géométrie 3D et les métadonnées de surface d'un quartier test exploitable par le solveur CFD.

#### 1a. Définition du domaine d'étude

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P1.1` | Sélection du quartier test | Zone dense, ~500 m × 500 m d'intérêt. Candidats : Paris 11e (rue Oberkampf), Lyon Part-Dieu, Toulouse Capitole. Critères : disponibilité BD TOPO + LiDAR HD + Urban Atlas. Le **domaine CFD** inclut une marge de 5H (~160 m) autour de la zone d'intérêt (total ~820 × 820 m) pour des conditions limites propres (cf. COST 732, Franke et al. 2007). | `training/configs/domain.yaml` (bbox WGS84, CRS, résolution, marges) |
| `P1.2` | Téléchargement BD TOPO IGN | Emprise au sol + hauteur des bâtiments. Format shapefile/GeoJSON. Script reproductible. Couvre le domaine CFD élargi. | `training/src/data/download_bdtopo.py` |
| `P1.3` | Téléchargement LiDAR HD IGN | Nuages de points (fichiers LAZ). Couvre le domaine CFD élargi. | `training/src/data/download_lidar.py` |
| `P1.4` | Téléchargement Urban Atlas (Copernicus) | Classification d'occupation des sols (~2.5 m résolution). | `training/src/data/download_urban_atlas.py` |
| `P1.5` | Téléchargement ERA5 / Météo-France | Données **méso-échelle** : T_air de fond, vent synoptique, rayonnement solaire global, humidité. Ces données servent de **conditions aux limites** (pas de conditions locales, la résolution ERA5 est ~31 km). | `training/src/data/download_meteo.py` |

#### 1b. Traitement géométrique

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P1.6` | Classification LiDAR | Classifier les points LiDAR en {sol, bâtiment, végétation haute, végétation basse} via Cloth Simulation Filter (sol) + analyse des caractéristiques de retour (bâti vs végétation). Extraction du MNS (Modèle Numérique de Surface) et du MNH (Modèle Numérique de Hauteur de canopée). | `training/src/preprocessing/lidar_classify.py` |
| `P1.7` | Fusion des sources géométriques | Combiner BD TOPO (bâtiments) + LiDAR classifié (canopée) + Urban Atlas (sols). Projection vers Lambert-93 (EPSG:2154). Résolution de conflits entre sources. | `training/src/preprocessing/geometry_fusion.py` |
| `P1.8` | Voxelisation 3D | Transformer la géométrie en grille régulière 3D. Résolution Δx = Δy = Δz = 2 m. **Domaine CFD** : ~416×416×96 voxels (832×832×192 m). **Domaine ML** (extraction intérieure) : **256×256×64** voxels (512×512×128 m). Les dimensions ML sont des **multiples de 16** (requis pour un U-Net 3D à 4 niveaux de pooling). Chaque voxel porte un label : {air, bâtiment, sol_bitume, sol_herbe, sol_eau, sol_gravier, végétation}. | `training/src/preprocessing/voxelizer.py` |
| `P1.9` | Attribution des propriétés physiques | Chaque voxel de surface reçoit : albédo (α), émissivité (ε), conductivité thermique (k), rugosité aérodynamique (z₀). Lookup table calibrée depuis Urban Atlas et littérature (Oke et al. 2017, *Urban Climates*). | `training/src/preprocessing/surface_properties.py` |
| `P1.10` | Validation et visualisation | Visualisation 3D de la grille voxelisée (PyVista). Vérification des volumes bâtis vs BD TOPO, détection des artefacts, couverture LiDAR. | `training/notebooks/validate_geometry.ipynb` |
| `P1.11` | Mapping coordonnées voxel ↔ WGS84 | Table de correspondance bidirectionnelle entre indices (i,j,k) de la grille voxel et coordonnées géographiques (lon,lat,alt). Nécessaire pour la Phase 5 (frontend CesiumJS). | `training/src/preprocessing/coord_mapping.py` |

**Critère de sortie** : Grille CFD `.npy` (~416×416×96×C) et grille ML `.npy` (256×256×64×C) sauvegardées. Table de mapping coordonnées versionnée.

---

### Phase 2 — Génération du dataset CFD

**Objectif** : Produire les paires [entrée → sortie] qui serviront de vérité terrain pour l'entraînement du réseau neuronal.

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P2.1` | Conteneurisation OpenFOAM | Dockerfile basé sur `openfoam/openfoam2406-default`. Scripts de setup et lancement. | `training/docker/Dockerfile.openfoam` |
| `P2.2` | Convertisseur voxels → maillage OpenFOAM | Transformer la grille voxelisée **CFD** en maillage `blockMesh` + `snappyHexMesh`. Boundary conditions : inlet (profil de vent), outlet (zero gradient), ground (wall + flux thermique), top (symmetry), sides (cyclic ou zero gradient). | `training/src/cfd/voxel_to_mesh.py` |
| `P2.3` | Template de cas OpenFOAM | Solveur `buoyantSimpleFoam` (RANS stationnaire, Boussinesq). Turbulence k-ω SST. Rayonnement `fvDOM` (Finite Volume Discrete Ordinates, plus scalable que `viewFactor` sur maillages > 1M faces). Fichiers `0/`, `constant/`, `system/`. | `training/src/cfd/case_template/` |
| `P2.4` | Générateur paramétrique de scénarios | Échantillonnage Latin Hypercube (LHS) sur l'espace des paramètres : wind_speed ∈ [0.5, 15] m/s, wind_direction ∈ [0°, 360°), sun_elevation ∈ [10°, 80°], albédo_toits ∈ [0.1, 0.8], T_air_ambiant ∈ [20, 40] °C. Cible : **N = 24 à 50 simulations** (le FNO est data-efficient grâce à la régularisation PINN ; le Local-FNO a démontré des résultats sur 24 sims pour un quartier de 1.2 km). | `training/src/cfd/scenario_generator.py` |
| `P2.5` | Runner batch de simulations | Orchestrateur qui lance les N simulations OpenFOAM (via GNU parallel ou job queue). Capture des résultats, gestion des échecs et relances, logging. | `training/src/cfd/batch_runner.py` |
| `P2.6` | Parseur de résultats CFD | Extraction des champs T(x,y,z) et v(x,y,z) depuis les fichiers OpenFOAM. Ré-échantillonnage sur la grille voxel ML (256×256×64) : interpolation trilinéaire depuis le maillage CFD non-structuré. Conversion de T absolu en **ΔT = T - T_ambiant** (le réseau prédit l'écart à la température ambiante, plus robuste et généralisable). | `training/src/cfd/result_parser.py` |
| `P2.7` | Constitution du dataset final | Concaténation des N paires. Format HDF5 avec groupes `inputs/` et `outputs/`. Séparation train/val/test (70/15/15, stratifié par wind_speed et sun_elevation). Métadonnées : paramètres de chaque simulation. | `training/src/cfd/dataset_builder.py` |
| `P2.8` | Validation statistique du dataset | Distributions de ΔT et |v|. Vérification conservation de masse (∇·v ≈ 0, tolérance attendue ~5% car Boussinesq). Corrélations entre paramètres d'entrée et sorties. Détection d'outliers (simulations divergées). | `training/notebooks/validate_dataset.ipynb` |

**Critère de sortie** : Dataset HDF5 de ~300 paires, documenté, avec splits train/val/test et rapport de qualité.

**Dimensionnement** :
- 50 simulations × ~3-4h/sim (RANS k-ω SST + fvDOM sur maillage ~5M cells) = ~175 h CPU
- Parallélisé sur 16 cœurs : ~11h wall-clock
- Taille dataset (domaine ML uniquement) : 50 × (256×256×64×4) × 4 bytes ≈ **3.4 Go**
- Réduction drastique vs CNN classique (~300 sims) grâce à l'efficacité du FNO + régularisation PINN

---

### Phase 3 — Entraînement du Local-FNO PINN

**Objectif** : Entraîner un Fourier Neural Operator (FNO) localisé, contraint par la physique (PINN), capable de prédire ΔT(x,y,z) et v(x,y,z) en < 200 ms — avec **zero-shot super-résolution** (évaluation à n'importe quelle résolution sans ré-entraînement).

#### 3a. Architecture du modèle

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P3.1` | Encodage des entrées | Canaux d'entrée du tenseur (grille 256×256×64) : **géométrie** — occupancy binaire (1), type de surface one-hot (6 classes → 6), propriétés physiques continues (α, ε, z₀ → 3) ; **conditions limites broadcastées** — wind_speed (1), wind_dir encodé (sin θ, cos θ → 2), sun_elevation (1), T_ambiant (1). **Total : 15 canaux d'entrée.** La direction du vent est encodée en (sin, cos) pour éviter la discontinuité 0°/360°. | `training/src/model/encoding.py` |
| `P3.2` | Architecture Local-FNO | **Couches Fourier spectrales** : chaque couche applique FFT 3D → multiplication par un noyau appris dans l'espace de Fourier (tronqué aux k_max premiers modes) → IFFT. 4 couches Fourier empilées. **Couches locales** : Conv3D 1×1×1 en résidu (bypass) pour capturer les interactions locales (sillage, turbulence de rue). Canaux de sortie : **4** (ΔT, vx, vy, vz). Implémentation via **NVIDIA Modulus** ou **neuraloperator** (PyTorch). k_max = 16 (modes de Fourier retenus par dimension). | `training/src/model/local_fno.py` |
| `P3.3` | Fonction de perte PINN complète | 5 termes (voir section 4.4) : MSE(ΔT), MSE(v), divergence ∇·v ≈ 0, no-slip v=0 dans les solides, **diffusion thermique** ∂T/∂t − α∇²T = 0. Le gradient discret est calculé par **différences finies centrales** sur la grille régulière. Un masque binaire sépare les voxels air des voxels solides. | `training/src/model/loss.py` |
| `P3.4` | Pipeline GeoJSON-to-Tensor | Convertisseur qui transforme un GeoJSON (polygone dessiné par l'utilisateur) en tenseur d'albédo/émissivité sur la grille voxel. Rasterisation du polygone → mise à jour du canal de type de surface et des propriétés physiques dans le tenseur d'entrée. Réutilisé côté backend Rust (P4.7). | `training/src/model/geojson_to_tensor.py` |

#### 3b. Pipeline d'entraînement

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P3.5` | DataLoader PyTorch | Chargement HDF5. Normalisation z-score par canal (μ et σ calculés sur le train set, sauvegardés pour le backend). Augmentation : rotation 90°/180°/270° + flip horizontal/vertical. **Lors d'une rotation de k×90°, le vecteur vent (vx,vy) et l'encodage (sin θ, cos θ) de la direction d'entrée sont transformés de manière cohérente** (rotation de la matrice 2D correspondante). | `training/src/model/dataloader.py` |
| `P3.6` | Boucle d'entraînement | Optimizer AdamW (weight decay 1e-4). Scheduler OneCycleLR (warm-up 5 epochs). Gradient clipping (max_norm=1.0). Logging TensorBoard ou W&B. Early stopping sur val_loss (patience=20). Mixed precision (AMP). **Le FNO est data-efficient : 24-50 simulations suffisent grâce à la régularisation PINN.** | `training/src/model/train.py` |
| `P3.7` | Recherche d'hyperparamètres | Grid/random search sur : learning_rate ∈ [1e-4, 5e-3], λ₁..λ₅ (5 termes de loss), k_max (modes de Fourier : 8, 12, 16, 24), nombre de couches Fourier (3-6), largeur des couches (32, 64, 128). | `training/src/model/hparam_search.py` |
| `P3.8` | Évaluation quantitative | Métriques sur le test set : MAE, RMSE, R² pour ΔT et |v|. Ventilation par zone (rue/canyon, toit, parc, ombre/soleil). **Test de super-résolution** : entraîner à 2 m, évaluer à 1 m et 0.5 m sans ré-entraînement (zero-shot). Comparer à la référence CFD interpolée. | `training/src/model/evaluate.py` |

#### 3c. Export et validation du modèle

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P3.9` | Export ONNX | `torch.onnx.export()` avec **dynamic axes** pour batch_size ET résolution spatiale (le FNO accepte des grilles de taille variable). Validation croisée PyTorch vs ONNX Runtime sur 10 cas de test (atol=1e-5). Sauvegarder les paramètres de normalisation (μ, σ). | `training/src/model/export_onnx.py` → `models/klima_v1.onnx` + `models/klima_v1_norm.json` |
| `P3.10` | Benchmark de latence | Mesurer le temps d'inférence sur CPU (8 threads) et GPU pour : (a) résolution standard (256×256×64), (b) haute résolution (512×512×128 — zoom). Cible : **< 200 ms CPU standard, < 500 ms CPU haute-res**. | `training/src/model/benchmark.py` |
| `P3.11` | Optimisation du graphe ONNX | ORT graph optimizations (Level3). La FFT 3D dans ONNX est supportée via les opérateurs DFT. Vérifier la compatibilité. Quantization INT8 dynamique si nécessaire. | `training/src/model/optimize_onnx.py` |

**Critère de sortie** : Fichier `.onnx` validé, RMSE(ΔT) < 1.0 °C, MAE(ΔT) < 0.5 °C, RMSE(|v|) < 0.5 m/s, latence < 200 ms CPU, super-résolution ×2 fonctionnelle.

---

### Phase 4 — Backend API complet

**Objectif** : Transformer le squelette Axum en API robuste, avec CRUD, inférence, et gestion des projets.

#### 4a. API CRUD et gestion des données

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P4.1` | CRUD Projects | `POST/GET/PUT/DELETE /api/projects`. Pagination sur le listing. Tests d'intégration. | `back/src/routes/projects.rs` |
| `P4.2` | CRUD Scenarios | `POST/GET/PUT/DELETE /api/projects/:id/scenarios`. Un scénario contient : géométrie modifiée (diff par rapport à la baseline), paramètres météo. | `back/src/routes/scenarios.rs` |
| `P4.3` | Gestion des résultats de simulation | `GET /api/simulations/:id`. Stockage des résultats en BYTEA compressé (zstd) dans PostgreSQL. | `back/src/routes/simulations.rs` |
| `P4.4` | Migrations versionnées | Système de migration incrémentale (numérotées) au lieu d'un `CREATE IF NOT EXISTS` monolithique. | `back/src/db/migrations/` |
| `P4.5` | Validation des entrées | Validation structurelle des requêtes (bornes, types, tailles max). Utiliser `validator` ou validation manuelle. | `back/src/validation.rs` |

#### 4b. Inférence ONNX

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P4.6` | Service ONNX (`OnnxService`) | Chargement du modèle `.onnx` au démarrage via `ort::Session`. Partagé via `Arc<OnnxService>` dans l'AppState. Gestion de l'absence de modèle (mode dégradé). Chargement des paramètres de normalisation (μ, σ) depuis le JSON adjacent. Versioning : supporte le chargement de différentes versions du modèle. | `back/src/inference/mod.rs` |
| `P4.7` | Préprocesseur GeoJSON → tenseur | **Pipeline GeoJSON-to-Tensor** : le frontend envoie des polygones GeoJSON (modifications de surface). Le backend les rasterise instantanément sur la grille voxel → mise à jour du tenseur d'albédo/émissivité/type de surface. Applique la normalisation z-score. Pour un zoom haute-res, le FNO est évalué sur une grille plus fine (dynamic axes ONNX). | `back/src/inference/preprocessor.rs` |
| `P4.8` | Postprocesseur tenseur → résultat | Dénormaliser les sorties (ΔT → T = ΔT + T_ambiant). **Extraire uniquement les données utiles au frontend** : (a) températures de surface (sol, toits, façades) = ~100-300K valeurs ; (b) champ de vent sous-échantillonné pour les particules (grille 64×64×16) = ~65K vecteurs. Ne PAS envoyer le champ 3D complet (~4M valeurs). | `back/src/inference/postprocessor.rs` |
| `P4.9` | Endpoint `/api/simulate` réel | Assembler P4.6 + P4.7 + P4.8. Mesurer et logguer le temps d'inférence. Sauvegarder le résultat en base. Format de réponse en 2 parties : `surface_temperatures` (indexé par coordonnées) + `wind_field` (grille sous-échantillonnée). | Mise à jour de `back/src/routes/simulate.rs` |

#### 4c. Fonctionnalités avancées

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P4.10` | Compression des réponses | Middleware de compression gzip/zstd pour les réponses volumineuses. Header `Accept-Encoding`. | Middleware tower-http |
| `P4.11` | Cache de simulation | LRU cache en mémoire (hash des paramètres + géométrie). Évite de relancer une inférence pour des paramètres identiques. | `back/src/cache.rs` |
| `P4.12` | Endpoint de données géographiques | `GET /api/geodata/buildings?bbox=...` — servir les bâtiments baseline depuis la BD TOPO importée. `GET /api/geodata/voxel-mapping` — table de correspondance voxel ↔ WGS84. | `back/src/routes/geodata.rs` |
| `P4.13` | WebSocket pour simulations longues | Channel pour notifier le frontend de la progression. Utile si on ajoute un mode "batch" ou des simulations en file d'attente. | `back/src/routes/ws.rs` |

**Critère de sortie** : API complète, tous les endpoints documentés (OpenAPI), tests d'intégration passent, inférence ONNX fonctionnelle (d'abord avec mock, puis avec vrai modèle).

**Dimensionnement du transfert de données** :

| Donnée | Taille brute | Après zstd | Viable ? |
|--------|-------------|------------|----------|
| Champ 3D complet (256³×64×4×4B) | ~67 Mo | ~7-15 Mo | NON — trop lourd pour du temps réel |
| Températures de surface uniquement (~200K floats) | ~800 Ko | ~80-200 Ko | OUI |
| Vent sous-échantillonné (64×64×16×3×4B) | ~3 Mo | ~300-600 Ko | OUI |
| **Total réponse API** | **~4 Mo** | **~400-800 Ko** | **OUI** |

---

### Phase 5 — Frontend interactif complet

**Objectif** : Construire l'interface complète permettant de visualiser, éditer et simuler.

#### 5a. Infrastructure frontend

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P5.1` | Pinia stores | `useProjectStore`, `useSimulationStore`, `useScenarioStore`. Gestion de l'état applicatif. | `front/src/stores/` |
| `P5.2` | Service API (composable) | `useApi()` : wrapper typé autour de fetch/axios pour tous les endpoints. Gestion erreurs, retry, loading states. | `front/src/composables/useApi.ts` |
| `P5.3` | Types partagés | Interfaces TypeScript miroir du contrat OpenAPI : `Project`, `Scenario`, `SimulateRequest`, `SimulateResponse`, `SurfaceTemperature`, `WindFieldSample`. | `front/src/types/` |

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
| `P5.8` | Outil de modification de surface | Clic ou pinceau sur le sol : changer le type (bitume → herbe → eau → gravier). Feedback visuel immédiat (colorisation de la zone). Utilise le mapping voxel ↔ WGS84 (P1.11 / P4.12) pour convertir le clic CesiumJS en indices de grille. | `front/src/composables/useSurfaceEditor.ts` |
| `P5.9` | Outil de placement d'objets | Placer des arbres (modèle 3D simplifié), du mobilier urbain, des panneaux solaires. Chaque objet a des propriétés physiques (ombre, évapotranspiration). | `front/src/composables/useObjectPlacer.ts` |
| `P5.10` | Sérialisation des modifications | Encoder les modifications de scène en format `GeometryDiff` (liste de voxels modifiés + nouveau type + nouvelles propriétés physiques). C'est ce qui est envoyé à l'API. | `front/src/utils/geometrySerializer.ts` |

#### 5d. Visualisation des résultats

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P5.11` | Overlay thermique (heatmap 3D) | Colorer les surfaces (sol, toits, façades) selon T : échelle bleu (frais) → rouge (chaud). Utilise les `SurfaceTemperature` reçues de l'API (coordonnées géo + valeur). Implémenté via `Cesium.Primitive` avec des attributs de couleur par vertex, ou `Cesium.GroundPrimitive` pour le sol. | `front/src/composables/useThermalOverlay.ts` |
| `P5.12` | Système de particules de vent | Animer des particules (sprites) suivant le champ de vent sous-échantillonné reçu de l'API. Densité proportionnelle à |v|. Interpolation trilinéaire entre les points de la grille. Implémenté via un WebGL shader custom ou le `ParticleSystem` de Cesium. | `front/src/composables/useWindParticles.ts` |
| `P5.13` | Slider temporel (heure du jour) | Contrôle l'élévation solaire. Mode interactif : relance une simulation à chaque changement (debounce 500 ms). Mode pré-calculé : interpole entre simulations stockées. | `front/src/components/TimeSlider.vue` |
| `P5.14` | Mode comparaison avant/après | Split-screen ou slider pour comparer l'état baseline vs le scénario modifié. | `front/src/components/ComparisonView.vue` |
| `P5.15` | Légende et métriques | Affichage : T_min, T_max, T_moyen, écart-type sur la zone d'intérêt. Échelle de couleur dynamique. UHI index (ΔT_urbain - ΔT_référence). Indicateur de confort thermique (UTCI si possible). | `front/src/components/ResultLegend.vue` |

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
| `P6.1` | Intégration du .onnx réel dans le backend | Remplacer le mock par le modèle P3.9. Configurer `KLIMA_MODEL_PATH` et `KLIMA_NORM_PATH`. Vérifier l'inférence sur les cas de test CFD. | Config + tests d'intégration |
| `P6.2` | Pipeline front → back → front | Test end-to-end : modifier la scène dans le frontend → appel API → inférence → affichage résultat. Mesurer la latence totale (cible : < 1 s réseau local, dont ~200 ms inférence + ~300 ms transfert + ~500 ms rendu). | Test E2E (Playwright ou Cypress) |
| `P6.3` | Optimisation du transfert de données | Si les payloads dépassent la cible (< 1 Mo compressé) : évaluer MessagePack au lieu de JSON pour le champ de vent. Profiler et optimiser. | Benchmark + implémentation |
| `P6.4` | Mode dégradé (sans modèle) | Si aucun `.onnx` n'est chargé : afficher un message clair dans le frontend, proposer un jeu de données de démonstration pré-calculé (3-4 scénarios d'exemple). | UI + mock data |

**Critère de sortie** : La boucle complète fonctionne en < 1.5 s (réseau local). Le mode dégradé est fonctionnel.

---

### Phase 7 — Validation scientifique & Production

**Objectif** : Valider les prédictions contre des observations réelles et préparer le déploiement.

#### 7a. Validation

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P7.1` | Validation satellite | Comparer les prédictions de température de surface (LST) avec les données Landsat 8 (100 m thermique, pan-sharpened 30 m) et Sentinel-3 (1 km) sur le quartier test. Attention : la comparaison est qualitative car les résolutions sont très différentes (2 m modèle vs 30-100 m satellite). Calcul du biais moyen et du RMSE sur des agrégats spatiaux. | `training/notebooks/validate_satellite.ipynb` |
| `P7.2` | Validation in-situ (si disponible) | Comparer avec des mesures de stations météo urbaines (réseau APUR à Paris, Météo-France urbain). Analyse des séries temporelles sur des épisodes de canicule. | Rapport de validation |
| `P7.3` | Tests de sensibilité physique | Vérifier que le modèle respecte les comportements attendus : (a) ↑ albédo toit → ↓ ΔT toit ; (b) ajout d'arbres → ↓ ΔT sous canopée ; (c) ↑ wind_speed → ↓ ΔT_max (meilleur brassage) ; (d) ↑ sun_elevation → ↑ ΔT zones exposées ; (e) v ≈ 0 à l'intérieur des bâtiments. Quantifier la magnitude de chaque effet et comparer à la littérature. | `training/notebooks/sensitivity_tests.ipynb` |
| `P7.4` | Documentation scientifique | Rapport méthodologique complet : architecture du modèle, hyperparamètres, métriques, limitations connues, domaine de validité (types de quartiers, plages météo). | `docs/scientific_report.md` |

#### 7b. Production

| ID | Tâche | Détails | Livrable |
|----|-------|---------|----------|
| `P7.5` | Dockerfiles de production | Multi-stage builds optimisés (Rust release, frontend build statique, Nginx). | `{back,front}/docker/Dockerfile.prod` |
| `P7.6` | Docker Compose production | Compose avec Nginx reverse proxy, HTTPS (Let's Encrypt), health checks. | `docker-compose.prod.yml` |
| `P7.7` | Monitoring et observabilité | Métriques Prometheus (latence inférence, requêtes/s, taille des payloads). Dashboards Grafana. Alertes sur latence > 500 ms. | `monitoring/` |
| `P7.8` | Documentation utilisateur | Guide pour les urbanistes : comment utiliser l'outil, interpréter les résultats, limites du modèle, glossaire (UHI, albédo, CFD, etc.). | `docs/user_guide.md` |

---

## 4. Dimensionnement scientifique

### 4.1 Résolution spatiale et domaines

Le projet utilise **deux domaines emboîtés** :

| | Domaine CFD (simulations) | Domaine ML (réseau neuronal) |
|-|---------------------------|------------------------------|
| **Rôle** | Fournir des conditions limites propres | Entrée/sortie du surrogate model |
| **Étendue horizontale** | ~832 × 832 m (zone d'intérêt + marge 5H) | 512 × 512 m (zone d'intérêt) |
| **Étendue verticale** | 192 m (sol + 5H au-dessus de H_max) | 128 m (zone d'intérêt) |
| **Résolution** | Δ = 2 m (uniforme) | Δ = 2 m (uniforme) |
| **Taille grille** | ~416 × 416 × 96 | **256 × 256 × 64** |
| **Nb voxels** | ~16.6 M | **4.2 M** |
| **Justification taille ML** | — | Multiples de 16 (2⁴) requis pour un U-Net 3D à 4 niveaux de MaxPool |

La marge de 5H autour de la zone d'intérêt suit les recommandations COST 732 (Franke et al. 2007) et AIJ (Tominaga et al. 2008) pour les simulations CFD urbaines.

**Justification de la hauteur verticale** : Pour H_max ≈ 30 m (immeubles parisiens typiques), la recommandation est d'avoir le bord supérieur du domaine à ≥ 5H au-dessus du toit le plus haut (Franke et al. 2007). Soit : 30 + 5×30 = 180 m. On arrondit à 192 m pour le domaine CFD (divisible par 2 m × 16 = 32 m). Le domaine ML extrait les 128 m inférieurs (64 niveaux × 2 m), couvrant 0-128 m ce qui englobe tous les bâtiments et la couche de mélange urbaine.

### 4.2 Modèle CFD de référence

| Paramètre | Choix | Justification |
|-----------|-------|---------------|
| Solveur | OpenFOAM `buoyantSimpleFoam` | RANS stationnaire, approximation de Boussinesq (valide pour ΔT < 30 K en milieu urbain). Couple convection forcée et naturelle. |
| Modèle de turbulence | k-ω SST (Menter 1994) | Meilleure performance que k-ε standard dans les couches limites urbaines et zones de recirculation (Blocken 2015). |
| Rayonnement | `fvDOM` (Finite Volume Discrete Ordinates) | Plus scalable que `viewFactor` (O(N) vs O(N²)). 48 directions angulaires (4×12). Prise en compte du rayonnement solaire direct + diffus et des échanges longwave entre surfaces. |
| Conditions limites inlet | Profil logarithmique de vent avec hauteur de déplacement | $U(z) = \frac{u_*}{\kappa} \ln\left(\frac{z - d}{z_0}\right)$ pour $z > d + z_0$, avec $\kappa = 0.41$, $z_0 = 1.0$ m (urbain dense, Grimmond & Oke 1999), $d = 0.7 \times H_{moy}$ (hauteur de déplacement). Pour H_moy ≈ 20 m : d ≈ 14 m. |
| Conditions limites top | Symmetry (zero gradient pour U, p) | Minimise l'influence du bord supérieur sur l'écoulement. |

### 4.3 Architecture du réseau neuronal : Local-FNO

Le **Fourier Neural Operator** (Li et al. 2020) apprend un opérateur dans l'espace des fréquences, ce qui le rend **indépendant de la résolution** de la grille d'entrée. Le **Local-FNO** ajoute des couches convolutives locales pour capturer les phénomènes à petite échelle.

| Paramètre | Local-FNO (recommandé) | U-Net 3D (fallback) |
|-----------|------------------------|----------------------|
| Input | Tenseur (B, 15, N_x, N_y, N_z) — **résolution variable** | Tenseur (B, 15, 256, 256, 64) — résolution fixe |
| Mécanisme | FFT 3D → noyau spectral appris → IFFT + résidu Conv3D local | Encoder-decoder convolutif avec skip connections |
| k_max (modes Fourier) | 16 par dimension (tronque les hautes fréquences) | N/A |
| Paramètres | ~5-12M | ~10-20M |
| Mémoire entraînement | ~8-20 Go GPU (AMP) | ~16-32 Go GPU (AMP) |
| Latence inférence (CPU) | ~100-300 ms (res. standard), ~300-600 ms (res. ×2) | ~100-300 ms (res. fixe uniquement) |
| **Super-résolution** | **OUI** — zero-shot, évalue sur grille plus fine sans ré-entraînement | NON — résolution figée |
| **Data efficiency** | **24-50 simulations** (régularisation PINN) | ~200-300 simulations |
| Avantage clé | Apprend la physique dans l'espace spectral, généralise mieux | Simple, bien compris, convolutions très optimisées |
| Export ONNX | Supporté (FFT via opérateur DFT) | Supporté nativement |

**Pourquoi le FNO est supérieur ici** : Pour un simulateur interactif où l'utilisateur zoome librement, la capacité de changer de résolution à la volée est un avantage décisif. Combiné à la réduction drastique du nombre de simulations CFD requises (24 vs 300), le FNO est le choix optimal pour Klima.

### 4.4 Fonction de perte PINN complète

Le réseau prédit **ΔT = T - T_ambiant** (écart à la température de fond) et **v = (vx, vy, vz)** (vecteur vent). Prédire ΔT plutôt que T absolu améliore la généralisation : le réseau apprend les *perturbations* induites par la géométrie, indépendamment de la température de base.

La loss intègre **5 termes** — 2 de data-fidelity et 3 de contrainte physique :

$$\mathcal{L}_{total} = \underbrace{\lambda_1 \frac{1}{N_{air}} \sum_{i \in \Omega_{air}} (\Delta T_i^{pred} - \Delta T_i^{cfd})^2}_{\text{MSE température (écart)}} + \underbrace{\lambda_2 \frac{1}{N_{air}} \sum_{i \in \Omega_{air}} \|\mathbf{v}_i^{pred} - \mathbf{v}_i^{cfd}\|^2}_{\text{MSE vent}}$$

$$+ \underbrace{\lambda_3 \frac{1}{N_{air}} \sum_{i \in \Omega_{air}} (\nabla \cdot \mathbf{v}_i^{pred})^2}_{\text{Continuité (quasi-incompressible)}} + \underbrace{\lambda_4 \frac{1}{N_{sol}} \sum_{i \in \Omega_{solide}} \|\mathbf{v}_i^{pred}\|^2}_{\text{No-slip (v=0 dans les solides)}}$$

$$+ \underbrace{\lambda_5 \frac{1}{N_{surf}} \sum_{i \in \Omega_{surface}} \left( \frac{\partial \Delta T_i}{\partial t} - \alpha_i \nabla^2 \Delta T_i \right)^2}_{\text{Diffusion thermique (équation de la chaleur)}}$$

Le dernier terme (nouveau vs U-Net) force le modèle à respecter l'**équation de diffusion de la chaleur** aux surfaces. α_i est la diffusivité thermique du matériau au voxel i. C'est ce qui permet à l'utilisateur de voir la chaleur "couler" d'un toit végétalisé vers la rue de manière physiquement cohérente.

Les dérivées partielles sont calculées par **différences finies centrales** sur la grille régulière :

$$\frac{\partial v_x}{\partial x}\bigg|_{i,j,k} = \frac{v_x[i+1,j,k] - v_x[i-1,j,k]}{2 \Delta x}$$

$$\nabla^2 T\bigg|_{i,j,k} = \frac{T[i+1,j,k] - 2T[i,j,k] + T[i-1,j,k]}{\Delta x^2} + (\text{idem } y, z)$$

**Note physique** : La contrainte ∇·v ≈ 0 est une approximation (Boussinesq). La divergence résiduelle dans les données CFD est ~1-5%. Le terme λ₃ est pondéré modérément.

Valeurs initiales recommandées :
- λ₁ = 1.0 (MSE ΔT, données normalisées z-score)
- λ₂ = 1.0 (MSE v, données normalisées z-score)
- λ₃ = 0.01 (divergence — faible car approximatif)
- λ₄ = 10.0 (no-slip — fort pour imposer v=0 dans les solides)
- λ₅ = 0.1 (diffusion thermique — régularisation physique aux surfaces)

---

## 5. Registre des risques

| # | Risque | Impact | Probabilité | Mitigation |
|---|--------|--------|-------------|------------|
| R1 | Simulations OpenFOAM trop lentes (~1000h CPU) → dataset insuffisant ou retardé | Modèle peu précis, chemin critique allongé | Moyenne | (a) Commencer avec un domaine ML réduit (128³). (b) Utiliser des solveurs simplifiés (PALM-4U, ENVI-met) pour un dataset préliminaire. (c) Cloud computing (spot instances). |
| R2 | FNO trop gourmand en mémoire pour la FFT 3D sur 256×256×64 | Impossibilité d'entraîner sur un GPU standard (24 Go) | Faible (FNO plus léger que U-Net) | (a) Mixed precision (AMP). (b) Réduire k_max (modes Fourier). (c) Tronquer les dimensions (grille 128³). (d) Fallback vers U-Net 3D classique si le FNO ne converge pas. |
| R3 | Le modèle ne généralise pas aux géométries modifiées par l'utilisateur | Prédictions aberrantes quand l'utilisateur modifie la scène | Moyenne (le FNO apprend dans l'espace spectral → meilleure généralisation que CNN) | (a) Augmentation de données (rotations, flips) avec transformation cohérente du vent. (b) Entraîner sur 2-3 quartiers variés. (c) Mécanisme de détection d'outlier (incertitude via MC Dropout ou ensemble). (d) Contraintes PINN comme régularisateur. |
| R4 | Latence d'inférence ONNX > 200 ms | Expérience utilisateur dégradée | Faible | (a) Quantization INT8. (b) Modèle plus compact. (c) Inférence GPU côté serveur (CUDA execution provider). |
| R5 | Cesium Ion rate limiting ou changement de pricing | Frontend 3D cassé | Faible | Supporter des tuiles 3D auto-hébergées (3DCityDB + serveur de tuiles local). |
| R6 | Pas de données de validation in-situ à la résolution du modèle (2 m) | Impossible de quantifier l'erreur absolue du modèle | Haute | (a) Validation satellite qualitative (Landsat). (b) Tests de sensibilité physique (le modèle réagit correctement aux perturbations). (c) Validation croisée vs données CFD (test set). (d) À terme : déployer des capteurs IoT. |
| R7 | Artefacts aux bords de la grille ML (effets de bord) | Températures et vents aberrants en périphérie de la zone | Moyenne | (a) La marge de 5H du domaine CFD protège la zone d'intérêt. (b) Padding réfléchissant lors de l'encodage. (c) Masquer les bords dans le postprocesseur (ne pas afficher les 10 premiers/derniers voxels). |
| R8 | L'augmentation par rotation corrompt les données si le vent n'est pas co-tourné | Modèle entraîné sur des données incohérentes | Haute si non détecté | Implémenté dès P3.5 avec tests unitaires vérifiant la cohérence direction d'entrée ↔ champ de vent ↔ rotation. |
| R9 | L'opérateur FFT 3D du FNO n'est pas bien supporté par ONNX Runtime | Export ONNX échoue ou performances dégradées | Moyenne | (a) Vérifier la compatibilité de l'opérateur DFT dans ORT avant de s'engager. (b) Fallback : implémenter la FFT comme des couches PyTorch standard exportables. (c) Dernière option : U-Net 3D classique. |

---

## 6. Matrice des tâches & parallélisation

### 6.1 Graphe de dépendances

```
P0 (Fondations + Mock ONNX)
 │
 ├──────────────────────────┬─────────────────────────────┐
 │                          │                             │
 ▼                          ▼                             ▼
P1 (Données géo)      P4a (CRUD API)              P5a-b (Stores, UI projets)
 │                          │                             │
 ▼                          ▼                             ▼
P2 (CFD dataset)       P4b (Service ONNX          P5c (Éditeur de scène)
 │                       avec mock)                      │
 ▼                          │                             ▼
P3 (Entraînement ML)        │                     P5d (Visualisation)
 │                          │                             │
 ▼                          │                             │
P3.9 (Export .onnx) ────────┤                             │
                            ▼                             │
                    P6 (Intégration E2E) ◄────────────────┘
                            │
                            ▼
                    P7 (Validation & Prod)
```

### 6.2 Matrice de parallélisation par agent

Chaque colonne représente un **agent autonome** (sous-agent) pouvant travailler en parallèle. Les lignes représentent des **slots temporels** (itérations). Une cellule vide signifie que l'agent attend une dépendance.

```
╔════════════╦═══════════════════════╦══════════════════════╦═══════════════════════╗
║   Slot     ║  Agent A              ║  Agent B             ║  Agent C              ║
║            ║  DATA / ML PIPELINE   ║  BACKEND RUST        ║  FRONTEND VUE         ║
╠════════════╬═══════════════════════╬══════════════════════╬═══════════════════════╣
║            ║                       ║                      ║                       ║
║  Slot 1    ║  P0.6  Docker Python  ║  P0.1  CI/CD         ║  P0.3  Tests front    ║
║  FONDATION ║  P0.7  Structure      ║  P0.2  Tests back    ║  P0.5  OpenAPI spec   ║
║            ║       training/       ║  P0.4  Error types   ║                       ║
║            ║  P0.8  Mock ONNX      ║                      ║                       ║
╠════════════╬═══════════════════════╬══════════════════════╬═══════════════════════╣
║            ║                       ║                      ║                       ║
║  Slot 2    ║  P1.1  Quartier test  ║  P4.1  CRUD Projects ║  P5.1  Pinia stores   ║
║  DONNÉES + ║  P1.2  BD TOPO        ║  P4.2  CRUD Scenar.  ║  P5.2  useApi()       ║
║  CRUD      ║  P1.3  LiDAR          ║  P4.3  Simulations   ║  P5.3  Types partagés ║
║            ║  P1.4  Urban Atlas    ║  P4.4  Migrations    ║                       ║
║            ║  P1.5  ERA5/Météo     ║  P4.5  Validation    ║                       ║
╠════════════╬═══════════════════════╬══════════════════════╬═══════════════════════╣
║            ║                       ║                      ║                       ║
║  Slot 3    ║  P1.6  Classif LiDAR  ║  P4.6  OnnxService   ║  P5.4  Page projets   ║
║  GÉOMETRIE ║  P1.7  Fusion géo     ║       (mock .onnx)   ║  P5.5  Dialog projet  ║
║  + ONNX    ║  P1.8  Voxelisation   ║  P4.7  Préprocesseur ║  P5.6  Routing        ║
║  MOCK      ║  P1.9  Props phys.    ║  P4.8  Postprocess.  ║                       ║
║            ║  P1.10 Validation     ║  P4.9  Simulate mock ║                       ║
║            ║  P1.11 Coord mapping  ║                      ║                       ║
╠════════════╬═══════════════════════╬══════════════════════╬═══════════════════════╣
║            ║                       ║                      ║                       ║
║  Slot 4    ║  P2.1  OpenFOAM dock  ║  P4.10 Compression   ║  P5.7  Toolbar        ║
║  CFD SETUP ║  P2.2  Voxel→mesh     ║  P4.11 Cache LRU     ║  P5.8  Surface editor ║
║  + ÉDITEUR ║  P2.3  Case template  ║  P4.12 Geodata API   ║  P5.9  Object placer  ║
║            ║  P2.4  Scénarios LHS  ║  P4.13 WebSocket     ║  P5.10 Sérialisation  ║
╠════════════╬═══════════════════════╬══════════════════════╬═══════════════════════╣
║            ║                       ║                      ║                       ║
║  Slot 5    ║  P2.5  Batch runner   ║  Tests intégration   ║  P5.11 Heatmap 3D     ║
║  CFD RUN + ║  P2.6  CFD parser     ║  back complets +     ║  P5.12 Wind particles ║
║  VISU      ║  P2.7  Dataset build  ║  perf benchmarks     ║  P5.13 Time slider    ║
║            ║  P2.8  Validation     ║  (avec mock ONNX)    ║  P5.14 Comparaison    ║
║            ║                       ║                      ║  P5.15 Légende        ║
╠════════════╬═══════════════════════╬══════════════════════╬═══════════════════════╣
║            ║                       ║                      ║                       ║
║  Slot 6    ║  P3.1  Encoding       ║  Hardening : sécu,   ║  P5.16 Export         ║
║  ML ARCH + ║  P3.2  U-Net 3D       ║  rate limiting,      ║  P5.17 Partage        ║
║  POLISH    ║  P3.3  Loss PINN      ║  docs API,           ║  Tests composants     ║
║            ║  P3.5  DataLoader     ║  logging structuré   ║  + accessibilité      ║
╠════════════╬═══════════════════════╬══════════════════════╬═══════════════════════╣
║            ║                       ║                      ║                       ║
║  Slot 7    ║  P3.6  Train          ║  P7.5  Docker prod   ║  P6.4  Mode dégradé   ║
║  TRAINING  ║  P3.7  HPO            ║  P7.6  Compose prod  ║  (données démo)       ║
║            ║  P3.8  Evaluate       ║  P7.7  Monitoring     ║  Tests E2E skeleton   ║
╠════════════╬═══════════════════════╬══════════════════════╬═══════════════════════╣
║            ║                       ║                      ║                       ║
║  Slot 8    ║  P3.9  Export ONNX    ║                      ║                       ║
║  EXPORT    ║  P3.10 Benchmark      ║       ← attend       ║       ← attend       ║
║            ║  P3.11 Optimize ONNX  ║        P3.9          ║         P6            ║
╠════════════╬═══════════════════════╬══════════════════════╬═══════════════════════╣
║            ║                       ║                      ║                       ║
║  Slot 9    ║  P7.1  Valid. sat.    ║  P6.1  Intégration   ║  P6.2  Test E2E       ║
║  INTÉGRA-  ║  P7.3  Sensibilité   ║        .onnx réel    ║  P6.3  Optim transfer ║
║  TION      ║                      ║                      ║                       ║
╠════════════╬═══════════════════════╬══════════════════════╬═══════════════════════╣
║            ║                       ║                      ║                       ║
║  Slot 10   ║  P7.2  Valid. in-situ ║  Perf tuning final   ║  P7.8  User guide     ║
║  VALIDATION║  P7.4  Rapport sci.   ║                      ║                       ║
╚════════════╩═══════════════════════╩══════════════════════╩═══════════════════════╝
```

**Note** : la matrice passe de 4 agents à **3 agents**. L'ancien "Agent D (DevOps/Docs)" est fusionné : les tâches CI/CD, Docker prod et monitoring sont absorbées par les agents B et C lors de leurs slots creux, ce qui améliore l'utilisation globale.

### 6.3 Résumé des dépendances critiques

| Tâche bloquée | Attend | Raison |
|---------------|--------|--------|
| P0.8 (mock ONNX) | P0.5 (spec tenseur) | Le mock doit respecter les dimensions I/O spécifiées |
| P1.6-P1.11 | P1.1-P1.5 | Besoin des données brutes pour classifier, fusionner et voxeliser |
| P2.2-P2.4 | P1.8 | Le maillage CFD est généré depuis la grille voxel |
| P2.5 | P2.1-P2.4 | Besoin du template de cas et du générateur de scénarios |
| P3.5-P3.8 | P2.7 | Besoin du dataset pour entraîner |
| P3.9 | P3.6-P3.8 | Besoin d'un modèle entraîné pour l'exporter |
| P4.6-P4.9 | P0.5 + P0.8 | Le service ONNX travaille avec le mock dès le Slot 3 |
| P5.8, P5.10 | P1.11 (coord mapping) | Le frontend doit convertir clics CesiumJS ↔ indices voxel |
| P5.11-P5.12 | P5.2, P5.10, P4.9 | Besoin du service API fonctionnel et du format de sérialisation |
| P6.1 | P3.9 | Besoin du vrai modèle ONNX |
| P6.2-P6.3 | P6.1 + P5.11 | Intégration = jonction de tous les flux |
| P7.* | P6.* | Validation = le système fonctionne de bout en bout |

### 6.4 Chemin critique

Le chemin critique (séquence la plus longue) est :

**P0 → P1.1-P1.5 → P1.6-P1.11 → P2.1-P2.4 → P2.5-P2.8 → P3.1-P3.8 → P3.9 → P6 → P7**

Ce chemin est dicté par le **pipeline de données et ML**. C'est lui qui détermine le temps total du projet. Les Agents B et C avancent en parallèle et sont prêts *avant* que le modèle ONNX ne soit disponible — ils travaillent avec le mock ONNX (P0.8) dès le Slot 3.

**Point d'attention** : le goulot d'étranglement est le **Slot 5** (batch runner CFD : ~32h wall-clock sur 32 cœurs). C'est le seul slot qui a une durée incompressible significative. Toutes les autres tâches sont des développements logiciels parallélisables.

### 6.5 Stratégie de mocks pour débloquer les agents

| Agent | Ce qu'il mocke | Quand le mock est remplacé |
|-------|----------------|---------------------------|
| Agent B (Backend) | Fichier `mock.onnx` (P0.8) : 2 couches Conv3D, mêmes dimensions I/O (15 canaux → 4 canaux, grille 256×256×64). Produit des sorties aléatoires mais de dimensions correctes. | Quand P3.9 livre le vrai modèle (Slot 9) |
| Agent C (Frontend) | (a) Réponses API simulées en Slot 1-2 (données aléatoires structurellement correctes). (b) À partir du Slot 3, utilise le vrai backend avec mock ONNX. | (a) remplacé Slot 3 quand P4.9 est fonctionnel. (b) remplacé Slot 9 quand P6.1 intègre le vrai modèle. |

### 6.6 Résumé quantitatif

| Métrique | Valeur |
|----------|--------|
| Nombre total de tâches | **85** |
| Phases | **8** (P0-P7) |
| Agents parallèles | **3** |
| Slots temporels | **10** |
| Tâches sur le chemin critique | **~30** (pipeline data/ML) |
| Tâches parallélisables (hors chemin critique) | **~55** (backend + frontend) |
| Goulot d'étranglement incompressible | Slot 5 : batch CFD (~32h wall-clock) |
