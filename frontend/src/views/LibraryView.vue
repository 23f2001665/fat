<script setup>
import { computed, onMounted, ref, watch } from 'vue'
import { toast } from 'vue3-toastify'
import EmptyState from '@/components/EmptyState.vue'
import ExerciseModal from '@/components/ExerciseModal.vue'
import Icon from '@/components/Icon.vue'
import LibraryItemModal from '@/components/LibraryItemModal.vue'
import PageHeader from '@/components/PageHeader.vue'
import SearchField from '@/components/SearchField.vue'
import StatCard from '@/components/StatCard.vue'
import ToggleSwitch from '@/components/ToggleSwitch.vue'
import {
  getEquipment,
  getExercises,
  getMuscleGroups,
  setEquipmentActive,
  setExerciseActive,
  setMuscleGroupActive,
} from '@/api/library'
import { isAdmin } from '@/stores/auth'
import { pluralize } from '@/utils/format'

const tabs = [
  { key: 'exercises', label: 'Exercises', icon: 'dumbbell' },
  { key: 'muscles', label: 'Muscle groups', icon: 'target' },
  { key: 'equipment', label: 'Equipment', icon: 'library' },
]

const activeTab = ref('exercises')
const search = ref('')
const showInactive = ref(true)
const loading = ref(true)
const errorMessage = ref('')
const exercises = ref([])
const muscleGroups = ref([])
const equipment = ref([])
const togglingKey = ref('')
const exerciseModalOpen = ref(false)
const editingExercise = ref(null)
const itemModalOpen = ref(false)
const itemModalKind = ref('muscle')
const editingItem = ref(null)

const currentTab = computed(() => tabs.find((tab) => tab.key === activeTab.value))

const pageDescription = computed(() =>
  isAdmin.value
    ? 'Manage the exercises, muscle groups, and equipment that power logging and analytics. Historical records always keep their references.'
    : 'Browse the shared exercise catalogue used for logging and analytics. Only administrators can change these reference records.',
)

const muscleUsage = computed(() => {
  const usage = new Map()
  for (const exercise of exercises.value) {
    for (const assignment of exercise.muscles) {
      usage.set(assignment.muscle_group_id, (usage.get(assignment.muscle_group_id) || 0) + 1)
    }
  }
  return usage
})

const counts = computed(() => ({
  exercises: {
    total: exercises.value.length,
    active: exercises.value.filter((row) => row.is_active).length,
  },
  muscles: {
    total: muscleGroups.value.length,
    active: muscleGroups.value.filter((row) => row.is_active).length,
  },
  equipment: {
    total: equipment.value.length,
    active: equipment.value.filter((row) => row.is_active).length,
  },
}))

const inactiveCount = computed(
  () =>
    exercises.value.filter((row) => !row.is_active).length +
    muscleGroups.value.filter((row) => !row.is_active).length +
    equipment.value.filter((row) => !row.is_active).length,
)

const tabCounts = computed(() => ({
  exercises: counts.value.exercises.total,
  muscles: counts.value.muscles.total,
  equipment: counts.value.equipment.total,
}))

function matchesSearch(...parts) {
  const needle = search.value.trim().toLowerCase()
  if (!needle) return true
  return parts.some((part) => String(part || '').toLowerCase().includes(needle))
}

const filteredExercises = computed(() =>
  exercises.value.filter((row) => {
    if (!showInactive.value && !row.is_active) return false
    return matchesSearch(
      row.name,
      row.category,
      row.description,
      row.muscle_summary,
      ...row.muscles.map((muscle) => muscle.muscle_group_name),
    )
  }),
)

const filteredMuscles = computed(() =>
  muscleGroups.value.filter((row) => {
    if (!showInactive.value && !row.is_active) return false
    return matchesSearch(row.name, row.description)
  }),
)

const filteredEquipment = computed(() =>
  equipment.value.filter((row) => {
    if (!showInactive.value && !row.is_active) return false
    return matchesSearch(row.name, row.category)
  }),
)

const visibleCount = computed(() => {
  if (activeTab.value === 'exercises') return filteredExercises.value.length
  if (activeTab.value === 'muscles') return filteredMuscles.value.length
  return filteredEquipment.value.length
})

const totalInTab = computed(() => tabCounts.value[activeTab.value] || 0)

