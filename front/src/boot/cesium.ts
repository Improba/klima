import { defineBoot } from '@quasar/app-vite/wrappers'
import { Ion, buildModuleUrl } from 'cesium'

export default defineBoot(() => {
  ;(window as Record<string, unknown>).CESIUM_BASE_URL = '/cesium'
  buildModuleUrl.setBaseUrl('/cesium/')

  // Une seule valeur à configurer : CESIUM_ION_TOKEN (racine). Compose la mappe en VITE_* pour Vite.
  const token = import.meta.env.VITE_CESIUM_ION_TOKEN?.trim() ?? ''
  Ion.defaultAccessToken = token
})
