<script setup>
import { computed, ref, watch } from 'vue'
import { toast } from 'vue3-toastify'
import BaseModal from './BaseModal.vue'
import Icon from './Icon.vue'
import ToggleSwitch from './ToggleSwitch.vue'
import { createExercise, updateExercise } from '@/api/library'

const props = defineProps({
  open: { type: Boolean, default: false },
  exercise: { type: Object, default: null },
  muscleGroups: { type: Array, default: () => [] },
})

const emit = defineEmits(['close', 'saved'])

const categories = ['Compound', 'Isolation', 'Bodyweight', 'Other']
const saving = ref(false)
const form = ref(emptyForm())

function emptyForm() {
  return {
    name: '',
    category: 'Compound',
    description: '',
    is_active: true,
    muscles: [],
  }
}

const editing = computed(() => Boolean(props.exercise))

const availableMuscles = computed(() => {
  const assigned = new Set(form.value.muscles.map((row) => row.muscle_group_id))
  return props.muscleGroups.filter(
    (muscle) => muscle.is_active || assigned.has(muscle.id),
  )
})

watch(
  () => props.open,
  (isOpen) => {
    if (!isOpen) return
    if (props.exercise) {
      form.value = {
        name: props.exercise.name,
        category: props.exercise.category,
        description: props.exercise.description || '',
        is_active: Boolean(props.exercise.is_active),
        muscles: props.exercise.muscles.map((row) => ({
          muscle_group_id: row.muscle_group_id,
          role: row.role,
        })),
      }
      return
    }
    form.value = emptyForm()
  },
  { immediate: true },
)

function optionsFor(index) {
  const selectedElsewhere = new Set(
    form.value.muscles
      .filter((_row, rowIndex) => rowIndex !== index)
      .map((row) => row.muscle_group_id),
  )
  return availableMuscles.value.filter((muscle) => !selectedElsewhere.has(muscle.id))
}

function addMuscleRow() {
  const selected = new Set(form.value.muscles.map((row) => row.muscle_group_id))
  const next = availableMuscles.value.find((muscle) => !selected.has(muscle.id))
  if (!next) {
    toast.info('Every muscle group is already assigned.')
    return
  }
  const hasPrimary = form.value.muscles.some((row) => row.role === 'Primary')
  form.value.muscles.push({
    muscle_group_id: next.id,
    role: hasPrimary ? 'Secondary' : 'Primary',
  })
}

function removeMuscleRow(index) {
  form.value.muscles.splice(index, 1)
}

function validate() {
  if (!form.value.name.trim()) {
    toast.error('Exercise name is required.')
    return false
  }
  const seen = new Set()
  for (const row of form.value.muscles) {
    if (!row.muscle_group_id) {
      toast.error('Choose a muscle group for every assignment.')
      return false
    }
    if (seen.has(row.muscle_group_id)) {
      toast.error('A muscle group can only be assigned once per exercise.')
      return false
    }
    seen.add(row.muscle_group_id)
  }
  return true
}

