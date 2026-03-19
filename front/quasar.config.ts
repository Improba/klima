import { configure } from 'quasar/wrappers'
import cesium from 'vite-plugin-cesium'

export default configure(() => {
  return {
    boot: ['cesium'],

    css: ['app.scss'],

    extras: ['roboto-font', 'material-icons'],

    build: {
      target: { browser: ['es2022', 'firefox115', 'chrome115', 'safari14'] },
      vueRouterMode: 'history',
      vitePlugins: [cesium()],
    },

    devServer: {
      open: false,
      port: 9000,
      proxy: {
        '/api': {
          target: 'http://klima-back:3000',
          changeOrigin: true,
        },
      },
    },

    framework: {
      config: {
        dark: 'auto',
      },
      plugins: ['Notify', 'Dialog', 'Loading'],
    },
  }
})
