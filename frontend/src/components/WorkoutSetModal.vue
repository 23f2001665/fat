<script setup>
import { computed, ref, watch } from 'vue'
import { toast } from 'vue3-toastify'
import BaseModal from './BaseModal.vue'
import Icon from './Icon.vue'
import { createWorkoutSet, updateWorkoutSet } from '@/api/workouts'
import { formatDateLong, formatNumber } from '@/utils/format'

const props = defineProps({
  open: { type: Boolean, default: false },
  date: { type: String, required: true },
  workoutSet: { type: Object, default: null },
  // Set to copy values from when logging a new set (smart default).
  templateSet: { type: Object, default: null },
  // Sets already logged on this date, used to suggest numbers per exercise.
  daySets: { type: Array, default: () => [] },
  exercises: { type: Array, default: () => [] },
  equipment: { type: Array, default: () => [] },
})

const emit = defineEmits(['close', 'saved'])

const saving = ref(false)
const form = ref(emptyForm())

function emptyForm() {
  return {
    date: props.date,
    exercise_id: '',
    equipment_id: '',
    weight: 0,
    complete_reps: 0,
    partial_reps: 0,
    rest_seconds: '',
    rir: '',
    extra_info: '',
  }
}

const editing = computed(() => Boolean(props.workoutSet))
const prefilledFrom = ref(null)

function lastSetFor(exerciseId) {
  const matches = props.daySets.filter((item) => item.exercise_id === Number(exerciseId))
  return matches[matches.length - 1] || null
}

const previousForExercise = computed(() =>
  editing.value ? null : lastSetFor(form.value.exercise_id),
)

function describeSet(item) {
  const partial = item.partial_reps ? ` + ${item.partial_reps} half` : ''
  return `${formatNumber(item.weight, 2)} kg × ${item.complete_reps}${partial}`
}

// When the exercise changes, carry over the numbers from its last set today.
function applyExerciseDefaults() {
  if (editing.value) return
  const previous = lastSetFor(form.value.exercise_id)
  if (!previous) return
  form.value.equipment_id = previous.equipment_id
  form.value.weight = previous.weight
  form.value.complete_reps = previous.complete_reps
  form.value.partial_reps = previous.partial_reps
}

const availableExercises = computed(() => {
  const currentId = props.workoutSet?.exercise_id
  return props.exercises.filter((item) => item.is_active || item.id === currentId)
})

const availableEquipment = computed(() => {
  const currentId = props.workoutSet?.equipment_id
  return props.equipment.filter((item) => item.is_active || item.id === currentId)
})

watch(
  () => props.open,
  (isOpen) => {
    if (!isOpen) return
    prefilledFrom.value = null
    if (props.workoutSet) {
      form.value = {
        date: props.workoutSet.date,
        exercise_id: props.workoutSet.exercise_id,
        equipment_id: props.workoutSet.equipment_id,
        weight: props.workoutSet.weight,
        complete_reps: props.workoutSet.complete_reps,
        partial_reps: props.workoutSet.partial_reps,
        rest_seconds: props.workoutSet.rest_seconds ?? '',
        rir: props.workoutSet.rir ?? '',
        extra_info: props.workoutSet.extra_info || '',
      }
      return
    }
    const template = props.templateSet
    const templateUsable =
      template &&
      availableExercises.value.some((item) => item.id === template.exercise_id && item.is_active) &&
      availableEquipment.value.some((item) => item.id === template.equipment_id && item.is_active)
    if (templateUsable) {
      prefilledFrom.value = template
      form.value = {
        ...emptyForm(),
        exercise_id: template.exercise_id,
        equipment_id: template.equipment_id,
        weight: template.weight,
        complete_reps: template.complete_reps,
        partial_reps: template.partial_reps,
        rest_seconds: template.rest_seconds ?? '',
        rir: template.rir ?? '',
      }
      return
    }
    const firstExercise = availableExercises.value.find((item) => item.is_active) || availableExercises.value[0]
    const preferredEquipment =
      availableEquipment.value.find((item) => item.id === 2) ||
      availableEquipment.value.find((item) => item.is_active) ||
      availableEquipment.value[0]
    form.value = {
      ...emptyForm(),
      exercise_id: firstExercise?.id || '',
      equipment_id: preferredEquipment?.id || '',
    }
  },
  { immediate: true },
)

