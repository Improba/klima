import { defineConfig } from '@quasar/app-vite/wrappers'
import { viteStaticCopy } from 'vite-plugin-static-copy'

export default defineConfig(() => {
  return {
    boot: ['pinia', 'cesium'],

    css: ['app.scss'],

    extras: ['roboto-font', 'material-icons'],

    build: {
      target: { browser: ['es2022', 'firefox115', 'chrome115', 'safari14'] },
      vueRouterMode: 'history',

      extendViteConf(viteConf) {
        viteConf.plugins ??= []
        viteConf.plugins.push(
          viteStaticCopy({
            targets: [
              { src: 'node_modules/cesium/Build/Cesium/Workers', dest: 'cesium' },
              { src: 'node_modules/cesium/Build/Cesium/ThirdParty', dest: 'cesium' },
              { src: 'node_modules/cesium/Build/Cesium/Assets', dest: 'cesium' },
              { src: 'node_modules/cesium/Build/Cesium/Widgets', dest: 'cesium' },
            ],
          }),
        )
      },
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
