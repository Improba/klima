/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** Injecté par Docker depuis `CESIUM_ION_TOKEN` (racine) — ne pas dupliquer manuellement. */
  readonly VITE_CESIUM_ION_TOKEN?: string
  readonly VITE_API_BASE_URL?: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}
