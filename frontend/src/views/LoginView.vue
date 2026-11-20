<script setup>
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { toast } from 'vue3-toastify'
import Icon from '@/components/Icon.vue'
import { login, register } from '@/stores/auth'

const props = defineProps({
  mode: { type: String, default: 'login' },
})

const route = useRoute()
const router = useRouter()

const username = ref('')
const name = ref('')
const password = ref('')
const showPassword = ref(false)
const submitting = ref(false)
const errorMessage = ref('')
const isRegister = computed(() => props.mode === 'register')

watch(isRegister, () => {
  errorMessage.value = ''
  password.value = ''
  showPassword.value = false
})

function destination() {
  const redirect = route.query.redirect
  if (
    typeof redirect === 'string' &&
    redirect.startsWith('/') &&
    !redirect.startsWith('//') &&
    redirect !== '/login' &&
    redirect !== '/register'
  ) {
    return redirect
  }
  return '/workout'
}

async function submit() {
  if (submitting.value) return
  errorMessage.value = ''
  if (isRegister.value && !name.value.trim()) {
    errorMessage.value = 'Enter your name.'
    return
  }
  if (!username.value.trim() || !password.value) {
    errorMessage.value = 'Enter your username and password.'
    return
  }
  if (isRegister.value && password.value.length < 8) {
    errorMessage.value = 'Choose a password with at least 8 characters.'
    return
  }

  submitting.value = true
  try {
    const user = isRegister.value
      ? await register(name.value.trim(), username.value.trim(), password.value)
      : await login(username.value.trim(), password.value)
    toast.success(
      isRegister.value ? `Welcome, ${user.name}` : `Welcome back, ${user.name}`,
    )
    await router.replace(destination())
  } catch (error) {
    errorMessage.value =
      error.userMessage ||
      (isRegister.value
        ? 'Could not create the account. Try a different username.'
        : 'Could not sign in. Check your credentials and try again.')
    password.value = ''
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <main class="login-page">
    <section class="login-card card">
      <div class="login-brand">
        <span class="brand-mark">
          <Icon name="dumbbell" :size="23" />
        </span>
        <span class="brand-copy">
          <strong>FAT</strong>
          <small>Workout tracker</small>
        </span>
      </div>

      <div class="login-heading">
        <h1>{{ isRegister ? 'Create your account' : 'Sign in to FAT' }}</h1>
        <p v-if="isRegister">Free, private, and takes under a minute.</p>
        <p v-else>Pick up where you left off.</p>
      </div>

      <form class="login-form" @submit.prevent="submit">
        <label v-if="isRegister" class="field">
          <span class="field-label">Name</span>
          <input
            v-model="name"
            class="input"
            type="text"
            autocomplete="name"
            placeholder="Your display name"
            required
          />
        </label>

        <label class="field">
          <span class="field-label">Username</span>
          <input
            v-model="username"
            class="input"
            type="text"
            autocomplete="username"
            autocapitalize="none"
            spellcheck="false"
            placeholder="your username"
            data-autofocus
            required
          />
        </label>

        <label class="field">
          <span class="field-label">Password</span>
          <span class="password-field">
            <input
              v-model="password"
              class="input"
              :type="showPassword ? 'text' : 'password'"
              :autocomplete="isRegister ? 'new-password' : 'current-password'"
              :placeholder="isRegister ? 'At least 8 characters' : 'Your password'"
              required
            />
            <button
              class="btn btn-ghost btn-icon btn-sm password-toggle"
              type="button"
              :aria-label="showPassword ? 'Hide password' : 'Show password'"
              :aria-pressed="showPassword"
              @click="showPassword = !showPassword"
            >
              <Icon :name="showPassword ? 'eyeOff' : 'eye'" :size="16" />
            </button>
          </span>
        </label>

        <div v-if="errorMessage" class="notice notice-danger login-error" role="alert">
          <Icon name="alert" :size="18" class="notice-icon" />
          <span>{{ errorMessage }}</span>
        </div>

        <button class="btn btn-primary btn-block login-submit" type="submit" :disabled="submitting">
          <span v-if="submitting" class="spinner" />
          <Icon v-else :name="isRegister ? 'userPlus' : 'shield'" :size="18" />
          <template v-if="submitting">
            {{ isRegister ? 'Creating account…' : 'Signing in…' }}
          </template>
          <template v-else>{{ isRegister ? 'Create account' : 'Sign in' }}</template>
        </button>
      </form>

      <p v-if="isRegister" class="login-footnote">
        Already have an account?
        <RouterLink to="/login">Sign in</RouterLink>
      </p>
      <p v-else class="login-footnote">
        New here? <RouterLink to="/register">Create a free account</RouterLink>
      </p>
    </section>

    <aside class="login-aside" aria-label="What you get">
      <div class="aside-glow" aria-hidden="true" />
      <h2>Log the work. Watch the trend.</h2>
      <p class="aside-copy">
        Set a weekly goal, log sets in a few taps, and see progress built only from what you
        actually recorded.
      </p>

      <figure class="aside-preview" aria-label="Example of the weekly goal tracker">
        <figcaption>Example</figcaption>
        <div class="preview-top">
          <span>This week</span>
          <span><strong class="mono">2</strong> of 3 sessions</span>
        </div>
        <div class="preview-bars" aria-hidden="true">
          <span class="on" /><span class="on" /><span />
        </div>
        <p class="preview-message">
          <Icon name="flame" :size="14" />
          1 more session by Sunday to keep your 4-week streak.
        </p>
      </figure>

      <ul class="aside-features">
        <li class="aside-feature">
          <span class="feature-icon"><Icon name="repeat" :size="18" /></span>
          <div>
            <strong>Fast set logging</strong>
            <p>Repeat your last set in one tap, then tweak what changed.</p>
          </div>
        </li>
        <li class="aside-feature">
          <span class="feature-icon"><Icon name="analytics" :size="18" /></span>
          <div>
            <strong>Honest analytics</strong>
            <p>Charts built only from sets you logged.</p>
          </div>
        </li>
        <li class="aside-feature">
          <span class="feature-icon"><Icon name="scale" :size="18" /></span>
          <div>
            <strong>Body measurements</strong>
            <p>Weight, girths and BMI tracked over time.</p>
          </div>
        </li>
      </ul>
    </aside>
  </main>
</template>

<style scoped>
.login-page {
  display: grid;
  grid-template-columns: minmax(0, 440px) minmax(0, 520px);
  align-items: center;
  justify-content: center;
  gap: 64px;
  min-height: 100vh;
  padding: 40px 28px;
}

.login-card {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 24px;
  padding: 28px;
  overflow: hidden;
}

.login-card::before {
  position: absolute;
  top: -120px;
  left: -90px;
  width: 280px;
  height: 280px;
  border-radius: 50%;
  background: radial-gradient(circle, rgba(94, 234, 212, 0.12), transparent 68%);
  content: "";
  pointer-events: none;
}

.login-brand {
  position: relative;
  display: inline-flex;
  align-items: center;
  gap: 12px;
}

.brand-mark {
  display: grid;
  width: 43px;
  height: 43px;
  border: 1px solid var(--accent-line);
  border-radius: 13px;
  background: var(--accent-soft);
  color: var(--accent);
  place-items: center;
  box-shadow: inset 0 0 18px rgba(94, 234, 212, 0.05);
}

.brand-copy {
  display: flex;
  flex-direction: column;
  line-height: 1.15;
}

.brand-copy strong {
  font-size: 1.25rem;
  font-weight: 700;
  letter-spacing: 0.08em;
}

.brand-copy small {
  margin-top: 4px;
  color: var(--muted-2);
  font-size: 0.75rem;
  font-weight: 500;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.login-heading h1 {
  margin-bottom: 8px;
  font-size: clamp(1.7rem, 4vw, 2.15rem);
}

.login-heading > p:last-child {
  color: var(--muted);
  font-size: 0.875rem;
  line-height: 1.6;
}

.login-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.login-error {
  align-items: center;
}

.login-submit {
  margin-top: 4px;
}

.login-footnote {
  padding-top: 16px;
  border-top: 1px solid var(--divider);
  color: var(--muted);
  font-size: var(--text-sm);
  line-height: 1.6;
}

.login-footnote a {
  color: var(--accent);
  font-weight: 600;
  text-decoration: none;
}

.login-footnote a:hover {
  text-decoration: underline;
}

.login-aside {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 32px 0;
}

.aside-glow {
  position: absolute;
  top: 10px;
  left: -50px;
  width: 360px;
  height: 360px;
  border-radius: 50%;
  background: radial-gradient(circle, rgba(125, 211, 252, 0.08), transparent 68%);
  pointer-events: none;
}

.login-aside h2 {
  position: relative;
  max-width: 430px;
  font-size: clamp(1.8rem, 3.2vw, 2.55rem);
  letter-spacing: -0.035em;
}

.aside-copy {
  position: relative;
  max-width: 460px;
  color: var(--muted);
  font-size: 0.875rem;
  line-height: 1.7;
}

.aside-features {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-top: 16px;
}

.aside-feature {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  max-width: 430px;
  padding: 12px 16px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: rgba(23, 33, 45, 0.44);
}

.feature-icon {
  display: grid;
  width: 34px;
  height: 34px;
  flex: 0 0 auto;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--surface-2);
  color: var(--accent);
  place-items: center;
}

.aside-feature strong {
  display: block;
  color: var(--text-soft);
  font-size: 0.8125rem;
  font-weight: 600;
}

.aside-feature p {
  margin-top: 4px;
  color: var(--muted-2);
  font-size: 0.75rem;
  line-height: 1.55;
}

@media (max-width: 980px) {
  .login-page {
    grid-template-columns: minmax(0, 440px);
    align-content: start;
    gap: 32px;
  }

  /* Keep the value proposition visible on small screens, just more compact. */
  .login-aside {
    gap: 12px;
    padding: 0 4px;
  }

  .login-aside h2 {
    font-size: 1.25rem;
  }

  .aside-copy,
  .aside-glow {
    display: none;
  }

  .aside-features {
    margin-top: 0;
  }
}

@media (max-width: 520px) {
  .login-page {
    padding: 20px 12px;
  }

  .login-card {
    padding: 20px;
  }
}

.password-field {
  position: relative;
  display: block;
}

.password-field .input {
  padding-right: 44px;
}

.password-toggle {
  position: absolute;
  top: 50%;
  right: 4px;
  transform: translateY(-50%);
}

.password-toggle:hover:not(:disabled) {
  transform: translateY(-50%);
}

.aside-features {
  padding: 0;
  list-style: none;
}

.aside-preview {
  position: relative;
  display: flex;
  max-width: 430px;
  flex-direction: column;
  gap: 8px;
  margin: 8px 0 0;
  padding: 16px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface);
}

.aside-preview figcaption {
  position: absolute;
  top: -10px;
  right: 12px;
  padding: 0 8px;
  border: 1px solid var(--border);
  border-radius: 999px;
  background: var(--surface-2);
  color: var(--muted);
  font-size: var(--text-xs);
}

.preview-top {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  color: var(--muted);
  font-size: var(--text-sm);
}

.preview-top > span:first-child {
  color: var(--text);
  font-weight: 600;
}

.preview-top strong {
  color: var(--text);
  font-size: var(--text-lg);
}

.preview-bars {
  display: flex;
  gap: 4px;
}

.preview-bars span {
  height: 6px;
  flex: 1;
  border-radius: 999px;
  background: var(--surface-3);
}

.preview-bars span.on {
  background: var(--accent);
}

.preview-message {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--warning);
  font-size: var(--text-sm);
}
</style>
