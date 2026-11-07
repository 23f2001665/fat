<script setup>
import { computed, ref, watch } from 'vue'
import { toast } from 'vue3-toastify'
import BaseModal from './BaseModal.vue'
import { updateProfile } from '@/api/profile'
import { todayISO } from '@/utils/format'

const props = defineProps({
  open: { type: Boolean, default: false },
  user: { type: Object, default: null },
})

const emit = defineEmits(['close', 'saved'])

const saving = ref(false)
const form = ref(emptyForm())

function emptyForm() {
  return {
    name: '',
    height_cm: '',
    date_of_birth: '',
  }
}

const today = todayISO()
const editing = computed(() => Boolean(props.user))

watch(
  () => props.open,
  (isOpen) => {
    if (!isOpen) return
    if (props.user) {
      form.value = {
        name: props.user.name || '',
        height_cm: props.user.height_cm ?? '',
        date_of_birth: props.user.date_of_birth || '',
      }
      return
    }
    form.value = emptyForm()
  },
  { immediate: true },
)

function optionalNumber(value) {
  return value === '' || value === null || value === undefined ? null : Number(value)
}

function validate() {
  if (!form.value.name.trim()) {
    toast.error('Name is required.')
    return false
  }
  const height = optionalNumber(form.value.height_cm)
  if (height !== null && (Number.isNaN(height) || height < 50 || height > 300)) {
    toast.error('Height must be between 50 and 300 cm.')
    return false
  }
  if (form.value.date_of_birth && form.value.date_of_birth > today) {
    toast.error('Date of birth cannot be in the future.')
    return false
  }
  return true
}

async function submit() {
  if (saving.value || !validate()) return
  saving.value = true
  try {
    const user = await updateProfile({
      name: form.value.name,
      height_cm: optionalNumber(form.value.height_cm),
      date_of_birth: form.value.date_of_birth || null,
    })
    toast.success('Profile updated')
    emit('saved', user)
    emit('close')
  } catch (error) {
    toast.error(error.userMessage || 'Could not update the profile.')
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <BaseModal
    :open="open"
    title="Edit profile"
    subtitle="Your account"
    size="sm"
    @close="emit('close')"
  >
    <form id="profile-form" class="profile-form" @submit.prevent="submit">
      <label class="field">
        <span class="field-label">Name</span>
        <input
          v-model="form.name"
          class="input"
          type="text"
          maxlength="120"
          placeholder="Your name"
          data-autofocus
          required
        />
      </label>

      <label class="field">
        <span class="field-label">Height <span class="optional">cm</span></span>
        <input
          v-model.number="form.height_cm"
          class="input"
          type="number"
          inputmode="decimal"
          min="50"
          max="300"
          step="0.1"
          placeholder="Optional"
        />
        <span class="field-hint">
          Used as your profile height. BMI still uses weight and height from the same snapshot.
        </span>
      </label>

      <label class="field">
        <span class="field-label">Date of birth <span class="optional">optional</span></span>
        <input
          v-model="form.date_of_birth"
          class="input"
          type="date"
          :max="today"
          min="1900-01-01"
        />
      </label>

      <div class="notice">
        Changing your profile does not alter any historical measurement snapshot.
      </div>
    </form>

    <template #footer>
      <button class="btn btn-secondary" type="button" :disabled="saving" @click="emit('close')">
        Cancel
      </button>
      <button class="btn btn-primary" type="submit" form="profile-form" :disabled="saving">
        <span v-if="saving" class="spinner" />
        Save profile
      </button>
    </template>
  </BaseModal>
</template>

<style scoped>
.profile-form {
  display: flex;
  flex-direction: column;
  gap: 17px;
}
</style>
