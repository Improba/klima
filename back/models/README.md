# Modèles ONNX (local)

Fichiers attendus par le backend si les variables d’environnement pointent ici (voir `back/docker/docker-compose.dev.yml`) :

| Fichier | Rôle |
|---------|------|
| `klima.onnx` | Graphe ONNX (entrée `[batch, 15, nx, ny, nz]`, sortie 4 canaux) |
| `norm_params.json` | Moyennes / écarts-types des canaux (`input_*`, `output_*`) |

Ces fichiers sont **ignorés par Git** (`.gitignore`).

## Limite actuelle — Local FNO + ONNX

L’opérateur PyTorch `fft_rfftn` utilisé dans le FNO **n’est pas exportable** vers ONNX avec l’exporteur officiel (`UnsupportedOperatorError` sur `aten::fft_rfftn`). Tant qu’un chemin d’export FFT→ONNX n’existe pas, un graphe **mock** (petit MLP convolutif aux mêmes dimensions I/O) peut servir à valider la chaîne API + ORT + normalisation.

Pour une inférence **avec les poids entraînés**, il faut exécuter le modèle en **PyTorch** (hors ONNX), ou mettre en place un opérateur ONNX personnalisé / autre runtime.
