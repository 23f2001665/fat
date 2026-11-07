<script setup>
import { ref } from 'vue'
import { toast } from 'vue3-toastify'
import BaseModal from './BaseModal.vue'
import { changePassword } from '@/api/auth'

const props = defineProps({
  open: { type: Boolean, default: false },
})

const emit = defineEmits(['close'])

const saving = ref(false)
const currentPassword = ref('')
const newPassword = ref('')
const confirmPassword = ref('')

function reset() {
  currentPassword.value = ''
  newPassword.value = ''
  confirmPassword.value = ''
}

function close() {
  if (saving.value) return
  reset()
  emit('close')
}

async function submit() {
  if (saving.value) return
  if (!currentPassword.value) {
    toast.error('Enter your current password.')
    return
  }
  if (newPassword.value.length < 8) {
    toast.error('The new password must be at least 8 characters.')
    return
  }
  if (newPassword.value !== confirmPassword.value) {
    toast.error('The new passwords do not match.')
    return
  }
  if (currentPassword.value === newPassword.value) {
    toast.error('The new password must be different.')
    return
  }

  saving.value = true
  try {
    await changePassword(currentPassword.value, newPassword.value)
    toast.success('Password changed')
    reset()
    emit('close')
  } catch (error) {
    toast.error(error.userMessage || 'Could not change the password.')
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <BaseModal :open="open" title="Change password" subtitle="Account security" size="sm" @close="close">
    <form id="password-form" class="password-form" @submit.prevent="submit">
      <label class="field">
        <span class="field-label">Current password</span>
        <input
          v-model="currentPassword"
          class="input"
          type="password"
          autocomplete="current-password"
          data-autofocus
          required
        />
      </label>

      <label class="field">
        <span class="field-label">New password</span>
        <input
          v-model="newPassword"
          class="input"
          type="password"
          autocomplete="new-password"
          minlength="8"
          maxlength="128"
          required
        />
        <span class="field-hint">At least 8 characters.</span>
      </label>

      <label class="field">
        <span class="field-label">Confirm new password</span>
        <input
          v-model="confirmPassword"
          class="input"
          type="password"
          autocomplete="new-password"
          minlength="8"
          maxlength="128"
          required
        />
      </label>

      <div class="notice">
        <span>You will stay signed in after changing your password.</span>
      </div>
    </form>

    <template #footer>
      <button class="btn btn-secondary" type="button" :disabled="saving" @click="close">
        Cancel
      </button>
      <button class="btn btn-primary" type="submit" form="password-form" :disabled="saving">
        <span v-if="saving" class="spinner" />
        Change password
      </button>
    </template>
  </BaseModal>
</template>

<style scoped>
.password-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
</style>
