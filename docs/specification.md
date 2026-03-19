# Spécification Projet : Simulateur IA "Surrogate" de Microclimat Urbain 3D

## 1. Vision et Objectif du Projet

Développer une application web 3D interactive permettant aux urbanistes et architectes de simuler en temps réel l'impact thermique (îlots de chaleur, flux d'air) des aménagements urbains.

Le système remplace les calculs CFD (Mécanique des Fluides Numérique) traditionnels, lents et coûteux, par un modèle d'Intelligence Artificielle de substitution (*Surrogate Model*) pré-entraîné, capable de générer des prédictions en quelques millisecondes.

## 2. Architecture Technique (La "Stack")

| Couche | Technologie |
|--------|-------------|
| Entraînement IA & Préparation des Données | Python (PyTorch, GeoPandas, Trimesh) |
| Format d'Échange Modèle | ONNX (Open Neural Network Exchange) |
| Backend (API & Inférence) | Rust — Axum + `ort` (ONNX Runtime) |
| Frontend (Interface & Visualisation 3D) | Vue.js 3 + Quasar + CesiumJS |
| Base de données | SQLite (local, dans le backend) |

---

## 3. Étapes Logiques de Développement

### Phase 1 : R&D et Préparation des Données (Python)

L'objectif est de constituer le jeu de données qui servira de "vérité terrain" pour apprendre à l'IA comment l'air et la chaleur se comportent autour des bâtiments.

1. **Extraction Géométrique :** Scraper ou télécharger un quartier test (ex: un arrondissement dense de Paris ou Lyon) en récupérant l'emprise au sol et la hauteur des bâtiments. Voxeliser cet espace 3D (le transformer en une grille 3D de cubes).
2. **Génération des Scénarios de Base :** Utiliser un logiciel de simulation thermique/CFD open-source (comme OpenFOAM ou Ladybug/Honeybee) pour faire tourner quelques centaines de simulations sur ce quartier (en faisant varier la vitesse du vent, l'angle du soleil, et l'albédo des toits).
3. **Constitution du Dataset :** Sauvegarder les paires `[Entrée : Matrice 3D du quartier + Météo] -> [Sortie : Matrice 3D des températures et flux d'air]`.

### Phase 2 : Entraînement du Modèle "Surrogate" (Python -> ONNX)

1. **Architecture du Réseau :** Développer un réseau de neurones en Python avec PyTorch (typiquement une architecture de type U-Net 3D ou un Graph Neural Network). Fonction de perte avec contrainte physique (PINN) :

$$Loss = \lambda_1 MSE(T_{pred}, T_{vrai}) + \lambda_2 \left\| \nabla \cdot \mathbf{v}_{pred} \right\|^2$$

2. **Entraînement :** Faire converger le modèle pour qu'il comprenne que "le vent contourne les obstacles" et que "l'asphalte au soleil chauffe plus que l'herbe à l'ombre".
3. **Exportation :** Une fois le modèle précis et léger, l'exporter au format `.onnx`.

### Phase 3 : Le Backend Haute Performance (Rust + Axum)

1. **Initialisation :** Serveur API Axum.
2. **Intégration ONNX :** Crate `ort` pour charger le `.onnx` au démarrage.
3. **Endpoints :** `POST /api/simulate` — reçoit la géométrie modifiée, renvoie les résultats d'inférence.
4. **Stockage :** SQLite pour projets, scénarios et résultats de simulation.

### Phase 4 : Frontend 3D Interactif (Vue.js + Quasar + CesiumJS)

1. **Structure UI (Quasar) :** Menus latéraux, sliders, outils de dessin.
2. **Intégration CesiumJS :** Carte 3D avec bâtiments via 3D Tiles / OSM Buildings.
3. **Interactivité :** Clic pour modifier le sol (bitume → parc) ou ajouter un arbre.
4. **Visualisation des Résultats :** Coloration thermique des façades, particules de vent.

---

## 4. Sources de Données

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
