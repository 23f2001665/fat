<script setup>
import { computed, ref, watch } from 'vue'
import { toast } from 'vue3-toastify'
import BaseModal from './BaseModal.vue'
import ToggleSwitch from './ToggleSwitch.vue'
import { createUser, updateUser } from '@/api/users'
import { authState } from '@/stores/auth'

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
    username: '',
    password: '',
    role: 'user',
    is_active: true,
  }
}

const editing = computed(() => Boolean(props.user))
const isSelf = computed(() => props.user?.id === authState.user?.id)
const title = computed(() => (editing.value ? 'Edit user' : 'New user'))

watch(
  () => props.open,
  (isOpen) => {
    if (!isOpen) return
    if (props.user) {
      form.value = {
        name: props.user.name || '',
        username: props.user.username || '',
        password: '',
        role: props.user.role || 'user',
        is_active: Boolean(props.user.is_active),
      }
      return
    }
    form.value = emptyForm()
  },
  { immediate: true },
)

function close() {
  if (saving.value) return
  emit('close')
}

function validate() {
  if (!form.value.name.trim()) {
    toast.error('Name is required.')
    return false
  }
  if (!editing.value && !form.value.username.trim()) {
    toast.error('Username is required.')
    return false
  }
  if (!editing.value && form.value.password.length < 8) {
    toast.error('Password must be at least 8 characters.')
    return false
  }
  if (editing.value && form.value.password && form.value.password.length < 8) {
    toast.error('The new password must be at least 8 characters.')
    return false
  }
  return true
}

async function submit() {
  if (saving.value || !validate()) return
  saving.value = true
  try {
    let saved
    if (editing.value) {
      const payload = {
        name: form.value.name,
        role: form.value.role,
        is_active: form.value.is_active,
      }
      if (form.value.password) payload.password = form.value.password
      saved = await updateUser(props.user.id, payload)
      toast.success(`${saved.name} updated`)
    } else {
      saved = await createUser({
        name: form.value.name,
        username: form.value.username,
        password: form.value.password,
        role: form.value.role,
      })
      toast.success(`${saved.name} created`)
    }
    emit('saved', saved)
    emit('close')
  } catch (error) {
    toast.error(error.userMessage || 'Could not save the account.')
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <BaseModal
    :open="open"
    :title="title"
    subtitle="Account access"
    size="sm"
    @close="close"
  >
    <form id="user-form" class="user-form" @submit.prevent="submit">
      <label class="field">
        <span class="field-label">Name</span>
        <input
          v-model="form.name"
          class="input"
          type="text"
          maxlength="120"
          placeholder="Display name"
          data-autofocus
          required
        />
      </label>

      <label class="field">
        <span class="field-label">Username</span>
        <input
          v-model="form.username"
          class="input"
          type="text"
          maxlength="40"
          autocapitalize="none"
          spellcheck="false"
          placeholder="e.g. alex"
          :disabled="editing"
          :required="!editing"
        />
        <span class="field-hint">
          <template v-if="editing">Usernames cannot be changed after creation.</template>
          <template v-else>
            3–40 characters using letters, numbers, dots, hyphens or underscores.
          </template>
        </span>
      </label>

      <label class="field">
        <span class="field-label">
          {{ editing ? 'New password' : 'Password' }}
          <span v-if="editing" class="optional">optional</span>
        </span>
        <input
          v-model="form.password"
          class="input"
          type="password"
          autocomplete="new-password"
          :placeholder="editing ? 'Leave blank to keep the current password' : 'At least 8 characters'"
          :required="!editing"
        />
        <span class="field-hint">
          <template v-if="editing">
            Set a new password here if the user needs a reset.
          </template>
          <template v-else>At least 8 characters.</template>
        </span>
      </label>

      <label class="field">
        <span class="field-label">Role</span>
        <select v-model="form.role" class="select" :disabled="isSelf">
          <option value="user">Member — log personal workouts</option>
          <option value="admin">Administrator — manage library and users</option>
        </select>
      </label>

      <div class="active-row">
        <div>
          <span class="field-label">Account active</span>
          <p class="field-hint">
            Deactivated users cannot sign in, but their history is kept.
          </p>
        </div>
        <ToggleSwitch
          v-model="form.is_active"
          :disabled="isSelf"
          :label="form.is_active ? 'Active' : 'Inactive'"
        />
      </div>

      <div v-if="isSelf" class="notice">
        <span>You cannot demote or deactivate your own account.</span>
      </div>
    </form>

    <template #footer>
      <button class="btn btn-secondary" type="button" :disabled="saving" @click="close">
        Cancel
      </button>
      <button class="btn btn-primary" type="submit" form="user-form" :disabled="saving">
        <span v-if="saving" class="spinner" />
        {{ editing ? 'Save changes' : 'Create user' }}
      </button>
    </template>
  </BaseModal>
</template>

<style scoped>
.user-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.active-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 12px 12px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--surface-2);
}

.active-row .field-hint {
  margin-top: 4px;
}
</style>
