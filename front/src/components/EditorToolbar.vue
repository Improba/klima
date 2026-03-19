<template>
  <div class="editor-toolbar">
    <q-btn-group flat>
      <q-btn
        v-for="tool in tools"
        :key="tool.id"
        :icon="tool.icon"
        :color="activeTool === tool.id ? 'cyan' : 'grey-5'"
        :class="{ 'bg-grey-8': activeTool === tool.id }"
        :disable="tool.disabled"
        flat
        dense
        @click="selectTool(tool.id)"
      >
        <q-tooltip>{{ tool.disabled ? `${tool.label} (bientôt)` : tool.label }}</q-tooltip>
      </q-btn>
    </q-btn-group>

    <q-select
      v-if="activeTool === 'brush'"
      v-model="surfaceType"
      :options="surfaceTypes"
      option-label="label"
      option-value="value"
      emit-value
      map-options
      dense
      dark
      filled
      class="surface-select q-mt-sm"
      @update:model-value="onSurfaceTypeChange"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import type { SurfaceType } from 'src/types'

interface ToolDef {
  id: string
  icon: string
  label: string
  /** Non branché sur la carte — bouton grisé. */
  disabled?: boolean
}

const emit = defineEmits<{
  'tool-change': [toolId: string]
  'surface-type-change': [type: SurfaceType]
}>()

const tools: ToolDef[] = [
  { id: 'select', icon: 'near_me', label: 'Sélectionner' },
  { id: 'brush', icon: 'format_paint', label: 'Surface' },
  { id: 'object', icon: 'park', label: 'Objet', disabled: true },
  { id: 'eraser', icon: 'delete', label: 'Gomme', disabled: true },
]

const surfaceTypes: { label: string; value: SurfaceType }[] = [
  { label: 'Herbe', value: 'herbe' },
  { label: 'Bitume', value: 'bitume' },
  { label: 'Eau', value: 'eau' },
  { label: 'Gravier', value: 'gravier' },
  { label: 'Végétation', value: 'vegetation' },
  { label: 'Bâtiment', value: 'batiment' },
]

const activeTool = ref<string>('select')
const surfaceType = ref<SurfaceType>('herbe')

function selectTool(toolId: string) {
  if (tools.some((t) => t.id === toolId && t.disabled)) return
  activeTool.value = toolId
}

function onSurfaceTypeChange(val: SurfaceType) {
  emit('surface-type-change', val)
}

watch(activeTool, (val) => {
  emit('tool-change', val)
})
</script>

<style scoped>
.editor-toolbar {
  position: absolute;
  top: 16px;
  right: 16px;
  z-index: 10;
  background: rgba(30, 30, 30, 0.92);
  border-radius: 8px;
  padding: 8px;
  backdrop-filter: blur(8px);
  display: flex;
  flex-direction: column;
  align-items: stretch;
  min-width: 48px;
}

.surface-select {
  min-width: 140px;
}
</style>
