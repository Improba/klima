import { createApp } from 'vue'
import { Quasar, Notify, Dialog, Loading } from 'quasar'
import { createPinia } from 'pinia'
import router from './router'

import '@quasar/extras/roboto-font/roboto-font.css'
import '@quasar/extras/material-icons/material-icons.css'
import 'quasar/src/css/index.sass'
import './css/app.scss'

import App from './App.vue'

const app = createApp(App)

app.use(Quasar, {
  plugins: { Notify, Dialog, Loading },
  config: { dark: 'auto' },
})

app.use(createPinia())
app.use(router)

app.mount('#q-app')
