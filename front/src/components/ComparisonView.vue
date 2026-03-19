<template>
  <div v-if="showComparison" class="comparison-container">
    <div class="comparison-toolbar">
      <q-icon name="compare" color="white" size="sm" />
      <span class="text-caption text-grey-4 q-ml-sm">Avant / Après</span>
      <q-slider
        v-model="splitPosition"
        :min="0"
        :max="100"
        color="white"
        class="comparison-slider q-ml-md"
      />
      <q-btn
        flat
        dense
        round
        icon="close"
        color="grey-4"
        size="sm"
        @click="emit('close')"
      />
    </div>
    <div class="split-line" :style="{ left: splitPosition + '%' }" />
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

defineProps<{
  showComparison: boolean
}>()

const emit = defineEmits<{
  close: []
  'update:splitPosition': [position: number]
}>()

const splitPosition = ref(50)
</script>

<style scoped>
.comparison-container {
  position: absolute;
  top: 16px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 10;
}

.comparison-toolbar {
  background: rgba(30, 30, 30, 0.92);
  border-radius: 8px;
  padding: 8px 16px;
  backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  min-width: 300px;
}

.comparison-slider {
  flex: 1;
}

.split-line {
  position: fixed;
  top: 0;
  bottom: 0;
  width: 2px;
  background: white;
  pointer-events: none;
  box-shadow: 0 0 6px rgba(255, 255, 255, 0.5);
}
</style>
