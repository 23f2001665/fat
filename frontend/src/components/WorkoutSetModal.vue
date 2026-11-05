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
    const firstExercise = availableExercises.value[0]
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

      <div class="form-grid two">
        <label class="field">
          <span class="field-label">Exercise</span>
          <select v-model="form.exercise_id" class="select" required>
            <option disabled value="">Choose an exercise</option>
            <option v-for="exercise in availableExercises" :key="exercise.id" :value="exercise.id">
              {{ exercise.name }}{{ exercise.is_active ? '' : ' · inactive' }}
            </option>
          </select>
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
  gap: 18px;
}

.preview-strip {
  display: flex;
  align-items: center;
  gap: 18px;
  padding: 13px 15px;
  border: 1px solid var(--accent-line);
  border-radius: 12px;
  background: var(--accent-soft);
}

.preview-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.preview-item span {
  color: #8be9dc;
  font-size: 0.62rem;
  font-weight: 650;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

.preview-item strong {
  color: var(--text);
  font-family: var(--font-mono);
  font-size: 1.02rem;
  font-weight: 600;
}

.preview-item small {
  color: var(--muted);
  font-size: 0.68rem;
  font-weight: 500;
}

.preview-divider {
  width: 1px;
  height: 30px;
  background: var(--accent-line);
}

.preview-strip p {
  margin-left: auto;
  color: #9adfd7;
  font-size: 0.68rem;
}

@media (max-width: 620px) {
  .preview-strip {
    gap: 13px;
    flex-wrap: wrap;
  }

  .preview-strip p {
    width: 100%;
    margin-left: 0;
  }
}
</style>
