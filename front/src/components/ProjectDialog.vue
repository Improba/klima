<template>
  <q-dialog :model-value="modelValue" @update:model-value="$emit('update:modelValue', $event)">
    <q-card class="bg-grey-10" style="min-width: 400px">
      <q-card-section>
        <div class="text-h6 text-white">Nouveau projet</div>
      </q-card-section>

      <q-card-section class="q-pt-none">
        <q-form @submit.prevent="save">
          <q-input
            v-model="name"
            label="Nom du projet"
            dark
            filled
            :rules="[(v: string) => !!v?.trim() || 'Le nom est requis']"
            lazy-rules
            class="q-mb-md"
          />
          <q-input
            v-model="description"
            label="Description (optionnel)"
            dark
            filled
            type="textarea"
            autogrow
          />
          <q-card-actions align="right" class="q-px-none q-pb-none q-pt-md">
            <q-btn flat label="Annuler" color="grey" v-close-popup type="button" />
            <q-btn
              unelevated
              label="Créer"
              color="cyan"
              type="submit"
              :loading="saving"
              :disable="!name.trim()"
            />
          </q-card-actions>
        </q-form>
      </q-card-section>
    </q-card>
  </q-dialog>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useQuasar } from 'quasar'
import { useProjectStore } from 'src/stores/project'

defineProps<{ modelValue: boolean }>()
const emit = defineEmits<{
  (e: 'update:modelValue', val: boolean): void
  (e: 'created'): void
}>()

const $q = useQuasar()
const store = useProjectStore()

const name = ref('')
const description = ref('')
const saving = ref(false)

async function save() {
  if (!name.value.trim()) return
  saving.value = true
  try {
    await store.create(name.value.trim(), description.value.trim() || undefined)
    $q.notify({ type: 'positive', message: 'Projet créé' })
    emit('created')
    emit('update:modelValue', false)
    name.value = ''
    description.value = ''
  } catch (err) {
    const detail = err instanceof Error ? err.message : String(err)
    $q.notify({
      type: 'negative',
      message: 'Erreur lors de la création',
      caption: detail.length > 120 ? `${detail.slice(0, 120)}…` : detail,
    })
  } finally {
    saving.value = false
  }
}
</script>
