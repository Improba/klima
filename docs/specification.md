# Spécification Projet : Simulateur IA "Surrogate" de Microclimat Urbain 3D

## 1. Vision et Objectif du Projet

Développer une application web 3D interactive permettant aux urbanistes et architectes de simuler en temps réel l'impact thermique (îlots de chaleur, flux d'air) des aménagements urbains.

Le système remplace les calculs CFD (Mécanique des Fluides Numérique) traditionnels, lents et coûteux, par un modèle d'Intelligence Artificielle de substitution (*Surrogate Model*) pré-entraîné, capable de générer des prédictions en quelques millisecondes.

## 2. Architecture Technique (La "Stack")

| Couche | Technologie |
|--------|-------------|
| Entraînement IA & Préparation des Données | Python (PyTorch, NVIDIA Modulus / NeuralOperator, GeoPandas, Trimesh) |
| Architecture du modèle IA | **Fourier Neural Operator (FNO)** — Local-FNO avec contraintes PINN |
| Format d'Échange Modèle | ONNX (Open Neural Network Exchange) |
| Backend (API & Inférence) | Rust — Axum + `ort` (ONNX Runtime) |
| Frontend (Interface & Visualisation 3D) | Vue.js 3 + Quasar + CesiumJS |
| Base de données | **PostgreSQL 16** (conteneurisée dans Docker) |

---

## 3. Concept scientifique : le Fourier Neural Operator (FNO) Localisé

### 3.1 Pourquoi le FNO et pas un CNN 3D classique ?

Les approches classiques (U-Net 3D, CNN) apprennent sur une grille de pixels à une résolution fixe. Si le modèle est entraîné à 2 m de résolution, il ne peut pas prédire à 50 cm sans ré-entraînement. Il est **dépendant de la résolution**.

Le **Fourier Neural Operator** (Li et al. 2020, *Fourier Neural Operator for Parametric Partial Differential Equations*) résout ce problème fondamental. Au lieu d'apprendre des filtres convolutifs dans le domaine spatial (les pixels), le FNO :

1. Applique une **Transformée de Fourier Rapide (FFT)** pour passer dans le domaine des fréquences.
2. Apprend un **opérateur paramétré** dans l'espace spectral — c'est-à-dire qu'il apprend les *lois physiques* qui gouvernent le phénomène, pas les valeurs sur une grille.
3. Applique une **FFT inverse** pour revenir dans le domaine spatial, à la résolution demandée.

### 3.2 Avantages pour Klima

| Propriété | Impact projet |
|-----------|---------------|
| **Zero-shot Super Resolution** | Le backend Rust envoie une grille basse résolution au modèle ONNX pour un calcul rapide. Si l'utilisateur zoome dans CesiumJS, le FNO peut extrapoler les détails (tourbillons, gradients thermiques) à une résolution plus fine sans ré-entraînement. |
| **Entraînement data-efficient** | Le Local-FNO (2025) a démontré la prédiction de champs 3D de vent et température sur un quartier de 1.2 km avec **seulement 24 simulations CFD**. Gain monumental par rapport aux milliers de simulations requises par un CNN. |
| **Accélération ×500** | Génération d'un champ de vent 3D complet en moins d'une minute (vs heures pour la CFD). Le backend Rust accélère encore via ONNX Runtime. |
| **Généralisation physique** | Apprend les lois dans l'espace des fréquences → meilleure extrapolation sur des géométries non vues. |

### 3.3 Le Local-FNO