const effectiveReps = computed(() => {
  const complete = Number(form.value.complete_reps) || 0
  const partial = Number(form.value.partial_reps) || 0
  return complete + partial / 2
})

const volume = computed(() => (Number(form.value.weight) || 0) * effectiveReps.value)

function optionalNumber(value) {
  return value === '' || value === null || value === undefined ? null : Number(value)
}

function validate() {
  if (!form.value.exercise_id) {
    toast.error('Choose an exercise.')
    return false
  }
  if (!form.value.equipment_id) {
    toast.error('Choose equipment. Use No Equipment when none is needed.')
    return false
  }
  if (form.value.weight === '' || Number(form.value.weight) < 0) {
    toast.error('Weight must be zero or more.')
    return false
  }
  const complete = Number(form.value.complete_reps) || 0
  const partial = Number(form.value.partial_reps) || 0
  if (complete < 0 || partial < 0) {
    toast.error('Reps cannot be negative.')
    return false
  }
  if (complete + partial === 0) {
    toast.error('A set needs at least one complete or partial rep.')
    return false
  }
  return true
}

async function submit() {
  if (saving.value || !validate()) return
  saving.value = true
  const payload = {
    date: props.workoutSet?.date || props.date,
    exercise_id: Number(form.value.exercise_id),
    equipment_id: Number(form.value.equipment_id),
    weight: Number(form.value.weight),
    complete_reps: Number(form.value.complete_reps) || 0,
    partial_reps: Number(form.value.partial_reps) || 0,
    rest_seconds: optionalNumber(form.value.rest_seconds),
    rir: optionalNumber(form.value.rir),
    extra_info: form.value.extra_info || '',
  }
  try {
    const result = props.workoutSet
      ? await updateWorkoutSet(
          props.workoutSet.date,
          props.workoutSet.sequence_number,
          payload,
        )
      : await createWorkoutSet(payload)
    toast.success(
      props.workoutSet
        ? `Set ${props.workoutSet.sequence_number} updated`
        : `Set ${result.set.sequence_number} logged`,
    )
    emit('saved', result.set)
    emit('close')
  } catch (error) {
    toast.error(error.userMessage || 'Could not save the set.')
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <BaseModal
    :open="open"
    :title="editing ? `Edit set #${workoutSet.sequence_number}` : 'Log a set'"
    :subtitle="formatDateLong(workoutSet?.date || date)"
    size="lg"
    @close="emit('close')"
  >
    <form id="workout-set-form" class="set-form" @submit.prevent="submit">
      <div v-if="!availableExercises.length || !availableEquipment.length" class="notice notice-danger">
        <Icon name="alert" :size="17" class="notice-icon" />
        <span>
          Add at least one active exercise and one piece of equipment in the Library before
          logging a set.
        </span>
      </div>

      <div v-if="prefilledFrom" class="notice notice-accent prefill-note">
        <Icon name="repeat" :size="16" class="notice-icon" />
        <span>Prefilled from set #{{ prefilledFrom.sequence_number }}. Change only what’s different.</span>
      </div>

      <div class="form-grid two">
        <label class="field">
          <span class="field-label">Exercise</span>
          <select v-model="form.exercise_id" class="select" required @change="applyExerciseDefaults">
            <option disabled value="">Choose an exercise</option>
            <option v-for="exercise in availableExercises" :key="exercise.id" :value="exercise.id">
              {{ exercise.name }}{{ exercise.is_active ? '' : ' · inactive' }}
            </option>
          </select>
          <span v-if="previousForExercise" class="field-hint previous-hint">
            Last set today: <strong class="mono">{{ describeSet(previousForExercise) }}</strong>
          </span>
        </label>

        <label class="field">
          <span class="field-label">Equipment</span>
          <select v-model="form.equipment_id" class="select" required>
            <option disabled value="">Choose equipment</option>
            <option v-for="item in availableEquipment" :key="item.id" :value="item.id">
              {{ item.name }}{{ item.is_active ? '' : ' · inactive' }}
            </option>
          </select>
        </label>
      </div>

      <div class="form-grid three">
        <label class="field">
          <span class="field-label">Weight <span class="optional">kg</span></span>
          <input
            v-model.number="form.weight"
            class="input"
            type="number"
            inputmode="decimal"
            min="0"
            max="2000"
            step="0.5"
            required
          />
          <span class="field-hint">Use 0 for bodyweight sets.</span>
        </label>

        <label class="field">
          <span class="field-label">Complete reps</span>
          <input
            v-model.number="form.complete_reps"
            class="input"
            type="number"
            inputmode="numeric"
            min="0"
            max="10000"
            step="1"
          />
        </label>

        <label class="field">
          <span class="field-label">Partial reps <span class="optional">half reps</span></span>
          <input
            v-model.number="form.partial_reps"
            class="input"
            type="number"
            inputmode="numeric"
            min="0"
            max="10000"
            step="1"
          />
        </label>
      </div>

      <div class="preview-strip">
        <div class="preview-item">
          <span>Effective reps</span>
          <strong>{{ formatNumber(effectiveReps, 2) }}</strong>
        </div>
        <div class="preview-divider" />
        <div class="preview-item">
          <span>Set volume</span>
          <strong>{{ formatNumber(volume, 1) }} <small>kg</small></strong>
        </div>
        <p>2 partial reps = 1 full rep.</p>
      </div>

      <div class="form-grid two">
        <label class="field">
          <span class="field-label">Rest <span class="optional">seconds</span></span>
          <input
            v-model.number="form.rest_seconds"
            class="input"
            type="number"
            inputmode="numeric"
            min="0"
            max="86400"
            step="1"
            placeholder="Optional"
          />
        </label>

        <label class="field">
          <span class="field-label">RIR <span class="optional">0–10</span></span>
          <input
            v-model.number="form.rir"
            class="input"
            type="number"
            inputmode="numeric"
            min="0"
            max="10"
            step="1"
            placeholder="Reps in reserve"
          />
        </label>
      </div>

      <label class="field">
        <span class="field-label">Extra information <span class="optional">optional</span></span>
        <textarea
          v-model="form.extra_info"
          class="textarea"
          rows="3"
          maxlength="4000"
          placeholder="Tempo, setup, how it felt, or anything worth remembering."
        />
      </label>
    </form>

    <template #footer>
      <button class="btn btn-secondary" type="button" :disabled="saving" @click="emit('close')">
        Cancel
      </button>
      <button
        class="btn btn-primary"
        type="submit"
        form="workout-set-form"
        :disabled="saving || !availableExercises.length || !availableEquipment.length"
      >
        <span v-if="saving" class="spinner" />
        {{ editing ? 'Save changes' : 'Log set' }}
      </button>
    </template>
  </BaseModal>
</template>

<style scoped>
.set-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.preview-strip {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 12px 16px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--surface-2);
}

.preview-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.preview-item span {
  color: var(--muted);
  font-size: var(--text-xs);
  font-weight: 500;
}

.preview-item strong {
  color: var(--text);
  font-family: var(--font-mono);
  font-size: 1rem;
  font-weight: 600;
}

.preview-item small {
  color: var(--muted);
  font-size: 0.75rem;
  font-weight: 500;
}

.preview-divider {
  width: 1px;
  height: 30px;
  background: var(--divider);
}

.preview-strip p {
  margin-left: auto;
  color: var(--muted);
  font-size: var(--text-xs);
}

.prefill-note {
  align-items: center;
}

.previous-hint strong {
  color: var(--text-soft);
  font-weight: 500;
}

@media (max-width: 620px) {
  .preview-strip {
    gap: 12px;
    flex-wrap: wrap;
  }

  .preview-strip p {
    width: 100%;
    margin-left: 0;
  }
}
</style>
