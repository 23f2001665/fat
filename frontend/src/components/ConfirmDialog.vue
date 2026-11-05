<script setup>
import BaseModal from './BaseModal.vue'
import Icon from './Icon.vue'

defineProps({
  open: { type: Boolean, default: false },
  title: { type: String, default: 'Are you sure?' },
  message: { type: String, default: '' },
  confirmLabel: { type: String, default: 'Confirm' },
  cancelLabel: { type: String, default: 'Cancel' },
  danger: { type: Boolean, default: false },
  busy: { type: Boolean, default: false },
})

const emit = defineEmits(['close', 'confirm'])
</script>

<template>
  <BaseModal :open="open" :title="title" size="sm" @close="emit('close')">
    <div class="confirm-body">
      <span class="confirm-icon" :class="{ danger }">
        <Icon :name="danger ? 'alert' : 'info'" :size="22" />
      </span>
      <p>{{ message }}</p>
    </div>
    <template #footer>
      <button class="btn btn-secondary" type="button" :disabled="busy" @click="emit('close')">
        {{ cancelLabel }}
      </button>
      <button
        class="btn"
        :class="danger ? 'btn-danger' : 'btn-primary'"
        type="button"
        :disabled="busy"
        @click="emit('confirm')"
      >
        <span v-if="busy" class="spinner" />
        {{ confirmLabel }}
      </button>
    </template>
  </BaseModal>
</template>

<style scoped>
.confirm-body {
  display: flex;
  align-items: flex-start;
  gap: 14px;
  padding: 4px 0 6px;
}

.confirm-body p {
  padding-top: 3px;
  color: var(--muted);
  font-size: 0.9rem;
  line-height: 1.65;
}

.confirm-icon {
  display: grid;
  flex: 0 0 auto;
  width: 42px;
  height: 42px;
  border: 1px solid var(--accent-line);
  border-radius: 12px;
  background: var(--accent-soft);
  color: var(--accent);
  place-items: center;
}

.confirm-icon.danger {
  border-color: var(--danger-line);
  background: var(--danger-soft);
  color: var(--danger);
}
</style>