async function submit() {
  if (saving.value || !validate()) return
  saving.value = true
  const payload = {
    name: form.value.name,
    category: form.value.category,
    description: form.value.description,
    is_active: Boolean(form.value.is_active),
    muscles: form.value.muscles.map((row) => ({
      muscle_group_id: Number(row.muscle_group_id),
      role: row.role,
    })),
  }
  try {
    const saved = props.exercise
      ? await updateExercise(props.exercise.id, payload)
      : await createExercise(payload)
    toast.success(editing.value ? 'Exercise updated' : 'Exercise created')
    emit('saved', saved)
    emit('close')
  } catch (error) {
    toast.error(error.userMessage || 'Could not save the exercise.')
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <BaseModal
    :open="open"
    :title="editing ? 'Edit exercise' : 'New exercise'"
    subtitle="Library catalogue"
    size="lg"
    @close="emit('close')"
  >
    <form id="exercise-form" class="exercise-form" @submit.prevent="submit">
      <div class="form-grid two">
        <label class="field">
          <span class="field-label">Exercise name</span>
          <input
            v-model="form.name"
            class="input"
            type="text"
            maxlength="120"
            placeholder="e.g. Incline Cable Press"
            data-autofocus
            required
          />
        </label>

        <label class="field">
          <span class="field-label">Category</span>
          <select v-model="form.category" class="select">
            <option v-for="category in categories" :key="category" :value="category">
              {{ category }}
            </option>
          </select>
        </label>
      </div>

      <label class="field">
        <span class="field-label">Description <span class="optional">optional</span></span>
        <textarea
          v-model="form.description"
          class="textarea"
          rows="2"
          maxlength="4000"
          placeholder="Setup cues, range of motion, or notes about this movement."
        />
      </label>

      <div class="muscles-section">
        <div class="muscles-header">
          <div>
            <h3>Muscle assignments</h3>
            <p>Primary work receives full volume attribution; secondary receives half.</p>
          </div>
          <button class="btn btn-secondary btn-sm" type="button" @click="addMuscleRow">
            <Icon name="plus" :size="14" />
            Add muscle
          </button>
        </div>

        <div v-if="form.muscles.length" class="assignment-list">
          <div
            v-for="(row, index) in form.muscles"
            :key="`${row.muscle_group_id}-${index}`"
            class="assignment-row"
          >
            <span class="assignment-index">{{ index + 1 }}</span>
            <label class="field assignment-field">
              <span class="visually-hidden">Muscle group</span>
              <select v-model.number="row.muscle_group_id" class="select">
                <option
                  v-for="muscle in optionsFor(index)"
                  :key="muscle.id"
                  :value="muscle.id"
                >
                  {{ muscle.name }}{{ muscle.is_active ? '' : ' · inactive' }}
                </option>
              </select>
            </label>

            <select v-model="row.role" class="select role-select" aria-label="Muscle role">
              <option value="Primary">Primary</option>
              <option value="Secondary">Secondary</option>
            </select>

            <button
              class="btn btn-ghost btn-icon delete-row"
              type="button"
              aria-label="Remove muscle assignment"
              title="Remove assignment"
              @click="removeMuscleRow(index)"
            >
              <Icon name="trash" :size="15" />
            </button>
          </div>
        </div>

        <p v-else class="no-assignments">
          No muscle groups assigned. You can still save the exercise and add them later.
        </p>
      </div>

      <div v-if="editing" class="active-row">
        <ToggleSwitch v-model="form.is_active" label="Exercise is active" />
        <span class="field-hint">
          Inactive exercises stay in history but cannot be used for new sets.
        </span>
      </div>
    </form>

    <template #footer>
      <button class="btn btn-secondary" type="button" :disabled="saving" @click="emit('close')">
        Cancel
      </button>
      <button class="btn btn-primary" type="submit" form="exercise-form" :disabled="saving">
        <span v-if="saving" class="spinner" />
        {{ editing ? 'Save changes' : 'Create exercise' }}
      </button>
    </template>
  </BaseModal>
</template>

<style scoped>
.exercise-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.muscles-section {
  padding: 16px;
  border: 1px solid var(--border);
  border-radius: 13px;
  background: var(--surface-2);
}

.muscles-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
}

.muscles-header h3 {
  font-size: 0.875rem;
}

.muscles-header p {
  margin-top: 4px;
  color: var(--muted);
  font-size: 0.75rem;
  line-height: 1.5;
}

.assignment-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.assignment-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.assignment-index {
  width: 20px;
  flex: 0 0 auto;
  color: var(--muted-2);
  font-family: var(--font-mono);
  font-size: 0.75rem;
  text-align: right;
}

.assignment-field {
  flex: 1;
}

.assignment-row .select {
  min-height: 39px;
  height: 39px;
  padding-top: 6px;
  padding-bottom: 6px;
  font-size: 0.8125rem;
}

.role-select {
  width: 122px;
  flex: 0 0 auto;
}

.delete-row:hover {
  color: var(--danger);
}

.no-assignments {
  color: var(--muted-2);
  font-size: 0.75rem;
  line-height: 1.55;
}

.active-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding-top: 2px;
}

@media (max-width: 620px) {
  .assignment-row {
    flex-wrap: wrap;
  }

  .assignment-index {
    display: none;
  }

  .assignment-field {
    flex-basis: calc(100% - 130px);
  }

  .role-select {
    width: 100px;
  }
}
</style>
