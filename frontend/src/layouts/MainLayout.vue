<script setup>
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { toast } from 'vue3-toastify'
import Icon from '@/components/Icon.vue'
import { authState, isAdmin, logout } from '@/stores/auth'
import { initials } from '@/utils/format'

const route = useRoute()
const router = useRouter()

const navItems = computed(() => {
  const items = [
    { name: 'workout', label: 'Workout', icon: 'dumbbell', path: '/workout' },
    { name: 'history', label: 'History', icon: 'history', path: '/history' },
    { name: 'analytics', label: 'Analytics', icon: 'analytics', path: '/analytics' },
    { name: 'library', label: 'Library', icon: 'library', path: '/library' },
    { name: 'profile', label: 'Profile', icon: 'profile', path: '/profile' },
  ]
  if (isAdmin.value) {
    items.push({ name: 'users', label: 'Users', icon: 'users', path: '/users' })
  }
  return items
})

const authUser = computed(() => authState.user)
const roleLabel = computed(() =>
  authState.user?.role === 'admin' ? 'Admin' : 'Member',
)
const avatarInitials = computed(() => initials(authState.user?.name))

function isActive(item) {
  return route.name === item.name
}

async function handleLogout() {
  await logout()
  toast.success('Signed out')
  await router.replace({ name: 'login' })
}
</script>

<template>
  <div class="app-shell">
    <aside class="sidebar">
      <RouterLink to="/workout" class="brand" aria-label="FAT home">
        <span class="brand-mark">
          <Icon name="dumbbell" :size="21" />
        </span>
        <span class="brand-copy">
          <strong>FAT</strong>
          <small>Workout tracker</small>
        </span>
      </RouterLink>

      <div class="sidebar-label">Navigate</div>
      <nav class="main-nav" aria-label="Main navigation">
        <RouterLink
          v-for="item in navItems"
          :key="item.name"
          :to="item.path"
          class="nav-link"
          :class="{ active: isActive(item) }"
        >
          <span class="nav-icon">
            <Icon :name="item.icon" :size="19" />
          </span>
          <span>{{ item.label }}</span>
          <span class="nav-dot" />
        </RouterLink>
      </nav>

      <div class="sidebar-spacer" />

      <div class="account-card">
        <span class="account-avatar" :class="{ admin: isAdmin }">
          {{ avatarInitials }}
        </span>
        <div class="account-copy">
          <strong>{{ authUser?.name || 'Account' }}</strong>
          <span>@{{ authUser?.username }} · {{ roleLabel }}</span>
        </div>
        <button
          class="btn btn-ghost btn-icon account-logout"
          type="button"
          aria-label="Sign out"
          title="Sign out"
          @click="handleLogout"
        >
          <Icon name="logout" :size="17" />
        </button>
      </div>
      <p class="version-line">FAT 2.0 · Flask + Vue</p>
    </aside>

    <div class="mobile-bar">
      <RouterLink to="/workout" class="brand mobile-brand" aria-label="FAT home">
        <span class="brand-mark">
          <Icon name="dumbbell" :size="18" />
        </span>
        <span class="brand-copy">
          <strong>FAT</strong>
          <small>Workout tracker</small>
        </span>
      </RouterLink>
      <div class="mobile-account">
        <span class="mobile-avatar" :class="{ admin: isAdmin }">{{ avatarInitials }}</span>
        <div class="mobile-account-copy">
          <strong>{{ authUser?.name }}</strong>
          <span>{{ roleLabel }}</span>
        </div>
        <button
          class="btn btn-ghost btn-icon btn-sm"
          type="button"
          aria-label="Sign out"
          title="Sign out"
          @click="handleLogout"
        >
          <Icon name="logout" :size="16" />
        </button>
      </div>
    </div>

    <div class="app-main">
      <main class="content">
        <RouterView v-slot="{ Component }">
          <Transition name="view" mode="out-in">
            <component :is="Component" :key="route.name" />
          </Transition>
        </RouterView>
      </main>
    </div>

    <nav
      class="bottom-nav"
      aria-label="Mobile navigation"
      :style="{ gridTemplateColumns: `repeat(${navItems.length}, 1fr)` }"
    >
      <RouterLink
        v-for="item in navItems"
        :key="item.name"
        :to="item.path"
        class="bottom-link"
        :class="{ active: isActive(item) }"
      >
        <Icon :name="item.icon" :size="20" />
        <span>{{ item.label }}</span>
      </RouterLink>
    </nav>
  </div>
