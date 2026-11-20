<script setup>
import { computed, ref, watch } from 'vue'
import { toast } from 'vue3-toastify'
import BaseModal from './BaseModal.vue'
import Icon from './Icon.vue'
import { addMeasurement } from '@/api/profile'
import { formatNumber, todayISO } from '@/utils/format'

const props = defineProps({
  open: { type: Boolean, default: false },
})

const emit = defineEmits(['close', 'saved'])

const today = todayISO()
const saving = ref(false)
const form = ref(emptyForm())

const girthFields = [
  { key: 'waist_cm', label: 'Waist' },
  { key: 'chest_cm', label: 'Chest' },
  { key: 'hip_cm', label: 'Hip' },
  { key: 'arm_cm', label: 'Arm' },
  { key: 'thigh_cm', label: 'Thigh' },
]

function emptyForm() {
  return {
    measured_at: today,
    weight_kg: '',
    height_cm: '',
    body_fat_percentage: '',
    waist_cm: '',
    chest_cm: '',
    hip_cm: '',
    arm_cm: '',
    thigh_cm: '',
    notes: '',
  }
}

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) form.value = emptyForm()
  },
)

const bmiValue = computed(() => {
  const weight = optionalNumber(form.value.weight_kg)
  const height = optionalNumber(form.value.height_cm)
  if (!weight || !height || weight <= 0 || height <= 0) return null
  const heightM = height / 100
  return weight / (heightM * heightM)
})

const bmiLabel = computed(() => {
  const value = bmiValue.value
  if (value === null) return ''
  if (value < 18.5) return 'Underweight'
  if (value < 25) return 'Healthy range'
  if (value < 30) return 'Overweight'
  return 'Obese range'
})

function optionalNumber(value) {
  return value === '' || value === null || value === undefined ? null : Number(value)
}

function validate() {
  if (!form.value.measured_at) {
    toast.error('Choose a measurement date.')
    return false
  }
  if (form.value.measured_at > today) {
    toast.error('A measurement cannot be recorded for a future date.')
    return false
  }
  const weight = optionalNumber(form.value.weight_kg)
  if (weight !== null && (Number.isNaN(weight) || weight <= 0 || weight > 1000)) {
    toast.error('Weight must be greater than 0 and at most 1000 kg.')
    return false
  }
  const height = optionalNumber(form.value.height_cm)
  if (height !== null && (Number.isNaN(height) || height <= 0 || height > 300)) {
    toast.error('Height must be greater than 0 and at most 300 cm.')
    return false
  }
  const bodyFat = optionalNumber(form.value.body_fat_percentage)
  if (bodyFat !== null && (Number.isNaN(bodyFat) || bodyFat < 0 || bodyFat > 100)) {
    toast.error('Body-fat percentage must be between 0 and 100.')
    return false
  }
  for (const field of girthFields) {
    const value = optionalNumber(form.value[field.key])
    if (value !== null && (Number.isNaN(value) || value < 0 || value > 500)) {
      toast.error(`${field.label} must be between 0 and 500 cm.`)
      return false
    }
  }
  const hasValue = [
    form.value.weight_kg,
    form.value.height_cm,
    form.value.body_fat_percentage,
    ...girthFields.map((field) => form.value[field.key]),
  ].some((value) => value !== '' && value !== null && value !== undefined)
  if (!hasValue && !form.value.notes.trim()) {
    toast.error('Enter at least one measurement or a note.')
    return false
  }
  return true
}

