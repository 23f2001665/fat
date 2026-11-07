import { createRouter, createWebHistory } from 'vue-router'
import { authState, clearAuth, fetchCurrentUser } from '@/stores/auth'

const MainLayout = () => import('@/layouts/MainLayout.vue')

const routes = [
  {
    path: '/register',
    name: 'register',
    component: () => import('@/views/LoginView.vue'),
    props: { mode: 'register' },
    meta: { title: 'Create account' },
  },
  {
    path: '/login',
    name: 'login',
    component: () => import('@/views/LoginView.vue'),
    props: { mode: 'login' },
    meta: { title: 'Sign in' },
  },
  {
    path: '/',
    component: MainLayout,
    children: [
      {
        path: '',
        redirect: '/workout',
      },
      {
        path: 'workout',
        name: 'workout',
        component: () => import('@/views/WorkoutView.vue'),
        meta: { title: 'Workout' },
      },
      {
        path: 'history',
        name: 'history',
        component: () => import('@/views/HistoryView.vue'),
        meta: { title: 'History' },
      },
      {
        path: 'analytics',
        name: 'analytics',
        component: () => import('@/views/AnalyticsView.vue'),
        meta: { title: 'Analytics' },
      },
      {
        path: 'library',
        name: 'library',
        component: () => import('@/views/LibraryView.vue'),
        meta: { title: 'Library' },
      },
      {
        path: 'profile',
        name: 'profile',
        component: () => import('@/views/ProfileView.vue'),
        meta: { title: 'Profile' },
      },
      {
        path: 'users',
        name: 'users',
        component: () => import('@/views/UsersView.vue'),
        meta: { title: 'Users', admin: true },
      },
    ],
  },
  {
    path: '/:pathMatch(.*)*',
    redirect: '/workout',
  },
]

const router = createRouter({
  history: createWebHistory(),
  routes,
  scrollBehavior() {
    return { top: 0, behavior: 'smooth' }
  },
})

router.beforeEach(async (to) => {
  if (!authState.loaded) {
    try {
      await fetchCurrentUser()
    } catch {
      clearAuth()
    }
  }

  if (to.name === 'login' || to.name === 'register') {
    if (authState.user) {
      const redirect =
        typeof to.query.redirect === 'string' && to.query.redirect.startsWith('/')
          ? to.query.redirect
          : '/workout'
      return redirect
    }
    return true
  }

  if (!authState.user) {
    return { name: 'login', query: { redirect: to.fullPath } }
  }

  if (to.meta.admin && authState.user.role !== 'admin') {
    return { name: 'workout' }
  }

  return true
})

router.afterEach((to) => {
  const title = to.meta?.title
  document.title = title ? `${title} · FAT` : 'FAT · Workout tracker'
})

export default router
