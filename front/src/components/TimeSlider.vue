<template>
  <div class="time-slider-container">
    <q-icon name="wb_sunny" color="orange" size="sm" />
    <q-slider
      v-model="hour"
      :min="6"
      :max="22"
      :step="0.5"
      label
      :label-value="formatHour(hour)"
      color="orange"
      class="time-slider"
      @update:model-value="onHourChange"
    />
    <span class="text-caption text-grey-4 time-label">{{ formatHour(hour) }}</span>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'

defineOptions({ name: 'TimeSlider' })

const props = defineProps<{
  modelValue?: number
}>()

const emit = defineEmits<{
  'update:hour': [hour: number]
  'update:modelValue': [hour: number]
  'update:sunElevation': [elevation: number]
}>()

const hour = ref(props.modelValue ?? 13)

function formatHour(h: number): string {
  const hours = Math.floor(h)
  const minutes = (h - hours) * 60
  return `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}`
}

const sunElevation = computed(() => {
  const solarNoon = 13
  const maxElevation = 65
  const dayLength = 16
  const normalized = ((hour.value - solarNoon) / (dayLength / 2)) * Math.PI
  return Math.max(0, maxElevation * Math.cos(normalized))
})

function onHourChange(val: number | null) {
  if (val === null) return
  emit('update:hour', val)
  emit('update:modelValue', val)
  emit('update:sunElevation', sunElevation.value)
}
</script>

<style scoped>
.time-slider-container {
  position: absolute;
  bottom: 24px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 10;
  background: rgba(30, 30, 30, 0.92);
  border-radius: 8px;
  padding: 10px 20px;
  backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 320px;
  max-width: 480px;
  width: 40vw;
}

.time-slider {
  flex: 1;
}

.time-label {
  min-width: 44px;
  text-align: right;
}
</style>
