import { computed, reactive } from 'vue'
import {
  getCurrentUser,
  login as loginRequest,
  logout as logoutRequest,
  register as registerRequest,
} from '@/api/auth'

export const authState = reactive({
  user: null,
  loaded: false,
})

export const isAuthenticated = computed(() => Boolean(authState.user))
export const isAdmin = computed(() => authState.user?.role === 'admin')

let inflight = null

export async function fetchCurrentUser({ force = false } = {}) {
  if (authState.loaded && !force) return authState.user
  if (inflight) return inflight

  inflight = getCurrentUser()
    .then((user) => {
      authState.user = user
      return user
    })
    .catch((error) => {
      if (error.response?.status === 401) {
        authState.user = null
        return null
      }
      throw error
    })
    .finally(() => {
      authState.loaded = true
      inflight = null
    })

  return inflight
}

export async function login(username, password) {
  const user = await loginRequest(username, password)
  authState.user = user
  authState.loaded = true
  return user
}

export async function register(name, username, password) {
  const user = await registerRequest(name, username, password)
  authState.user = user
  authState.loaded = true
  return user
}

export async function logout() {
  try {
    await logoutRequest()
  } catch {
    // The session may already be gone; clearing local state is still correct.
  }
  clearAuth()
}

export function clearAuth() {
  authState.user = null
  authState.loaded = true
}

export function updateCurrentUser(user) {
  if (authState.user?.id === user.id) authState.user = user
}