async function submit() {
  if (saving.value || !validate()) return
  saving.value = true
  try {
    const measurement = await addMeasurement({
      measured_at: form.value.measured_at,
      weight_kg: optionalNumber(form.value.weight_kg),
      height_cm: optionalNumber(form.value.height_cm),
      body_fat_percentage: optionalNumber(form.value.body_fat_percentage),
      waist_cm: optionalNumber(form.value.waist_cm),
      chest_cm: optionalNumber(form.value.chest_cm),
      hip_cm: optionalNumber(form.value.hip_cm),
      arm_cm: optionalNumber(form.value.arm_cm),
      thigh_cm: optionalNumber(form.value.thigh_cm),
      notes: form.value.notes,
    })
    toast.success('Measurement snapshot added')
    emit('saved', measurement)
    emit('close')
  } catch (error) {
    toast.error(error.userMessage || 'Could not save the measurement.')
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <BaseModal
    :open="open"
    title="New measurement snapshot"
    subtitle="Append-only body data"
    size="lg"
    @close="emit('close')"
  >
    <form id="measurement-form" class="measurement-form" @submit.prevent="submit">
      <div class="form-grid two">
        <label class="field">
          <span class="field-label">Date</span>
          <input
            v-model="form.measured_at"
            class="input"
            type="date"
            :max="today"
            min="1900-01-01"
            required
          />
        </label>

        <div class="snapshot-note">
          <Icon name="info" :size="15" />
          <span>Only fill in the values you actually measured.</span>
        </div>
      </div>

      <div class="form-grid three">
        <label class="field">
          <span class="field-label">Weight <span class="optional">kg</span></span>
          <input
            v-model.number="form.weight_kg"
            class="input"
            type="number"
            inputmode="decimal"
            min="0.1"
            max="1000"
            step="0.1"
            placeholder="Optional"
          />
        </label>

        <label class="field">
          <span class="field-label">Height <span class="optional">cm</span></span>
          <input
            v-model.number="form.height_cm"
            class="input"
            type="number"
            inputmode="decimal"
            min="1"
            max="300"
            step="0.1"
            placeholder="Optional"
          />
        </label>

        <label class="field">
          <span class="field-label">Body fat <span class="optional">%</span></span>
          <input
            v-model.number="form.body_fat_percentage"
            class="input"
            type="number"
            inputmode="decimal"
            min="0"
            max="100"
            step="0.1"
            placeholder="Optional"
          />
        </label>
      </div>

      <div v-if="bmiValue !== null" class="bmi-preview">
        <div>
          <span>BMI from this snapshot</span>
          <strong>{{ formatNumber(bmiValue, 1) }}</strong>
        </div>
        <span class="badge badge-accent">{{ bmiLabel }}</span>
      </div>

      <div class="girth-section">
        <div class="girth-heading">
          <h3>Body measurements</h3>
          <span>All optional · centimetres</span>
        </div>
        <div class="form-grid three">
          <label v-for="field in girthFields" :key="field.key" class="field">
            <span class="field-label">{{ field.label }}</span>
            <input
              v-model.number="form[field.key]"
              class="input"
              type="number"
              inputmode="decimal"
              min="0"
              max="500"
              step="0.1"
              placeholder="Optional"
            />
          </label>
        </div>
      </div>

      <label class="field">
        <span class="field-label">Notes <span class="optional">optional</span></span>
        <textarea
          v-model="form.notes"
          class="textarea"
          rows="3"
          maxlength="4000"
          placeholder="Morning fasted, post-holiday, hydration, how you felt…"
        />
      </label>

      <div class="notice">
        <Icon name="shield" :size="16" class="notice-icon" />
        <span>
          Snapshots are append-only. BMI is only derived when weight and height are recorded
          together in the same snapshot.
        </span>
      </div>
    </form>

    <template #footer>
      <button class="btn btn-secondary" type="button" :disabled="saving" @click="emit('close')">
        Cancel
      </button>
      <button class="btn btn-primary" type="submit" form="measurement-form" :disabled="saving">
        <span v-if="saving" class="spinner" />
        Append snapshot
      </button>
    </template>
  </BaseModal>
</template>

<style scoped>
.measurement-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.snapshot-note {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--surface-2);
  color: var(--muted);
  font-size: 0.75rem;
}

.snapshot-note :deep(svg) {
  flex: 0 0 auto;
  color: var(--info);
}

.bmi-preview {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 16px;
  border: 1px solid var(--accent-line);
  border-radius: 12px;
  background: var(--accent-soft);
}

.bmi-preview div {
  display: flex;
  align-items: baseline;
  gap: 8px;
}

.bmi-preview span {
  color: var(--muted);
  font-size: 0.75rem;
  font-weight: 600;
}

.bmi-preview strong {
  color: var(--text);
  font-family: var(--font-mono);
  font-size: 1.125rem;
  font-weight: 600;
}

.girth-section {
  padding: 16px;
  border: 1px solid var(--border);
  border-radius: 13px;
  background: var(--surface-2);
}

.girth-heading {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
}

.girth-heading h3 {
  font-size: 0.8125rem;
}

.girth-heading span {
  color: var(--muted-2);
  font-size: 0.75rem;
}
</style>