const panelCopy = computed(() => {
  if (activeTab.value === 'exercises') {
    return {
      title: 'Exercise catalogue',
      description:
        'Movements available when logging sets. Primary muscles receive full volume attribution and secondary muscles half.',
      addLabel: 'New exercise',
      searchPlaceholder: 'Search exercises, categories, muscles…',
    }
  }
  if (activeTab.value === 'muscles') {
    return {
      title: 'Muscle groups',
      description:
        'The attribution targets used by exercises and analytics. A group used by an exercise cannot be deactivated.',
      addLabel: 'New muscle group',
      searchPlaceholder: 'Search muscle groups…',
    }
  }
  return {
    title: 'Equipment',
    description:
      'Tools available for sets. No Equipment is reserved, always active, and cannot be removed.',
    addLabel: 'New equipment',
    searchPlaceholder: 'Search equipment or category…',
  }
})

const emptyTitle = computed(() => {
  if (search.value.trim()) return 'No matching library items'
  if (activeTab.value === 'exercises') return 'No exercises to show'
  if (activeTab.value === 'muscles') return 'No muscle groups to show'
  return 'No equipment to show'
})

const emptyMessage = computed(() => {
  if (search.value.trim()) {
    return 'Try a different search, or clear it to see the rest of the catalogue.'
  }
  if (!showInactive.value) {
    return 'Everything in this section is inactive. Turn on “Show inactive” to see it.'
  }
  if (!isAdmin.value) {
    return 'An administrator manages this shared catalogue.'
  }
  return 'Create the first item in this section to make it available while logging workouts.'
})

function isToggling(kind, id) {
  return togglingKey.value === `${kind}-${id}`
}

async function loadAll({ silent = false } = {}) {
  if (!silent) loading.value = true
  errorMessage.value = ''
  try {
    const [exerciseRows, muscleRows, equipmentRows] = await Promise.all([
      getExercises({ includeInactive: true }),
      getMuscleGroups({ includeInactive: true }),
      getEquipment({ includeInactive: true }),
    ])
    exercises.value = exerciseRows
    muscleGroups.value = muscleRows
    equipment.value = equipmentRows
  } catch (error) {
    errorMessage.value = error.userMessage || 'Could not load the library catalogue.'
  } finally {
    loading.value = false
  }
}

function replaceRow(rows, saved) {
  const index = rows.value.findIndex((row) => row.id === saved.id)
  if (index >= 0) rows.value.splice(index, 1, saved)
  else rows.value.push(saved)
}

async function toggle(kind, row, isActive) {
  if (!isAdmin.value) return
  const key = `${kind}-${row.id}`
  if (togglingKey.value) return
  togglingKey.value = key
  try {
    let updated
    if (kind === 'exercise') {
      updated = await setExerciseActive(row.id, isActive)
      replaceRow(exercises, updated)
    } else if (kind === 'muscle') {
      updated = await setMuscleGroupActive(row.id, isActive)
      replaceRow(muscleGroups, updated)
    } else {
      updated = await setEquipmentActive(row.id, isActive)
      replaceRow(equipment, updated)
    }
    toast.success(`${updated.name} is now ${updated.is_active ? 'active' : 'inactive'}`)
  } catch (error) {
    toast.error(error.userMessage || 'Could not change the active state.')
  } finally {
    togglingKey.value = ''
  }
}

function openCreate() {
  if (!isAdmin.value) return
  if (activeTab.value === 'exercises') {
    editingExercise.value = null
    exerciseModalOpen.value = true
    return
  }
  itemModalKind.value = activeTab.value === 'muscles' ? 'muscle' : 'equipment'
  editingItem.value = null
  itemModalOpen.value = true
}

function openExercise(exercise) {
  if (!isAdmin.value) return
  editingExercise.value = exercise
  exerciseModalOpen.value = true
}

function openItem(kind, item) {
  if (!isAdmin.value) return
  itemModalKind.value = kind
  editingItem.value = item
  itemModalOpen.value = true
}

async function handleSaved() {
  await loadAll({ silent: true })
}

watch(activeTab, () => {
  search.value = ''
})

onMounted(() => loadAll())
</script>

