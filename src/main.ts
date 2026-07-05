import { createApp } from 'vue'
import App from './App.vue'
import SettingsPlaceholder from './components/SettingsPlaceholder.vue'
import './styles/main.css'

const isSettingsWindow = new URLSearchParams(window.location.search).get('window') === 'settings'

createApp(isSettingsWindow ? SettingsPlaceholder : App).mount('#app')