Le FNO standard apprend des patterns globaux sur tout le domaine. Le **Local-FNO** ajoute une composante locale (via des couches de convolution ou d'attention) pour capturer les détails fins (sillage derrière un bâtiment, turbulence dans une rue). L'architecture combine :

- **Couches Fourier globales** : capturent les gradients de pression et les mouvements d'air à grande échelle.
- **Couches locales (Conv3D ou attention)** : capturent les interactions bâtiment-vent à l'échelle du mètre.

---

## 4. Concept : Pipeline "GeoJSON-to-Tensor" PINN

### 4.1 Le pont entre le dessin de l'utilisateur et la physique

Plusieurs travaux récents (fin 2025 / début 2026, framework *Smart Urban Cooling*) introduisent une mécanique élégante : le **pont direct** entre le format web (ce que dessine l'utilisateur) et les mathématiques de la physique.

**Flux utilisateur** :
1. L'utilisateur dessine un polygone sur CesiumJS (ex: "Je mets un toit végétalisé ici").
2. Ce dessin génère un **GeoJSON** standard.
3. Le backend convertit instantanément ce GeoJSON en un **tenseur d'émissivité et d'albédo** (pas de maillage 3D complexe intermédiaire).
4. Le FNO-PINN intègre ce tenseur et prédit le nouveau champ thermique + vent.

### 4.2 Contrainte physique dans la Loss (PINN)

Le réseau de neurones intègre directement l'**équation de diffusion de la chaleur** dans sa fonction de perte :

$$\mathcal{L}_{diffusion} = \left\| \frac{\partial T}{\partial t} - \alpha \nabla^2 T \right\|^2$$

où α est la diffusivité thermique du matériau. Combinée aux contraintes de Navier-Stokes simplifiées (divergence, no-slip), la loss totale devient :

$$\mathcal{L}_{total} = \lambda_1 \mathcal{L}_{data} + \lambda_2 \mathcal{L}_{divergence} + \lambda_3 \mathcal{L}_{no\text{-}slip} + \lambda_4 \mathcal{L}_{diffusion}$$

**Résultat visuel** : l'utilisateur voit l'îlot de fraîcheur "couler" depuis le toit végétalisé vers la rue en contrebas au fil des heures de la journée, de manière physiquement cohérente, généré en temps réel.

---

## 5. Étapes Logiques de Développement

### Phase 1 : R&D et Préparation des Données (Python)

1. **Extraction Géométrique :** Télécharger un quartier test (ex: Paris 11e) via BD TOPO IGN + LiDAR HD. Voxeliser l'espace 3D (grille 256×256×64 à 2 m).
2. **Génération des Scénarios :** Lancer **24 à 50 simulations CFD** via OpenFOAM (au lieu de centaines grâce à l'efficacité du FNO). Varier : vent, soleil, albédo.
3. **Constitution du Dataset :** Paires `[Entrée : Grille 3D + Météo] → [Sortie : Champs ΔT et v]`.

### Phase 2 : Entraînement du FNO-PINN (Python → ONNX)

1. **Architecture :** Local-FNO avec couches Fourier spectrales + couches Conv3D locales. Implémentation via **NVIDIA Modulus** ou la librairie **NeuralOperator** (PyTorch).
2. **Loss PINN :** Combiner erreur data-fidelity + contraintes physiques (diffusion thermique, divergence, no-slip).
3. **Entraînement :** 24-50 simulations suffisent grâce à la régularisation physique.
4. **Export ONNX :** Le FNO s'exporte en `.onnx` comme n'importe quel modèle PyTorch. Le backend Rust n'a pas besoin de savoir que c'est un opérateur de Fourier — il passe des tenseurs et récupère des résultats.

### Phase 3 : Le Backend Haute Performance (Rust + Axum + PostgreSQL)

1. **API Axum** avec base de données **PostgreSQL** (projets, scénarios, simulations).
2. **Pipeline GeoJSON-to-Tensor** : conversion instantanée des polygones GeoJSON en tenseurs d'entrée.
3. **Inférence ONNX** via `ort` : le FNO prédit ΔT et v en < 200 ms.
4. **Super-résolution à la volée** : si l'utilisateur demande un zoom, le FNO évalue sur une grille plus fine sans ré-entraînement.

### Phase 4 : Frontend 3D Interactif (Vue.js + Quasar + CesiumJS)

1. **Outils de dessin GeoJSON** : l'utilisateur dessine des polygones (toit végétalisé, parc, arbres) directement sur la carte 3D.
2. **Visualisation thermique** : heatmap 3D des surfaces + animation de particules de vent.
3. **Slider temporel** : voir la diffusion de chaleur au fil de la journée (exploite la composante temporelle du PINN).
4. **Zero-shot zoom** : quand l'utilisateur zoome, le frontend demande au backend une inférence à résolution plus fine.

---

## 6. Sources de Données

### Géométrie 3D

| Source | Description |
|--------|-------------|
| LiDAR HD IGN (France) | Nuages de points, 10 pts/m² |
| BD TOPO IGN | Bâtiments 3D (shapefile / CityGML) |

### Vérité Terrain Thermique

| Source | Description |
|--------|-------------|
| Landsat 8 / Sentinel-3 (Copernicus) | Imagerie thermique (LST) |
| Urban Atlas (Copernicus) | Occupation des sols, coefficients d'albédo |

### Météo et Climat

| Source | Description |
|--------|-------------|
| API Météo-France | Données synoptiques horaires |
| ERA5 (ECMWF) | Réanalyse climatique, scénarios de canicule |

---

## 7. Références Scientifiques

- Li, Z. et al. (2020). *Fourier Neural Operator for Parametric Partial Differential Equations*. arXiv:2010.08895.
- Local-FNO (2025). Prédiction 3D vent + température sur quartier 1.2 km avec 24 simulations.
- Raissi, M. et al. (2019). *Physics-Informed Neural Networks*. Journal of Computational Physics.
- Franke, J. et al. (2007). *Best practice guideline for the CFD simulation of flows in the urban environment*. COST 732.
- Grimmond, C.S.B. & Oke, T.R. (1999). *Aerodynamic properties of urban areas derived from analysis of surface form*. J. Applied Meteorology.
- Oke, T.R. et al. (2017). *Urban Climates*. Cambridge University Press.
