import { createApp } from 'vue'
import App from './App.vue'
import ConfigWindow from './components/ConfigWindow.vue'
import './styles/main.css'

const isSettingsWindow = new URLSearchParams(window.location.search).get('window') === 'settings'

createApp(isSettingsWindow ? ConfigWindow : App).mount('#app')
