import { createApp } from 'vue'
import Vue3Toastify, { toast } from 'vue3-toastify'
import 'vue3-toastify/dist/index.css'

import App from './App.vue'
import router from './router'
import { clearAuth } from './stores/auth'
import './assets/theme.css'

const app = createApp(App)

app.use(router)
app.use(Vue3Toastify, {
  autoClose: 3200,
  position: toast.POSITION.BOTTOM_RIGHT,
  theme: 'dark',
  closeOnClick: true,
  pauseOnHover: true,
})

window.addEventListener('fat:unauthorized', () => {
  const current = router.currentRoute.value
  clearAuth()
  if (current.name !== 'login') {
    router.replace({
      name: 'login',
      query: { redirect: current.fullPath },
    })
  }
})

app.mount('#app')
