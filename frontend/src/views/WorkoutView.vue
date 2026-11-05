<script setup>
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { toast } from 'vue3-toastify'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import EmptyState from '@/components/EmptyState.vue'
import Icon from '@/components/Icon.vue'
import PageHeader from '@/components/PageHeader.vue'
import StatCard from '@/components/StatCard.vue'
import WorkoutSetModal from '@/components/WorkoutSetModal.vue'
import { deleteWorkoutSet, getWorkoutDay } from '@/api/workouts'
import { getEquipment, getExercises } from '@/api/library'
import {
  formatDateLong,
  formatNumber,
  formatRest,
  formatWeight,
  shiftDateISO,
  todayISO,
} from '@/utils/format'

const route = useRoute()
const router = useRouter()

const day = ref(null)
const loading = ref(true)
const refreshing = ref(false)
const errorMessage = ref('')
const selectedDate = ref(todayISO())
const exercises = ref([])
const equipment = ref([])
const modalOpen = ref(false)
const editingSet = ref(null)
const deleteTarget = ref(null)
const deleteBusy = ref(false)

const today = todayISO()

const isToday = computed(() => selectedDate.value === today)
const canGoNext = computed(() => selectedDate.value < today)
const sets = computed(() => day.value?.sets || [])
const totalEffectiveReps = computed(() =>
  sets.value.reduce((sum, item) => sum + Number(item.effective_reps || 0), 0),
)
const exerciseCount = computed(() => new Set(sets.value.map((item) => item.exercise_id)).size)
const dateHeading = computed(() => formatDateLong(selectedDate.value))
const description = computed(() =>
  isToday.value
    ? 'Log the work you put in today. Every set becomes part of your history and analytics.'
    : `Reviewing ${dateHeading.value}. Add, edit, or remove sets without changing the sequence of the rest.`,
)

async function loadReferences() {
  try {
    const [exerciseRows, equipmentRows] = await Promise.all([
      getExercises({ includeInactive: true }),
      getEquipment({ includeInactive: true }),
    ])
    exercises.value = exerciseRows
    equipment.value = equipmentRows
  } catch (error) {
    toast.error(error.userMessage || 'Could not load exercises and equipment.')
  }
}

async function loadDay(date, { silent = false } = {}) {
  if (!date) return
  if (silent) refreshing.value = true
  else loading.value = true
  errorMessage.value = ''
  try {
    day.value = await getWorkoutDay(date)
  } catch (error) {
    errorMessage.value = error.userMessage || 'Could not load this workout day.'
  } finally {
    loading.value = false
    refreshing.value = false
  }
}

function setDate(date) {
  if (!date) return
  if (date > today) {
    toast.error('Workouts cannot be logged for a future date.')
    return
  }
  if (date === selectedDate.value) return
  router.push({ name: 'workout', query: { date } })
}

function moveDay(offset) {
  setDate(shiftDateISO(selectedDate.value, offset))
}

function openAdd() {
  if (!isToday.value && selectedDate.value > today) {
    toast.error('Choose a past or present date first.')
    return
  }
  editingSet.value = null
  modalOpen.value = true
}

function openEdit(workoutSet) {
  editingSet.value = workoutSet
  modalOpen.value = true
}

async function handleSaved() {
  await loadDay(selectedDate.value, { silent: true })
}

function askDelete(workoutSet) {
  deleteTarget.value = workoutSet
}

async function confirmDelete() {
  if (!deleteTarget.value) return
  deleteBusy.value = true
  try {
    await deleteWorkoutSet(deleteTarget.value.date, deleteTarget.value.sequence_number)
    toast.success(`Set ${deleteTarget.value.sequence_number} deleted`)
    deleteTarget.value = null
    await loadDay(selectedDate.value, { silent: true })
  } catch (error) {
    toast.error(error.userMessage || 'Could not delete the set.')
  } finally {
    deleteBusy.value = false
  }
}

