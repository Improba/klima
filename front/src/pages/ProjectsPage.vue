<template>
  <q-page class="q-pa-lg" style="overflow-y: auto;">
    <div class="row items-center q-mb-lg">
      <div class="text-h4 text-white text-weight-bold">Projets</div>
      <q-space />
      <q-btn
        color="cyan"
        icon="add"
        label="Nouveau projet"
        unelevated
        @click="showDialog = true"
      />
    </div>

    <q-spinner-dots v-if="store.loading" color="cyan" size="3em" class="block q-mx-auto q-mt-xl" />

    <div v-else-if="store.projects.length === 0" class="text-center q-mt-xl">
      <q-icon name="folder_open" size="4em" color="grey-7" />
      <p class="text-grey-5 text-h6 q-mt-md">Aucun projet pour le moment</p>
      <q-btn
        color="cyan"
        label="Créer votre premier projet"
        unelevated
        @click="showDialog = true"
      />
    </div>

    <div v-else class="row q-col-gutter-md">
      <div
        v-for="project in store.projects"
        :key="project.id"
        class="col-12 col-sm-6 col-md-4 col-lg-3"
      >
        <q-card
          class="bg-grey-10 cursor-pointer full-height project-card"
          @click="$router.push(`/projects/${project.id}`)"
        >
          <q-card-section>
            <div class="text-h6 text-white">{{ project.name }}</div>
            <div class="text-caption text-grey-5 q-mt-xs">
              {{ formatDate(project.created_at) }}
            </div>
          </q-card-section>
          <q-card-section v-if="project.description" class="q-pt-none">
            <div class="text-body2 text-grey-4 ellipsis-2-lines">
              {{ project.description }}
            </div>
          </q-card-section>
          <q-card-actions align="right">
            <q-btn
              flat
              dense
              icon="delete"
              color="negative"
              @click.stop="confirmDelete(project)"
            />
          </q-card-actions>
        </q-card>
      </div>
    </div>

    <ProjectDialog v-model="showDialog" @created="onProjectCreated" />
  </q-page>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useQuasar } from 'quasar'
import { useProjectStore } from 'src/stores/project'
import type { Project } from 'src/types'
import ProjectDialog from 'components/ProjectDialog.vue'

const $q = useQuasar()
const store = useProjectStore()
const showDialog = ref(false)

onMounted(() => {
  store.fetchProjects()
})

function formatDate(iso: string) {
  return new Date(iso).toLocaleDateString('fr-FR', {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
  })
}

function onProjectCreated() {
  showDialog.value = false
}

function confirmDelete(project: Project) {
  $q.dialog({
    title: 'Supprimer le projet',
    message: `Voulez-vous supprimer « ${project.name} » ?`,
    cancel: true,
    persistent: true,
    color: 'negative',
  }).onOk(async () => {
    try {
      await store.remove(project.id)
      $q.notify({ type: 'positive', message: 'Projet supprimé' })
    } catch {
      $q.notify({ type: 'negative', message: 'Erreur lors de la suppression' })
    }
  })
}
</script>

<style scoped lang="scss">
.project-card {
  transition: transform 0.2s, box-shadow 0.2s;
  &:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 20px rgba(0, 188, 212, 0.15);
  }
}
</style>