<template>
  <div class="page">
    <PageHeader
      eyebrow="Reference data"
      title="Library"
      :description="pageDescription"
    >
      <template #actions>
        <button class="btn btn-secondary" type="button" :disabled="loading" @click="loadAll()">
          <Icon name="refresh" :size="16" />
          Refresh
        </button>
        <button v-if="isAdmin" class="btn btn-primary" type="button" @click="openCreate">
          <Icon name="plus" :size="16" />
          {{ panelCopy.addLabel }}
        </button>
      </template>
    </PageHeader>

    <div v-if="!isAdmin" class="notice read-only-notice">
      <Icon name="shield" :size="18" class="notice-icon" />
      <span>
        You are browsing the shared library in read-only mode. An administrator manages
        exercises, muscle groups and equipment for everyone.
      </span>
    </div>

    <div v-if="errorMessage" class="notice notice-danger">
      <Icon name="alert" :size="18" class="notice-icon" />
      <span>{{ errorMessage }}</span>
      <button class="btn btn-sm btn-secondary retry-button" type="button" @click="loadAll()">
        Retry
      </button>
    </div>

    <div v-if="loading" class="summary-grid">
      <div v-for="index in 4" :key="index" class="card skeleton summary-skeleton" />
    </div>

    <div v-else class="summary-grid">
      <StatCard
        label="Exercises"
        :value="counts.exercises.active"
        icon="dumbbell"
        :hint="`${counts.exercises.total} total · ${counts.exercises.total - counts.exercises.active} inactive`"
        compact
      />
      <StatCard
        label="Muscle groups"
        :value="counts.muscles.active"
        icon="target"
        tone="info"
        :hint="`${counts.muscles.total} total · ${counts.muscles.total - counts.muscles.active} inactive`"
        compact
      />
      <StatCard
        label="Equipment"
        :value="counts.equipment.active"
        icon="library"
        tone="violet"
        :hint="`${counts.equipment.total} total · No Equipment reserved`"
        compact
      />
      <StatCard
        label="Inactive"
        :value="inactiveCount"
        icon="shield"
        tone="warning"
        hint="Kept for historical records"
        compact
      />
    </div>

    <section class="library-panel card">
      <header class="library-tabs">
        <div class="segmented tab-bar" role="tablist" aria-label="Library sections">
          <button
            v-for="tab in tabs"
            :key="tab.key"
            class="segment tab-button"
            :class="{ active: activeTab === tab.key }"
            type="button"
            role="tab"
            :aria-selected="activeTab === tab.key"
            @click="activeTab = tab.key"
          >
            <Icon :name="tab.icon" :size="15" />
            {{ tab.label }}
            <span class="tab-count">{{ tabCounts[tab.key] }}</span>
          </button>
        </div>
      </header>

      <div class="library-toolbar">
        <div class="toolbar-copy">
          <h2>
            {{ panelCopy.title }}
            <span v-if="!isAdmin" class="badge badge-muted">Read-only</span>
          </h2>
          <p>{{ panelCopy.description }}</p>
        </div>
        <div class="toolbar-controls">
          <SearchField
            v-model="search"
            :placeholder="panelCopy.searchPlaceholder"
            :label="`Search ${currentTab.label.toLowerCase()}`"
          />
          <ToggleSwitch v-model="showInactive" label="Show inactive" />
        </div>
      </div>

      <div class="list-meta">
        <span>{{ pluralize(visibleCount, 'item') }} shown</span>
        <span v-if="search" class="meta-search">Matching “{{ search }}”</span>
      </div>

      <div v-if="activeTab === 'exercises' && filteredExercises.length" class="library-list">
        <article
          v-for="exercise in filteredExercises"
          :key="exercise.id"
          class="library-row"
          :class="{ inactive: !exercise.is_active }"
        >
          <div class="row-icon">
            <Icon name="dumbbell" :size="18" />
          </div>

          <div class="row-main">
            <div class="row-heading">
              <h3>{{ exercise.name }}</h3>
              <span class="badge">{{ exercise.category }}</span>
              <span v-if="!exercise.is_active" class="badge badge-muted">Inactive</span>
            </div>
            <p v-if="exercise.description" class="row-description">
              {{ exercise.description }}
            </p>
            <div v-if="exercise.muscles.length" class="muscle-tags">
              <span
                v-for="muscle in exercise.muscles"
                :key="muscle.muscle_group_id"
                class="chip muscle-chip"
                :class="muscle.role === 'Primary' ? 'primary' : 'secondary'"
              >
                <span class="role-letter">{{ muscle.role === 'Primary' ? 'P' : 'S' }}</span>
                {{ muscle.muscle_group_name }}
              </span>
            </div>
            <p v-else class="row-description subtle">No muscle groups assigned.</p>
          </div>

          <div v-if="isAdmin" class="row-actions">
            <ToggleSwitch
              :model-value="exercise.is_active"
              :disabled="isToggling('exercise', exercise.id)"
              :label="exercise.is_active ? 'Active' : 'Inactive'"
              @update:model-value="toggle('exercise', exercise, $event)"
            />
            <button
              class="btn btn-ghost btn-icon"
              type="button"
              :aria-label="`Edit ${exercise.name}`"
              title="Edit exercise"
              @click="openExercise(exercise)"
            >
              <Icon name="edit" :size="16" />
            </button>
          </div>
        </article>
      </div>

      <div v-else-if="activeTab === 'muscles' && filteredMuscles.length" class="library-list">
        <article
          v-for="muscle in filteredMuscles"
          :key="muscle.id"
          class="library-row"
          :class="{ inactive: !muscle.is_active }"
        >
          <div class="row-icon muscle-icon">
            <Icon name="target" :size="18" />
          </div>

          <div class="row-main">
            <div class="row-heading">
              <h3>{{ muscle.name }}</h3>
              <span v-if="!muscle.is_active" class="badge badge-muted">Inactive</span>
              <span v-if="muscleUsage.get(muscle.id)" class="badge badge-info">
                Used by {{ pluralize(muscleUsage.get(muscle.id), 'exercise') }}
              </span>
            </div>
            <p v-if="muscle.description" class="row-description">
              {{ muscle.description }}
            </p>
            <p v-else class="row-description subtle">No description.</p>
          </div>

          <div v-if="isAdmin" class="row-actions">
            <ToggleSwitch
              :model-value="muscle.is_active"
              :disabled="isToggling('muscle', muscle.id)"
              :label="muscle.is_active ? 'Active' : 'Inactive'"
              @update:model-value="toggle('muscle', muscle, $event)"
            />
            <button
              class="btn btn-ghost btn-icon"
              type="button"
              :aria-label="`Edit ${muscle.name}`"
              title="Edit muscle group"
              @click="openItem('muscle', muscle)"
            >
              <Icon name="edit" :size="16" />
            </button>
          </div>
        </article>
      </div>

      <div v-else-if="activeTab === 'equipment' && filteredEquipment.length" class="library-list">
        <article
          v-for="item in filteredEquipment"
          :key="item.id"
          class="library-row"
          :class="{ inactive: !item.is_active, reserved: item.is_reserved }"
        >
          <div class="row-icon equipment-icon">
            <Icon name="library" :size="18" />
          </div>

          <div class="row-main">
            <div class="row-heading">
              <h3>{{ item.name }}</h3>
              <span v-if="item.category" class="badge">{{ item.category }}</span>
              <span v-if="item.is_reserved" class="badge badge-accent">Reserved</span>
              <span v-if="!item.is_active" class="badge badge-muted">Inactive</span>
            </div>
            <p class="row-description subtle">
              <template v-if="item.is_reserved">
                Always active and available for bodyweight or equipment-free sets. This record
                cannot be deactivated or deleted.
              </template>
              <template v-else>
                {{ item.is_active ? 'Available when logging new sets.' : 'Hidden from new sets but kept on historical records.' }}
              </template>
            </p>
          </div>

          <div v-if="isAdmin" class="row-actions">
            <ToggleSwitch
              :model-value="item.is_active"
              :disabled="item.is_reserved || isToggling('equipment', item.id)"
              :label="item.is_reserved ? 'Reserved' : item.is_active ? 'Active' : 'Inactive'"
              @update:model-value="toggle('equipment', item, $event)"
            />
            <button
              class="btn btn-ghost btn-icon"
              type="button"
              :aria-label="`Edit ${item.name}`"
              title="Edit equipment"
              @click="openItem('equipment', item)"
            >
              <Icon name="edit" :size="16" />
            </button>
          </div>
        </article>
      </div>

      <EmptyState
        v-else
        :icon="currentTab.icon"
        :title="emptyTitle"
        :message="emptyMessage"
      >
        <button
          v-if="search"
          class="btn btn-secondary"
          type="button"
          @click="search = ''"
        >
          Clear search
        </button>
        <button v-else-if="isAdmin" class="btn btn-primary" type="button" @click="openCreate">
          <Icon name="plus" :size="16" />
          {{ panelCopy.addLabel }}
        </button>
      </EmptyState>
    </section>

    <ExerciseModal
      v-if="isAdmin"
      :open="exerciseModalOpen"
      :exercise="editingExercise"
      :muscle-groups="muscleGroups"
      @close="exerciseModalOpen = false"
      @saved="handleSaved"
    />

    <LibraryItemModal
      v-if="isAdmin"
      :open="itemModalOpen"
      :kind="itemModalKind"
      :item="editingItem"
      @close="itemModalOpen = false"
      @saved="handleSaved"
    />
  </div>
