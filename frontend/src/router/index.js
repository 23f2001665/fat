import { createRouter, createWebHistory } from 'vue-router'

const MainLayout = () => import('@/layouts/MainLayout.vue')

const routes = [
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

router.afterEach((to) => {
  const title = to.meta?.title
  document.title = title ? `${title} · FAT` : 'FAT · Workout tracker'
})

export default router
