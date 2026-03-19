import { defineBoot } from '@quasar/app-vite/wrappers'
import { Ion } from 'cesium'

export default defineBoot(() => {
  Ion.defaultAccessToken = import.meta.env.VITE_CESIUM_ION_TOKEN ?? ''
})
