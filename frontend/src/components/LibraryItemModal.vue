<script setup>
import { computed, ref, watch } from 'vue'
import { toast } from 'vue3-toastify'
import BaseModal from './BaseModal.vue'
import ToggleSwitch from './ToggleSwitch.vue'
import {
  createEquipment,
  createMuscleGroup,
  updateEquipment,
  updateMuscleGroup,
} from '@/api/library'

const props = defineProps({
  open: { type: Boolean, default: false },
  kind: { type: String, default: 'muscle' },
  item: { type: Object, default: null },
})

const emit = defineEmits(['close', 'saved'])

const saving = ref(false)
const form = ref(emptyForm())

function emptyForm() {
  return {
    name: '',
    detail: '',
    is_active: true,
  }
}

const editing = computed(() => Boolean(props.item))
const isMuscle = computed(() => props.kind === 'muscle')
const reserved = computed(() => Boolean(props.item?.is_reserved))
const title = computed(() => {
  const noun = isMuscle.value ? 'muscle group' : 'equipment'
  return `${editing.value ? 'Edit' : 'New'} ${noun}`
})

watch(
  () => props.open,
  (isOpen) => {
    if (!isOpen) return
    if (props.item) {
      form.value = {
        name: props.item.name,
        detail: isMuscle.value
          ? props.item.description || ''
          : props.item.category || '',
        is_active: Boolean(props.item.is_active),
      }
      return
    }
    form.value = emptyForm()
  },
  { immediate: true },
)

watch(
  () => props.kind,
  () => {
    if (props.open) form.value = emptyForm()
  },
)

function close() {
  if (saving.value) return
  emit('close')
}

async function submit() {
  if (saving.value) return
  if (!form.value.name.trim()) {
    toast.error(`${isMuscle.value ? 'Muscle group' : 'Equipment'} name is required.`)
    return
  }
  saving.value = true
  try {
    let saved
    if (isMuscle.value) {
      const payload = {
        name: form.value.name,
        description: form.value.detail,
        is_active: Boolean(form.value.is_active),
      }
      saved = props.item
        ? await updateMuscleGroup(props.item.id, payload)
        : await createMuscleGroup(payload)
    } else {
      const payload = {
        name: form.value.name,
        category: form.value.detail,
        is_active: Boolean(form.value.is_active),
      }
      saved = props.item
        ? await updateEquipment(props.item.id, payload)
        : await createEquipment(payload)
    }
    toast.success(editing.value ? 'Library item updated' : 'Library item created')
    emit('saved', saved)
    emit('close')
  } catch (error) {
    toast.error(error.userMessage || 'Could not save this library item.')
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <BaseModal :open="open" :title="title" subtitle="Library catalogue" @close="close">
    <form id="library-item-form" class="item-form" @submit.prevent="submit">
      <label class="field">
        <span class="field-label">
          {{ isMuscle ? 'Muscle group name' : 'Equipment name' }}
        </span>
        <input
          v-model="form.name"
          class="input"
          type="text"
          maxlength="120"
          :placeholder="isMuscle ? 'e.g. Obliques' : 'e.g. Trap Bar'"
          :disabled="reserved"
          data-autofocus
          required
        />
        <span v-if="reserved" class="field-hint">
          The reserved No Equipment record must keep its name.
        </span>
      </label>

      <label class="field">
        <span class="field-label">
          {{ isMuscle ? 'Description' : 'Category' }}
          <span class="optional">optional</span>
        </span>
        <input
          v-model="form.detail"
          class="input"
          type="text"
          maxlength="4000"
          :placeholder="isMuscle ? 'What this muscle group covers' : 'e.g. Free weights'"
        />
      </label>

      <div class="active-block">
        <ToggleSwitch
          v-if="editing"
          v-model="form.is_active"
          :label="isMuscle ? 'Muscle group is active' : 'Equipment is active'"
          :disabled="reserved"
        />
        <p v-else class="field-hint new-item-hint">New library items are created active.</p>
        <p class="field-hint">
          <template v-if="reserved">
            No Equipment is reserved, stays active, and cannot be removed from the catalogue.
          </template>
          <template v-else-if="isMuscle">
            A muscle group used by an exercise cannot be deactivated until it is removed from
            those exercise definitions.
          </template>
          <template v-else>
            Inactive equipment stays on historical sets but cannot be selected for new ones.
          </template>
        </p>
      </div>
    </form>

    <template #footer>
      <button class="btn btn-secondary" type="button" :disabled="saving" @click="close">
        Cancel
      </button>
      <button class="btn btn-primary" type="submit" form="library-item-form" :disabled="saving">
        <span v-if="saving" class="spinner" />
        {{ editing ? 'Save changes' : 'Create' }}
      </button>
    </template>
  </BaseModal>
</template>

<style scoped>
.item-form {
  display: flex;
  flex-direction: column;
  gap: 17px;
}

.active-block {
  padding: 13px 14px;
  border: 1px solid var(--border);
  border-radius: 11px;
  background: var(--surface-2);
}

.active-block .field-hint {
  margin-top: 8px;
}
</style>