watch(
  () => route.query.date,
  (value) => {
    let date = typeof value === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(value) ? value : today
    if (date > today) {
      date = today
      router.replace({ name: 'workout' })
      return
    }
    selectedDate.value = date
    loadDay(date)
  },
  { immediate: true },
)

onMounted(loadReferences)
</script>

<template>
  <div class="page">
    <PageHeader
      eyebrow="Training log"
      title="Workout"
      :description="description"
    >
      <template #actions>
        <button class="btn btn-primary" type="button" @click="openAdd">
          <Icon name="plus" :size="17" />
          Log a set
        </button>
      </template>
    </PageHeader>

    <section class="day-toolbar card">
      <div class="date-navigator">
        <button
          class="btn btn-ghost btn-icon"
          type="button"
          aria-label="Previous day"
          title="Previous day"
          @click="moveDay(-1)"
        >
          <Icon name="chevronLeft" :size="18" />
        </button>
        <label class="date-field">
          <Icon name="calendar" :size="16" />
          <span class="visually-hidden">Workout date</span>
          <input
            class="date-input"
            type="date"
            :value="selectedDate"
            :max="today"
            min="1900-01-01"
            @change="setDate($event.target.value)"
          />
        </label>
        <button
          class="btn btn-ghost btn-icon"
          type="button"
          aria-label="Next day"
          title="Next day"
          :disabled="!canGoNext"
          @click="moveDay(1)"
        >
          <Icon name="chevronRight" :size="18" />
        </button>
      </div>

      <div class="day-toolbar-right">
        <span v-if="isToday" class="badge badge-accent">Today</span>
        <span v-else class="badge">{{ formatDateLong(selectedDate) }}</span>
        <button
          class="btn btn-secondary btn-sm"
          type="button"
          :disabled="isToday"
          @click="setDate(today)"
        >
          Jump to today
        </button>
        <button
          class="btn btn-ghost btn-icon btn-sm"
          type="button"
          aria-label="Refresh workout"
          title="Refresh"
          :disabled="refreshing"
          @click="loadDay(selectedDate, { silent: true })"
        >
          <Icon name="refresh" :size="15" :class="{ spinning: refreshing }" />
        </button>
      </div>
    </section>

    <div v-if="errorMessage" class="notice notice-danger error-banner">
      <Icon name="alert" :size="18" class="notice-icon" />
      <span>{{ errorMessage }}</span>
      <button class="btn btn-sm btn-secondary" type="button" @click="loadDay(selectedDate)">
        Retry
      </button>
    </div>

    <template v-if="loading && !day">
      <div class="summary-grid">
        <div v-for="index in 4" :key="index" class="card skeleton summary-skeleton" />
      </div>
      <div class="card skeleton sets-skeleton" />
    </template>

    <template v-else>
      <div class="summary-grid">
        <StatCard
          label="Sets"
          :value="sets.length"
          icon="dumbbell"
          hint="Recorded on this day"
          compact
        />
        <StatCard
          label="Volume"
          :value="formatNumber(day?.total_volume || 0, 1)"
          unit="kg"
          icon="chartBar"
          tone="info"
          hint="Weight × effective reps"
          compact
        />
        <StatCard
          label="Effective reps"
          :value="formatNumber(totalEffectiveReps, 1)"
          icon="target"
          tone="violet"
          hint="Complete + ½ partials"
          compact
        />
        <StatCard
          label="Exercises"
          :value="exerciseCount"
          icon="library"
          tone="warning"
          hint="Distinct movements"
          compact
        />
      </div>

      <section class="sets-card card">
        <header class="sets-header">
          <div>
            <h2>Recorded sets</h2>
            <p v-if="sets.length">
              Sequence numbers stay stable even after a set is deleted.
            </p>
            <p v-else>No sets have been logged for this date yet.</p>
          </div>
          <span v-if="sets.length" class="badge">{{ sets.length }} total</span>
        </header>

        <div v-if="sets.length" class="set-list">
          <article
            v-for="workoutSet in sets"
            :key="workoutSet.sequence_number"
            class="set-row"
          >
            <div class="sequence-badge">
              <span>#</span>
              <strong>{{ String(workoutSet.sequence_number).padStart(2, '0') }}</strong>
            </div>

            <div class="set-main">
              <div class="set-topline">
                <div class="set-title">
                  <h3>{{ workoutSet.exercise_name }}</h3>
                  <span class="equipment-name">{{ workoutSet.equipment_name }}</span>
                </div>
                <div class="set-actions">
                  <button
                    class="btn btn-ghost btn-icon btn-sm"
                    type="button"
                    :aria-label="`Edit set ${workoutSet.sequence_number}`"
                    title="Edit set"
                    @click="openEdit(workoutSet)"
                  >
                    <Icon name="edit" :size="15" />
                  </button>
                  <button
                    class="btn btn-ghost btn-icon btn-sm delete-button"
                    type="button"
                    :aria-label="`Delete set ${workoutSet.sequence_number}`"
                    title="Delete set"
                    @click="askDelete(workoutSet)"
                  >
                    <Icon name="trash" :size="15" />
                  </button>
                </div>
              </div>

              <div class="set-data">
                <div class="data-block primary-data">
                  <span class="data-label">Weight × reps</span>
                  <strong>
                    {{ formatNumber(workoutSet.weight, 2) }} kg
                    <span class="times">×</span>
                    {{ workoutSet.complete_reps }}
                    <small v-if="workoutSet.partial_reps">+ {{ workoutSet.partial_reps }} half</small>
                  </strong>
                </div>
                <div class="data-block">
                  <span class="data-label">Effective</span>
                  <strong>{{ formatNumber(workoutSet.effective_reps, 1) }} <small>reps</small></strong>
                </div>
                <div class="data-block">
                  <span class="data-label">Volume</span>
                  <strong>{{ formatNumber(workoutSet.volume, 1) }} <small>kg</small></strong>
                </div>
              </div>

              <div v-if="workoutSet.rir !== null || workoutSet.rest_seconds !== null || workoutSet.extra_info" class="set-meta">
                <span v-if="workoutSet.rir !== null" class="badge badge-muted">
                  RIR {{ workoutSet.rir }}
                </span>
                <span v-if="workoutSet.rest_seconds !== null" class="badge badge-muted">
                  <Icon name="clock" :size="12" />
                  {{ formatRest(workoutSet.rest_seconds) }}
                </span>
                <p v-if="workoutSet.extra_info" class="set-note">{{ workoutSet.extra_info }}</p>
              </div>
            </div>
          </article>
        </div>

        <EmptyState
          v-else
          icon="dumbbell"
          title="A clean slate"
          message="Log your first set for this date. Reps, weight, RIR, and rest are all captured in one place."
        >
          <button class="btn btn-primary" type="button" @click="openAdd">
            <Icon name="plus" :size="16" />
            Log the first set
          </button>
        </EmptyState>
      </section>
    </template>

    <WorkoutSetModal
      :open="modalOpen"
      :date="selectedDate"
      :workout-set="editingSet"
      :exercises="exercises"
      :equipment="equipment"
      @close="modalOpen = false"
      @saved="handleSaved"
    />

    <ConfirmDialog
      :open="Boolean(deleteTarget)"
      title="Delete this set?"
      :message="deleteTarget ? `Set ${deleteTarget.sequence_number} (${deleteTarget.exercise_name}) will be removed. The remaining sets keep their existing sequence numbers.` : ''"
      confirm-label="Delete set"
      danger
      :busy="deleteBusy"
      @close="deleteTarget = null"
      @confirm="confirmDelete"
    />
  </div>
