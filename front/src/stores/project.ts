import { ref } from 'vue'
import { defineStore } from 'pinia'
import type { Project } from 'src/types'
import { useApi } from 'src/composables/useApi'

export const useProjectStore = defineStore('project', () => {
  const projects = ref<Project[]>([])
  const currentProject = ref<Project | null>(null)
  const api = useApi()

  async function fetchProjects() {
    projects.value = await api.getProjects()
  }

  async function fetchProject(id: string) {
    currentProject.value = await api.getProject(id)
  }

  async function create(name: string, description?: string) {
    const project = await api.createProject(name, description)
    projects.value.unshift(project)
    return project
  }

  async function update(id: string, name: string, description?: string) {
    const project = await api.updateProject(id, name, description)
    const idx = projects.value.findIndex((p) => p.id === id)
    if (idx !== -1) projects.value[idx] = project
    if (currentProject.value?.id === id) currentProject.value = project
    return project
  }

  async function remove(id: string) {
    await api.deleteProject(id)
    projects.value = projects.value.filter((p) => p.id !== id)
    if (currentProject.value?.id === id) currentProject.value = null
  }

  return {
    projects,
    currentProject,
    loading: api.loading,
    error: api.error,
    fetchProjects,
    fetchProject,
    create,
    update,
    remove,
  }
})
