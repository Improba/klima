<template>
  <div v-if="result" class="result-legend">
    <div class="legend-title text-subtitle2 text-white q-mb-sm">Température de surface</div>
    <div class="color-bar" />
    <div class="legend-labels text-caption text-grey-4 q-mt-xs">
      <span>{{ minTemp.toFixed(1) }}°C</span>
      <span>{{ maxTemp.toFixed(1) }}°C</span>
    </div>

    <q-separator dark class="q-my-sm" />

    <div class="stats text-caption text-grey-3">
      <div>T moy: {{ avgTemp.toFixed(1) }}°C</div>
      <div>ΔT max: {{ deltaT.toFixed(1) }}°C</div>
      <div>Vent max: {{ maxWind.toFixed(1) }} m/s</div>
      <div v-if="inferenceTime">Inférence: {{ inferenceTime }}ms</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { SimulationResult } from 'src/types'

const props = defineProps<{
  result: SimulationResult | null
}>()

const minTemp = computed(() => {
  if (!props.result || props.result.surface_temperatures.length === 0) return 0
  return Math.min(...props.result.surface_temperatures.map((t) => t.temperature))
})

const maxTemp = computed(() => {
  if (!props.result || props.result.surface_temperatures.length === 0) return 0
  return Math.max(...props.result.surface_temperatures.map((t) => t.temperature))
})

const avgTemp = computed(() => {
  if (!props.result || props.result.surface_temperatures.length === 0) return 0
  const sum = props.result.surface_temperatures.reduce((s, t) => s + t.temperature, 0)
  return sum / props.result.surface_temperatures.length
})

const deltaT = computed(() => maxTemp.value - minTemp.value)

const maxWind = computed(() => {
  if (!props.result || props.result.wind_field.length === 0) return 0
  return Math.max(
    ...props.result.wind_field.map((w) =>
      Math.sqrt(w.vx * w.vx + w.vy * w.vy + w.vz * w.vz),
    ),
  )
})

const inferenceTime = computed(() => props.result?.metadata.inference_time_ms ?? null)
</script>

<style scoped>
.result-legend {
  position: absolute;
  bottom: 80px;
  left: 16px;
  z-index: 10;
  background: rgba(30, 30, 30, 0.92);
  border-radius: 8px;
  padding: 12px 16px;
  backdrop-filter: blur(8px);
  min-width: 180px;
}

.color-bar {
  height: 12px;
  border-radius: 4px;
  background: linear-gradient(to right, #0000ff, #00ffff, #00ff00, #ffff00, #ff0000);
}

.legend-labels {
  display: flex;
  justify-content: space-between;
}

.stats div {
  line-height: 1.6;
}
</style>