</template>

<style scoped>
.day-toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  padding: 12px 14px;
}

.date-navigator {
  display: flex;
  align-items: center;
  gap: 5px;
}

.date-field {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 40px;
  padding: 0 11px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--surface-2);
  color: var(--accent);
}

.date-input {
  width: 145px;
  border: 0;
  outline: 0;
  background: transparent;
  color: var(--text);
  font-family: var(--font-mono);
  font-size: 0.8rem;
}

.date-input::-webkit-calendar-picker-indicator {
  cursor: pointer;
  opacity: 0.55;
}

.day-toolbar-right {
  display: flex;
  align-items: center;
  gap: 9px;
}

.badge :deep(svg) {
  flex: 0 0 auto;
}

.error-banner {
  align-items: center;
}

.error-banner .btn {
  margin-left: auto;
}

.summary-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 13px;
}

.summary-skeleton {
  height: 105px;
}

.sets-skeleton {
  height: 360px;
}

.sets-card {
  overflow: hidden;
}

.sets-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  padding: 20px 22px;
  border-bottom: 1px solid var(--border);
}

.sets-header p {
  margin-top: 5px;
  color: var(--muted);
  font-size: 0.8rem;
}

.set-list {
  display: flex;
  flex-direction: column;
}

.set-row {
  display: flex;
  gap: 15px;
  padding: 18px 22px;
  border-bottom: 1px solid var(--border);
  transition: background 140ms ease;
}