</template>

<style scoped>
.retry-button {
  margin-left: auto;
}

.read-only-notice {
  align-items: center;
}

.summary-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 13px;
}

.summary-skeleton {
  height: 105px;
}

.library-panel {
  overflow: hidden;
}

.library-tabs {
  padding: 15px 18px;
  border-bottom: 1px solid var(--border);
}

.tab-bar {
  display: inline-flex;
}

.tab-button {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  padding-right: 13px;
  padding-left: 13px;
}

.tab-count {
  display: inline-grid;
  min-width: 20px;
  height: 19px;
  padding: 0 5px;
  border-radius: 6px;
  background: rgba(148, 163, 184, 0.11);
  color: var(--muted-2);
  font-family: var(--font-mono);
  font-size: 0.61rem;
  place-items: center;
}

.segment.active .tab-count {
  background: var(--accent-soft);
  color: var(--accent);
}

.library-toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-start;
  justify-content: space-between;
  gap: 18px;
  padding: 20px 22px 14px;
}

.toolbar-copy {
  max-width: 620px;
}

.toolbar-copy h2 {
  font-size: 1.12rem;
}

.toolbar-copy p {
  margin-top: 6px;
  color: var(--muted);
  font-size: 0.79rem;
  line-height: 1.6;
}

.toolbar-controls {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 13px;
}

