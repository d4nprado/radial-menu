<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'

function closeWindow() {
  void invoke('hide_settings')
}
</script>

<template>
  <main class="settings-placeholder">
    <section class="settings-placeholder__card">
      <h1>Configurar launcher</h1>
      <p>Configuração será implementada na próxima etapa</p>
      <button type="button" @click="closeWindow">Fechar</button>
    </section>
  </main>
</template>

<style scoped>
.settings-placeholder {
  display: grid;
  padding: 40px;
  place-items: center;
  color: #202231;
  background: #f4f5f9;
}

.settings-placeholder__card {
  width: min(100%, 440px);
  padding: 36px;
  border: 1px solid #dfe1ea;
  border-radius: 16px;
  background: #fff;
  box-shadow: 0 16px 40px rgb(35 39 58 / 10%);
}

h1 {
  margin: 0;
  font-size: 26px;
}

p {
  margin: 14px 0 28px;
  color: #626678;
  line-height: 1.5;
}

button {
  padding: 10px 18px;
  border: 0;
  border-radius: 9px;
  color: #fff;
  background: #6254d9;
  cursor: pointer;
}

button:hover {
  background: #5143c6;
}

button:focus-visible {
  outline: 3px solid rgb(98 84 217 / 28%);
  outline-offset: 2px;
}
</style>
