import { defineBoot } from '@quasar/app-vite/wrappers'
import { createPinia } from 'pinia'

export default defineBoot(({ app }) => {
  const pinia = createPinia()
  app.use(pinia)
})
