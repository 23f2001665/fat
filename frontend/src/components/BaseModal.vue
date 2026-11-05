<script setup>
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import Icon from './Icon.vue'

const props = defineProps({
  open: { type: Boolean, default: false },
  title: { type: String, default: '' },
  subtitle: { type: String, default: '' },
  size: { type: String, default: 'md' },
})

const emit = defineEmits(['close'])
const panel = ref(null)
let previousOverflow = ''

function close() {
  emit('close')
}

function onKeydown(event) {
  if (event.key === 'Escape' && props.open) close()
}

async function focusPanel() {
  await nextTick()
  if (!panel.value) return
  const target = panel.value.querySelector(
    '[data-autofocus], input:not([type="hidden"]), select, textarea, button',
  )
  target?.focus()
}

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) {
      previousOverflow = document.body.style.overflow
      document.body.style.overflow = 'hidden'
      focusPanel()
    } else {
      document.body.style.overflow = previousOverflow || ''
    }
  },
)

onMounted(() => window.addEventListener('keydown', onKeydown))
onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown)
  if (props.open) document.body.style.overflow = previousOverflow || ''
})
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div
        v-if="open"
        class="modal-backdrop"
        role="presentation"
        @click.self="close"
      >
        <div
          ref="panel"
          class="modal-panel"
          :class="`size-${size}`"
          role="dialog"
          aria-modal="true"
          :aria-label="title || 'Dialog'"
        >
          <header class="modal-header">
            <div class="modal-heading">
              <p v-if="subtitle" class="modal-subtitle">{{ subtitle }}</p>
              <h2>{{ title }}</h2>
            </div>
            <button class="btn btn-ghost btn-icon" type="button" aria-label="Close dialog" @click="close">
              <Icon name="close" :size="18" />
            </button>
          </header>

          <div class="modal-body">
            <slot />
          </div>

          <footer v-if="$slots.footer" class="modal-footer">
            <slot name="footer" />
          </footer>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.modal-backdrop {
  position: fixed;
  inset: 0;
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 22px;
  background: rgba(4, 7, 11, 0.74);
  backdrop-filter: blur(8px);
}

.modal-panel {
  display: flex;
  flex-direction: column;
  width: min(100%, 600px);
  max-height: min(88vh, 900px);
  overflow: hidden;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-xl);
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.025), transparent 25%),
    var(--surface);
  box-shadow: 0 30px 100px rgba(0, 0, 0, 0.55);
}

.modal-panel.size-sm {
  width: min(100%, 450px);
}

.modal-panel.size-lg {
  width: min(100%, 760px);
}

.modal-panel.size-xl {
  width: min(100%, 940px);
}

.modal-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  padding: 23px 24px 17px;
  border-bottom: 1px solid var(--border);
}

.modal-heading {
  min-width: 0;
}

.modal-subtitle {
  margin-bottom: 5px;
  color: var(--accent);
  font-size: 0.66rem;
  font-weight: 750;
  letter-spacing: 0.15em;
  text-transform: uppercase;
}

.modal-header h2 {
  font-size: 1.35rem;
}

.modal-body {
  flex: 1;
  padding: 22px 24px;
  overflow-y: auto;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding: 16px 24px 20px;
  border-top: 1px solid var(--border);
  background: rgba(11, 15, 20, 0.28);
}

.modal-enter-active,
.modal-leave-active {
  transition: opacity 180ms ease;
}

.modal-enter-active .modal-panel,
.modal-leave-active .modal-panel {
  transition: transform 220ms cubic-bezier(0.22, 1, 0.36, 1);
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-from .modal-panel,
.modal-leave-to .modal-panel {
  transform: translateY(14px) scale(0.985);
}

@media (max-width: 700px) {
  .modal-backdrop {
    align-items: flex-end;
    padding: 0;
  }

  .modal-panel,
  .modal-panel.size-sm,
  .modal-panel.size-lg,
  .modal-panel.size-xl {
    width: 100%;
    max-height: 92vh;
    border-right: 0;
    border-bottom: 0;
    border-left: 0;
    border-radius: 22px 22px 0 0;
  }

  .modal-header {
    padding: 20px 18px 15px;
  }

  .modal-body {
    padding: 18px;
  }

  .modal-footer {
    padding: 14px 18px calc(16px + env(safe-area-inset-bottom));
  }

  .modal-footer :deep(.btn) {
    flex: 1;
  }
}
</style>