</template>

<style scoped>
.app-shell {
  min-height: 100vh;
}

.sidebar {
  position: fixed;
  top: 0;
  bottom: 0;
  left: 0;
  z-index: 50;
  display: flex;
  width: var(--sidebar-width);
  flex-direction: column;
  padding: 25px 18px 20px;
  border-right: 1px solid var(--border);
  background:
    linear-gradient(180deg, rgba(94, 234, 212, 0.035), transparent 28%),
    rgba(10, 14, 20, 0.88);
  backdrop-filter: blur(18px);
}

.brand {
  display: inline-flex;
  align-items: center;
  gap: 11px;
  padding: 3px 7px 3px 3px;
  color: var(--text);
}

.brand-mark {
  display: grid;
  width: 39px;
  height: 39px;
  border: 1px solid var(--accent-line);
  border-radius: 12px;
  background: var(--accent-soft);
  color: var(--accent);
  place-items: center;
  box-shadow: inset 0 0 18px rgba(94, 234, 212, 0.04);
}

.brand-copy {
  display: flex;
  flex-direction: column;
  line-height: 1.15;
}

.brand-copy strong {
  font-size: 1.15rem;
  font-weight: 750;
  letter-spacing: 0.08em;
}

.brand-copy small {
  margin-top: 4px;
  color: var(--muted-2);
  font-size: 0.62rem;
  font-weight: 550;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

.sidebar-label {
  margin: 37px 12px 11px;
  color: var(--muted-2);
  font-size: 0.61rem;
  font-weight: 700;
  letter-spacing: 0.15em;
  text-transform: uppercase;
}

.main-nav {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.nav-link {
  position: relative;
  display: flex;
  align-items: center;
  gap: 12px;
  min-height: 45px;
  padding: 8px 11px;
  border: 1px solid transparent;
  border-radius: 11px;
  color: var(--muted);
  font-size: 0.86rem;
  font-weight: 550;
  transition: all 160ms ease;
}

.nav-link:hover {
  border-color: var(--border);
  background: rgba(255, 255, 255, 0.018);
  color: var(--text-soft);
}

.nav-link.active {
  border-color: var(--accent-line);
  background: var(--accent-soft);
  color: var(--accent);
}

.nav-icon {
  display: grid;
  width: 25px;
  height: 25px;
  place-items: center;
}

.nav-dot {
  width: 5px;
  height: 5px;
  margin-left: auto;
  border-radius: 50%;
  background: transparent;
}

.nav-link.active .nav-dot {
  background: var(--accent);
  box-shadow: 0 0 0 4px rgba(94, 234, 212, 0.08);
}

.sidebar-spacer {
  flex: 1;
}

.account-card {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 11px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: rgba(23, 33, 45, 0.5);
}

.account-avatar {
  display: grid;
  width: 34px;
  height: 34px;
  flex: 0 0 auto;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--surface-2);
  color: var(--info);
  font-family: var(--font-mono);
  font-size: 0.64rem;
  font-weight: 650;
  letter-spacing: 0.03em;
  place-items: center;
}

.account-avatar.admin {
  border-color: var(--accent-line);
  background: var(--accent-soft);
  color: var(--accent);
}

.account-copy {
  display: flex;
  min-width: 0;
  flex: 1;
  flex-direction: column;
  line-height: 1.25;
}

.account-copy strong {
  overflow: hidden;
  color: var(--text-soft);
  font-size: 0.76rem;
  font-weight: 650;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.account-copy span {
  overflow: hidden;
  margin-top: 2px;
  color: var(--muted-2);
  font-size: 0.62rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.account-logout {
  flex: 0 0 auto;
  color: var(--muted-2);
}

.version-line {
  margin-top: 16px;
  color: #526177;
  font-family: var(--font-mono);
  font-size: 0.59rem;
  text-align: center;
}

.app-main {
  width: calc(100% - var(--sidebar-width));
  min-height: 100vh;
  margin-left: var(--sidebar-width);
}

.content {
  width: 100%;
  max-width: var(--content-max);
  margin: 0 auto;
  padding: 35px 38px 60px;
}

.mobile-bar,
.bottom-nav {
  display: none;
}

.view-enter-active,
.view-leave-active {
  transition:
    opacity 150ms ease,
    transform 150ms ease;
}

.view-enter-from {
  opacity: 0;
  transform: translateY(5px);
}

.view-leave-to {
  opacity: 0;
  transform: translateY(-3px);
}

@media (max-width: 1100px) {
  :root {
    --sidebar-width: 225px;
  }

  .content {
    padding-right: 26px;
    padding-left: 26px;
  }
}

@media (max-width: 900px) {
  .sidebar {
    display: none;
  }

  .app-main {
    width: 100%;
    margin-left: 0;
  }

  .mobile-bar {
    position: sticky;
    top: 0;
    z-index: 40;
    display: flex;
    height: 63px;
    align-items: center;
    justify-content: space-between;
    padding: 10px 18px;
    border-bottom: 1px solid var(--border);
    background: rgba(11, 15, 20, 0.82);
    backdrop-filter: blur(18px);
  }

  .mobile-brand .brand-mark {
    width: 34px;
    height: 34px;
    border-radius: 10px;
  }

  .mobile-brand .brand-copy strong {
    font-size: 1rem;
  }

  .mobile-brand .brand-copy small {
    font-size: 0.56rem;
  }

  .mobile-account {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .mobile-avatar {
    display: grid;
    width: 30px;
    height: 30px;
    border: 1px solid var(--border);
    border-radius: 9px;
    background: var(--surface-2);
    color: var(--info);
    font-family: var(--font-mono);
    font-size: 0.58rem;
    font-weight: 650;
    place-items: center;
  }

  .mobile-avatar.admin {
    border-color: var(--accent-line);
    background: var(--accent-soft);
    color: var(--accent);
  }

  .mobile-account-copy {
    display: flex;
    max-width: 110px;
    flex-direction: column;
    line-height: 1.15;
    text-align: right;
  }

  .mobile-account-copy strong {
    overflow: hidden;
    color: var(--text-soft);
    font-size: 0.72rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mobile-account-copy span {
    margin-top: 2px;
    color: var(--muted-2);
    font-size: 0.58rem;
    font-weight: 600;
  }

  .content {
    max-width: 780px;
    padding: 24px 18px calc(98px + env(safe-area-inset-bottom));
  }

  .bottom-nav {
    position: fixed;
    right: 12px;
    bottom: calc(10px + env(safe-area-inset-bottom));
    left: 12px;
    z-index: 60;
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 3px;
    padding: 5px;
    border: 1px solid var(--border-strong);
    border-radius: 18px;
    background: rgba(18, 25, 35, 0.92);
    box-shadow: 0 18px 45px rgba(0, 0, 0, 0.42);
    backdrop-filter: blur(20px);
  }

  .bottom-link {
    display: flex;
    min-width: 0;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    padding: 7px 2px 5px;
    border-radius: 12px;
    color: var(--muted-2);
    font-size: 0.57rem;
    font-weight: 650;
    letter-spacing: 0.01em;
    transition: all 150ms ease;
  }

  .bottom-link span {
    overflow: hidden;
    max-width: 100%;
    text-overflow: ellipsis;
  }

  .bottom-link.active {
    background: var(--accent-soft);
    color: var(--accent);
  }
}

@media (max-width: 380px) {
  .content {
    padding-right: 12px;
    padding-left: 12px;
  }

  .bottom-link {
    font-size: 0.52rem;
  }
}
</style>
