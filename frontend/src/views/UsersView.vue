<script setup>
import { computed, onMounted, ref } from 'vue'
import EmptyState from '@/components/EmptyState.vue'
import Icon from '@/components/Icon.vue'
import PageHeader from '@/components/PageHeader.vue'
import StatCard from '@/components/StatCard.vue'
import UserModal from '@/components/UserModal.vue'
import { getUsers } from '@/api/users'
import { authState, updateCurrentUser } from '@/stores/auth'
import { initials } from '@/utils/format'

const users = ref([])
const loading = ref(true)
const errorMessage = ref('')
const modalOpen = ref(false)
const editingUser = ref(null)

const currentUserId = computed(() => authState.user?.id)
const activeUsers = computed(() => users.value.filter((user) => user.is_active))
const inactiveUsers = computed(() => users.value.filter((user) => !user.is_active))
const admins = computed(() =>
  users.value.filter((user) => user.role === 'admin' && user.is_active),
)

function openCreate() {
  editingUser.value = null
  modalOpen.value = true
}

function openEdit(user) {
  editingUser.value = user
  modalOpen.value = true
}

async function loadUsers({ silent = false } = {}) {
  if (!silent) loading.value = true
  errorMessage.value = ''
  try {
    users.value = await getUsers()
  } catch (error) {
    errorMessage.value = error.userMessage || 'Could not load user accounts.'
  } finally {
    loading.value = false
  }
}

function handleSaved(saved) {
  updateCurrentUser(saved)
  loadUsers({ silent: true })
}

onMounted(() => loadUsers())
</script>

<template>
  <div class="page">
    <PageHeader
      eyebrow="Administration"
      title="Users"
      description="Manage who can sign in to this tracker. Members keep their own workouts and measurements; administrators also maintain the shared library."
    >
      <template #actions>
        <button class="btn btn-secondary" type="button" :disabled="loading" @click="loadUsers()">
          <Icon name="refresh" :size="16" />
          Refresh
        </button>
        <button class="btn btn-primary" type="button" @click="openCreate">
          <Icon name="userPlus" :size="16" />
          New user
        </button>
      </template>
    </PageHeader>

    <div v-if="errorMessage" class="notice notice-danger">
      <Icon name="alert" :size="18" class="notice-icon" />
      <span>{{ errorMessage }}</span>
      <button class="btn btn-sm btn-secondary retry-button" type="button" @click="loadUsers()">
        Retry
      </button>
    </div>

    <div v-if="loading" class="summary-grid">
      <div v-for="index in 4" :key="index" class="card skeleton summary-skeleton" />
    </div>

    <div v-else class="summary-grid">
      <StatCard
        label="Accounts"
        :value="users.length"
        icon="users"
        hint="Sign-in identities on this server"
        compact
      />
      <StatCard
        label="Active"
        :value="activeUsers.length"
        icon="checkCircle"
        tone="info"
        hint="Can sign in today"
        compact
      />
      <StatCard
        label="Administrators"
        :value="admins.length"
        icon="shield"
        tone="violet"
        hint="Can manage library and accounts"
        compact
      />
      <StatCard
        label="Inactive"
        :value="inactiveUsers.length"
        icon="alert"
        tone="warning"
        hint="History retained, sign-in blocked"
        compact
      />
    </div>

    <section class="users-card card">
      <header class="users-card-header">
        <div>
          <h2>Account directory</h2>
          <p>
            Accounts are deactivated rather than deleted, so every historical set and
            measurement keeps its owner.
          </p>
        </div>
        <span class="badge">{{ users.length }} total</span>
      </header>

      <div v-if="users.length" class="table-wrap users-table-wrap">
        <table class="data-table users-table">
          <thead>
            <tr>
              <th>Account</th>
              <th>Username</th>
              <th>Role</th>
              <th>Status</th>
              <th class="actions-heading">Actions</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="user in users" :key="user.id">
              <td>
                <div class="user-cell">
                  <span class="user-avatar" :class="{ admin: user.role === 'admin' }">
                    {{ initials(user.name) }}
                  </span>
                  <div class="user-copy">
                    <strong>{{ user.name }}</strong>
                    <span v-if="user.id === currentUserId" class="badge badge-accent you-badge">
                      You
                    </span>
                  </div>
                </div>
              </td>
              <td><span class="mono username">@{{ user.username }}</span></td>
              <td>
                <span
                  class="badge"
                  :class="user.role === 'admin' ? 'badge-accent' : 'badge-muted'"
                >
                  <Icon :name="user.role === 'admin' ? 'shield' : 'profile'" :size="12" />
                  {{ user.role === 'admin' ? 'Administrator' : 'Member' }}
                </span>
              </td>
              <td>
                <span
                  class="badge"
                  :class="user.is_active ? 'badge-info' : 'badge-danger'"
                >
                  {{ user.is_active ? 'Active' : 'Inactive' }}
                </span>
              </td>
              <td class="actions-cell">
                <button
                  class="btn btn-ghost btn-icon btn-sm"
                  type="button"
                  :aria-label="`Edit ${user.name}`"
                  title="Edit account"
                  @click="openEdit(user)"
                >
                  <Icon name="edit" :size="15" />
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <EmptyState
        v-else
        icon="users"
        title="No accounts yet"
        message="Create the first account to let someone sign in."
      >
        <button class="btn btn-primary" type="button" @click="openCreate">
          <Icon name="userPlus" :size="16" />
          New user
        </button>
      </EmptyState>
    </section>

    <div class="notice users-note">
      <Icon name="info" :size="17" class="notice-icon" />
      <span>
        At least one active administrator must always remain. Passwords can be reset from
        the edit dialog, and users keep their own training history when deactivated.
      </span>
    </div>

    <UserModal
      :open="modalOpen"
      :user="editingUser"
      @close="modalOpen = false"
      @saved="handleSaved"
    />
  </div>
