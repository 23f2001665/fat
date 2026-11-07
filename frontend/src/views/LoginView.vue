<script setup>
import { ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { toast } from 'vue3-toastify'
import Icon from '@/components/Icon.vue'
import { login } from '@/stores/auth'

const route = useRoute()
const router = useRouter()

const username = ref('')
const password = ref('')
const submitting = ref(false)
const errorMessage = ref('')

function destination() {
  const redirect = route.query.redirect
  if (
    typeof redirect === 'string' &&
    redirect.startsWith('/') &&
    !redirect.startsWith('//') &&
    redirect !== '/login'
  ) {
    return redirect
  }
  return '/workout'
}

async function submit() {
  if (submitting.value) return
  errorMessage.value = ''
  if (!username.value.trim() || !password.value) {
    errorMessage.value = 'Enter your username and password.'
    return
  }

  submitting.value = true
  try {
    const user = await login(username.value.trim(), password.value)
    toast.success(`Welcome back, ${user.name}`)
    await router.replace(destination())
  } catch (error) {
    errorMessage.value =
      error.userMessage || 'Could not sign in. Check your credentials and try again.'
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
        <p class="eyebrow">Private training log</p>
        <h1>Sign in to FAT</h1>
        <p>
          Your workouts, measurements and progress are tied to your own account.
        </p>
      </div>

      <form class="login-form" @submit.prevent="submit">
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
          <input
            v-model="password"
            class="input"
            type="password"
            autocomplete="current-password"
            placeholder="Your password"
            required
          />
        </label>

        <div v-if="errorMessage" class="notice notice-danger login-error" role="alert">
          <Icon name="alert" :size="18" class="notice-icon" />
          <span>{{ errorMessage }}</span>
        </div>

        <button class="btn btn-primary btn-block login-submit" type="submit" :disabled="submitting">
          <span v-if="submitting" class="spinner" />
          <Icon v-else name="shield" :size="18" />
          {{ submitting ? 'Signing in…' : 'Sign in' }}
        </button>
      </form>

      <p class="login-footnote">
        Accounts are managed by an administrator. Contact them if you need access or a
        password reset.
      </p>
    </section>

    <aside class="login-aside" aria-hidden="true">
      <div class="aside-glow" />
      <p class="eyebrow">Built for consistent training</p>
      <h2>Log the work. Watch the trend.</h2>
      <p class="aside-copy">
        Every account keeps its own sets, body measurements and analytics. The shared exercise
        library stays consistent across the whole server.
      </p>

      <div class="aside-features">
        <div class="aside-feature">
          <span class="feature-icon"><Icon name="dumbbell" :size="18" /></span>
          <div>
            <strong>Fast set logging</strong>
            <p>Weights, complete and partial reps, rest and RIR.</p>
          </div>
        </div>
        <div class="aside-feature">
          <span class="feature-icon"><Icon name="analytics" :size="18" /></span>
          <div>
            <strong>Honest analytics</strong>
            <p>Charts built only from records you actually logged.</p>
          </div>
        </div>
        <div class="aside-feature">
          <span class="feature-icon"><Icon name="scale" :size="18" /></span>
          <div>
            <strong>Body snapshots</strong>
            <p>Append-only measurements with BMI from the same snapshot.</p>
          </div>
        </div>
      </div>
    </aside>
  </main>
</template>

<style scoped>
.login-page {
  display: grid;
  grid-template-columns: minmax(0, 440px) minmax(0, 520px);
  align-items: center;
  justify-content: center;
  gap: 66px;
  min-height: 100vh;
  padding: 40px 28px;
}

.login-card {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 25px;
  padding: 30px;
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
  gap: 11px;
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
  font-size: 1.2rem;
  font-weight: 750;
  letter-spacing: 0.08em;
}

.brand-copy small {
  margin-top: 4px;
  color: var(--muted-2);
  font-size: 0.62rem;
  font-weight: 550;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.login-heading h1 {
  margin-bottom: 9px;
  font-size: clamp(1.7rem, 4vw, 2.15rem);
}

.login-heading > p:last-child {
  color: var(--muted);
  font-size: 0.86rem;
  line-height: 1.6;
}

.login-form {
  display: flex;
  flex-direction: column;
  gap: 15px;
}

.login-error {
  align-items: center;
}

.login-submit {
  margin-top: 4px;
}

.login-footnote {
  padding-top: 18px;
  border-top: 1px solid var(--border);
  color: var(--muted-2);
  font-size: 0.68rem;
  line-height: 1.6;
}

.login-aside {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 34px 0;
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
  font-size: 0.9rem;
  line-height: 1.7;
}

.aside-features {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 13px;
  margin-top: 17px;
}

.aside-feature {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  max-width: 430px;
  padding: 13px 15px;
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
  font-size: 0.8rem;
  font-weight: 650;
}

.aside-feature p {
  margin-top: 3px;
  color: var(--muted-2);
  font-size: 0.72rem;
  line-height: 1.55;
}

@media (max-width: 980px) {
  .login-page {
    grid-template-columns: minmax(0, 440px);
    gap: 0;
  }

  .login-aside {
    display: none;
  }
}

@media (max-width: 520px) {
  .login-page {
    padding: 22px 14px;
  }

  .login-card {
    padding: 22px;
  }
}
</style>
