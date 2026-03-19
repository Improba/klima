<template>
  <q-layout view="hHh lpR fFf">
    <q-header elevated class="bg-dark">
      <q-toolbar>
        <q-btn flat dense round icon="menu" aria-label="Menu" @click="toggleLeftDrawer" />
        <q-toolbar-title class="text-weight-bold">
          Klima
          <span class="text-caption text-grey-5 q-ml-sm">Simulateur Microclimat Urbain</span>
        </q-toolbar-title>
      </q-toolbar>
    </q-header>

    <q-drawer v-model="leftDrawerOpen" show-if-above bordered class="bg-grey-10">
      <q-list>
        <q-item-label header class="text-white text-weight-bold q-pt-lg">
          Paramètres de simulation
        </q-item-label>

        <q-item class="q-px-md">
          <q-item-section>
            <q-item-label class="text-grey-4 q-mb-xs">Vitesse du vent (m/s)</q-item-label>
            <q-slider v-model="windSpeed" :min="0" :max="30" :step="0.5" label color="cyan" />
          </q-item-section>
        </q-item>

        <q-item class="q-px-md">
          <q-item-section>
            <q-item-label class="text-grey-4 q-mb-xs">Direction du vent (°)</q-item-label>
            <q-slider v-model="windDirection" :min="0" :max="360" :step="5" label color="cyan" />
          </q-item-section>
        </q-item>

        <q-item class="q-px-md">
          <q-item-section>
            <q-item-label class="text-grey-4 q-mb-xs">Élévation solaire (°)</q-item-label>
            <q-slider v-model="sunElevation" :min="0" :max="90" :step="1" label color="orange" />
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
              @click="runSimulation"
            />
          </q-item-section>
        </q-item>
      </q-list>
    </q-drawer>

    <q-page-container>
      <router-view
        :wind-speed="windSpeed"
        :wind-direction="windDirection"
        :sun-elevation="sunElevation"
      />
    </q-page-container>
  </q-layout>
</template>

<script setup lang="ts">
import { ref } from 'vue'

const leftDrawerOpen = ref(false)
const windSpeed = ref(5)
const windDirection = ref(180)
const sunElevation = ref(45)

function toggleLeftDrawer() {
  leftDrawerOpen.value = !leftDrawerOpen.value
}

function runSimulation() {
  // TODO: call /api/simulate via store or composable
  console.log('Simulation:', {
    windSpeed: windSpeed.value,
    windDirection: windDirection.value,
    sunElevation: sunElevation.value,
  })
}
</script>