</template>

<style scoped>
.retry-button {
  margin-left: auto;
}

.summary-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 13px;
}

.summary-skeleton {
  height: 105px;
}

.users-card {
  overflow: hidden;
}

.users-card-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 18px;
  padding: 21px 22px;
  border-bottom: 1px solid var(--border);
}

.users-card-header h2 {
  font-size: 1.08rem;
}

.users-card-header p {
  max-width: 660px;
  margin-top: 6px;
  color: var(--muted);
  font-size: 0.78rem;
  line-height: 1.6;
}

.users-table-wrap {
  border: 0;
  border-radius: 0;
}

.users-table th,
.users-table td {
  padding-right: 22px;
  padding-left: 22px;
}

.users-table td {
  vertical-align: middle;
}

.user-cell {
  display: flex;
  align-items: center;
  gap: 11px;
}

.user-avatar {
  display: grid;
  width: 35px;
  height: 35px;
  flex: 0 0 auto;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--surface-2);
  color: var(--info);
  font-family: var(--font-mono);
  font-size: 0.66rem;
  font-weight: 650;
  letter-spacing: 0.03em;
  place-items: center;
}

.user-avatar.admin {
  border-color: var(--accent-line);
  background: var(--accent-soft);
  color: var(--accent);
}

.user-copy {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
}

.user-copy strong {
  color: var(--text);
  font-size: 0.84rem;
  font-weight: 650;
}

.you-badge {
  min-height: 20px;
  padding: 1px 7px;
  font-size: 0.6rem;
}

.username {
  color: var(--muted);
  font-size: 0.76rem;
}

.actions-heading,
.actions-cell {
  width: 76px;
  text-align: right;
}

.users-note {
  align-items: center;
}

@media (max-width: 1000px) {
  .summary-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 760px) {
  .summary-grid {
    gap: 9px;
  }

  .users-card-header {
    flex-direction: column;
    padding: 17px;
  }

  .users-table th,
  .users-table td {
    padding-right: 14px;
    padding-left: 14px;
  }
}
</style>
