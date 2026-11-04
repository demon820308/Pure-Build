const t1 = performance.timeOrigin + performance.now()
;(window as unknown as { __PBL_T1__?: number }).__PBL_T1__ = t1

import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'

import '@unocss/reset/tailwind.css'
import 'virtual:uno.css'
import './styles/theme.css'

const app = createApp(App)
app.use(createPinia())
app.mount('#app')
