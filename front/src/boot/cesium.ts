import { defineBoot } from '@quasar/app-vite/wrappers'
import { Ion, buildModuleUrl } from 'cesium'

export default defineBoot(() => {
  ;(window as Record<string, unknown>).CESIUM_BASE_URL = '/cesium'
  buildModuleUrl.setBaseUrl('/cesium/')

  const token = import.meta.env.VITE_CESIUM_ION_TOKEN?.trim() ?? ''
  Ion.defaultAccessToken = token.length > 0 ? token : 'none'
})