.list-meta {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 22px 12px;
  color: var(--muted-2);
  font-size: 0.68rem;
}

.meta-search {
  color: var(--muted);
}

.library-list {
  display: flex;
  flex-direction: column;
  border-top: 1px solid var(--border);
}

.library-row {
  display: flex;
  align-items: flex-start;
  gap: 14px;
  padding: 16px 22px;
  border-bottom: 1px solid var(--border);
  transition: background 140ms ease;
}

.library-row:last-child {
  border-bottom: 0;
}

.library-row:hover {
  background: rgba(255, 255, 255, 0.014);
}

.library-row.inactive {
  opacity: 0.67;
}

.library-row.reserved {
  background: rgba(94, 234, 212, 0.025);
}

.row-icon {
  display: grid;
  width: 40px;
  height: 40px;
  flex: 0 0 auto;
  border: 1px solid var(--border);
  border-radius: 11px;
  background: var(--surface-2);
  color: var(--accent);
  place-items: center;
}

.muscle-icon {
  color: var(--info);
}

.equipment-icon {
  color: var(--violet);
}

.row-main {
  min-width: 0;
  flex: 1;
}

.row-heading {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 7px;
}

.row-heading h3 {
  font-size: 0.93rem;
}

.row-description {
  margin-top: 5px;
  color: var(--muted);
  font-size: 0.76rem;
  line-height: 1.55;
}

.muscle-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
  margin-top: 8px;
}

.muscle-chip {
  min-height: 22px;
  padding: 2px 8px;
  font-size: 0.64rem;
}

.muscle-chip.primary {
  border-color: var(--accent-line);
  background: var(--accent-soft);
  color: var(--accent);
}

.muscle-chip.secondary {
  color: var(--muted);
}

.role-letter {
  display: inline-grid;
  width: 14px;
  height: 14px;
  border-radius: 4px;
  background: rgba(148, 163, 184, 0.13);
  font-family: var(--font-mono);
  font-size: 0.53rem;
  font-weight: 700;
  place-items: center;
}

.muscle-chip.primary .role-letter {
  background: rgba(94, 234, 212, 0.16);
}

.row-actions {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 7px;
  padding-top: 4px;
}

@media (max-width: 1000px) {
  .summary-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 760px) {
  .summary-grid {
    gap: 9px;
  }

  .library-tabs {
    padding: 12px;
    overflow-x: auto;
  }

  .tab-bar {
    display: flex;
    width: max-content;
  }

  .library-toolbar {
    flex-direction: column;
    padding: 17px 17px 12px;
  }

  .toolbar-controls {
    width: 100%;
    justify-content: space-between;
  }

  .toolbar-controls :deep(.search-field) {
    flex: 1;
  }

  .list-meta {
    padding: 0 17px 10px;
  }

  .library-row {
    flex-wrap: wrap;
    gap: 11px;
    padding: 15px 17px;
  }

  .row-main {
    flex-basis: calc(100% - 52px);
  }

  .row-actions {
    width: 100%;
    justify-content: flex-end;
    padding-top: 0;
  }

  .row-heading h3 {
    font-size: 0.88rem;
  }
}
</style>
