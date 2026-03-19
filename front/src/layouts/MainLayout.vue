<template>
  <q-layout view="hHh lpR fFf">
    <q-header elevated class="bg-dark">
      <q-toolbar>
        <q-btn flat dense round icon="menu" aria-label="Menu" @click="toggleLeftDrawer" />
        <q-btn
          v-if="isProjectPage"
          flat
          dense
          round
          icon="arrow_back"
          class="q-mr-sm"
          @click="$router.push('/projects')"
        />
        <q-toolbar-title class="text-weight-bold">
          Klima
          <span class="text-caption text-grey-5 q-ml-sm">Simulateur Microclimat Urbain</span>
        </q-toolbar-title>
      </q-toolbar>
    </q-header>

    <q-drawer v-model="leftDrawerOpen" show-if-above bordered class="bg-grey-10">
      <q-list v-if="isProjectPage">
        <q-item-label header class="text-white text-weight-bold q-pt-lg">
          Paramètres de simulation
        </q-item-label>

        <q-item class="q-px-md">
          <q-item-section>
            <q-item-label class="text-grey-4 q-mb-xs">Vitesse du vent (m/s)</q-item-label>
            <q-slider
              v-model="simStore.params.windSpeed"
              :min="0"
              :max="30"
              :step="0.5"
              label
              color="cyan"
            />
          </q-item-section>
        </q-item>

        <q-item class="q-px-md">
          <q-item-section>
            <q-item-label class="text-grey-4 q-mb-xs">Direction du vent (°)</q-item-label>
            <q-slider
              v-model="simStore.params.windDirection"
              :min="0"
              :max="360"
              :step="5"
              label
              color="cyan"
            />
          </q-item-section>
        </q-item>

        <q-item class="q-px-md">
          <q-item-section>
            <q-item-label class="text-grey-4 q-mb-xs">Élévation solaire (°)</q-item-label>
            <q-slider
              v-model="simStore.params.sunElevation"
              :min="0"
              :max="90"
              :step="1"
              label
              color="orange"
            />
          </q-item-section>
        </q-item>

        <q-item class="q-px-md">
          <q-item-section>
            <q-item-label class="text-grey-4 q-mb-xs">Température ambiante (°C)</q-item-label>
            <q-slider
              v-model="simStore.params.tAmbient"
              :min="-10"
              :max="50"
              :step="1"
              label
              color="orange"
            />
          </q-item-section>
        </q-item>

        <q-separator dark class="q-my-md" />

        <q-item class="q-px-md">
          <q-item-section>
            <q-btn
              color="cyan"
              label="Lancer la simulation"
              icon="play_arrow"
              class="full-width"
              unelevated
              :loading="simStore.isSimulating"
              @click="runSimulation"
            />
          </q-item-section>
        </q-item>

        <q-item v-if="simStore.lastResult" class="q-px-md q-mt-sm">
          <q-item-section>
            <div class="text-caption text-grey-5">
              Dernière inférence :
              {{ simStore.lastResult.metadata.inference_time_ms }} ms
            </div>
          </q-item-section>
        </q-item>
      </q-list>
      <q-list v-else>
        <q-item-label header class="text-white text-weight-bold q-pt-lg">
          Klima
        </q-item-label>
        <q-item class="q-px-md">
          <q-item-section>
            <div class="text-body2 text-grey-4">
              Sélectionnez un projet pour commencer la simulation.
            </div>
          </q-item-section>
        </q-item>
      </q-list>
    </q-drawer>

    <q-page-container>
      <router-view />
    </q-page-container>
  </q-layout>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRoute } from 'vue-router'
import { useQuasar } from 'quasar'
import { useSimulationStore } from 'src/stores/simulation'

const $q = useQuasar()
const route = useRoute()
const simStore = useSimulationStore()

const leftDrawerOpen = ref(false)

const isProjectPage = computed(() => {
  return typeof route.params.id === 'string'
})

function toggleLeftDrawer() {
  leftDrawerOpen.value = !leftDrawerOpen.value
}

async function runSimulation() {
  try {
    const projectId = typeof route.params.id === 'string' ? route.params.id : undefined
    await simStore.runSimulation(projectId)
  } catch {
    $q.notify({ type: 'negative', message: 'Erreur lors de la simulation' })
  }
}
</script>