.set-row:last-child {
  border-bottom: 0;
}

.set-row:hover {
  background: rgba(255, 255, 255, 0.012);
}

.sequence-badge {
  display: flex;
  width: 45px;
  height: 45px;
  flex: 0 0 auto;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--border);
  border-radius: 13px;
  background: var(--surface-2);
  color: var(--muted-2);
  font-family: var(--font-mono);
  line-height: 1;
}

.sequence-badge span {
  font-size: 0.55rem;
}

.sequence-badge strong {
  margin-top: 3px;
  color: var(--accent);
  font-size: 0.83rem;
  font-weight: 600;
}

.set-main {
  min-width: 0;
  flex: 1;
}

.set-topline {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.set-title {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 8px;
}

.set-title h3 {
  font-size: 0.98rem;
}

.equipment-name {
  color: var(--muted-2);
  font-size: 0.72rem;
}

.set-actions {
  display: flex;
  gap: 2px;
  margin-top: -5px;
  margin-right: -5px;
}

.delete-button:hover {
  color: var(--danger);
}

.set-data {
  display: flex;
  flex-wrap: wrap;
  gap: 26px;
  margin-top: 11px;
}

.data-block {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.data-label {
  color: var(--muted-2);
  font-size: 0.6rem;
  font-weight: 650;
  letter-spacing: 0.07em;
  text-transform: uppercase;
}

.data-block strong {
  color: var(--text-soft);
  font-family: var(--font-mono);
  font-size: 0.83rem;
  font-weight: 550;
  white-space: nowrap;
}

.data-block small {
  color: var(--muted);
  font-size: 0.68rem;
  font-weight: 500;
}

.primary-data strong {
  color: var(--text);
  font-size: 0.94rem;
}

.times {
  margin: 0 3px;
  color: var(--muted-2);
}

.set-meta {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 7px;
  margin-top: 12px;
}

.set-note {
  flex-basis: 100%;
  padding: 8px 10px;
  border-left: 2px solid var(--border-strong);
  color: var(--muted);
  font-size: 0.74rem;
  line-height: 1.55;
}

.spinning {
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

@media (max-width: 1000px) {
  .summary-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 700px) {
  .day-toolbar {
    align-items: stretch;
    flex-direction: column;
  }

  .date-navigator {
    justify-content: space-between;
  }

  .date-field {
    flex: 1;
    justify-content: center;
  }

  .date-input {
    width: auto;
    flex: 1;
    text-align: center;
  }

  .day-toolbar-right {
    justify-content: space-between;
  }

  .day-toolbar-right .badge {
    flex: 1;
  }

  .summary-grid {
    gap: 9px;
  }

  .sets-header {
    padding: 17px;
  }

  .set-row {
    gap: 11px;
    padding: 16px;
  }

  .sequence-badge {
    width: 38px;
    height: 38px;
    border-radius: 11px;
  }

  .set-data {
    gap: 15px;
  }

  .data-block strong {
    font-size: 0.78rem;
  }

  .primary-data {
    flex-basis: 100%;
  }

  .set-title h3 {
    font-size: 0.92rem;
  }
}
</style>
