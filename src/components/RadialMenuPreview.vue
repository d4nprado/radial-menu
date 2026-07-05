<script setup lang="ts">
import { computed } from 'vue'
import type { MenuItem } from '../types/menu'

const props = defineProps<{ items: MenuItem[]; selectedId: string | null }>()
defineEmits<{ select: [id: string]; add: [] }>()

const selected = computed(() => props.items.find((item) => item.id === props.selectedId))

function position(index: number) {
  const angle = (index / props.items.length) * Math.PI * 2 - Math.PI / 2
  return {
    '--x': `${Math.cos(angle) * 124}px`,
    '--y': `${Math.sin(angle) * 124}px`,
    '--accent': props.items[index].accent,
  }
}
</script>

<template>
  <section class="card preview">
    <header><div><small>VISUALIZAÇÃO</small><h2>Preview radial</h2></div><b>{{ items.length }} ações</b></header>
    <div class="ring">
      <i class="orbit outer" /><i class="orbit inner" />
      <button
        v-for="(item, index) in items"
        :key="item.id"
        class="item"
        :class="{ selected: item.id === selectedId }"
        :style="position(index)"
        :title="`${item.label} — ${item.hint}`"
        @click="$emit('select', item.id)"
      ><span>{{ item.icon }}</span></button>
      <div class="center"><strong>ORBIT</strong><span>{{ selected?.label ?? 'Selecione uma ação' }}</span></div>
    </div>
    <footer>
      <div><i :style="{ background: selected?.accent }" /><strong>{{ selected?.label ?? 'Nenhum item' }}</strong><small>{{ selected?.hint }}</small></div>
      <button class="add" @click="$emit('add')">＋ Adicionar ação</button>
    </footer>
  </section>
</template>

<style scoped>
.card{display:flex;min-height:0;padding:24px;flex-direction:column;border:1px solid #ffffff14;border-radius:22px;background:radial-gradient(circle at 50% 45%,#6855ff1f,transparent 39%),#0d101cd9;box-shadow:0 24px 70px #0004}
header,footer,footer div{display:flex;align-items:center}header{justify-content:space-between}header small{color:#8d82ff;font-size:10px;font-weight:700;letter-spacing:.14em}h2{margin:4px 0 0;font-size:20px}header b{padding:6px 9px;border-radius:999px;color:#8c91a8;background:#ffffff08;font-size:10px}
.ring{position:relative;width:340px;height:340px;margin:auto}.orbit{position:absolute;top:50%;left:50%;border:1px solid #ffffff14;border-radius:50%;transform:translate(-50%,-50%)}.outer{width:286px;height:286px;border-style:dashed}.inner{width:148px;height:148px;border-color:#8b7cff26}
.item{position:absolute;z-index:2;top:50%;left:50%;display:grid;width:62px;height:62px;padding:0;place-items:center;border:1px solid color-mix(in srgb,var(--accent) 35%,#ffffff14);border-radius:19px;color:color-mix(in srgb,var(--accent) 65%,white);background:linear-gradient(145deg,#25293df5,#0f121ffa);box-shadow:0 10px 24px #0005;cursor:pointer;transform:translate(calc(-50% + var(--x)),calc(-50% + var(--y)));transition:.15s}.item span{display:grid;width:36px;height:36px;place-items:center;border-radius:12px;background:color-mix(in srgb,var(--accent) 12%,transparent);font-size:12px;font-weight:700}.item:hover,.item.selected{border-color:color-mix(in srgb,var(--accent) 78%,white);box-shadow:0 0 0 4px color-mix(in srgb,var(--accent) 14%,transparent),0 12px 28px #0005;transform:translate(calc(-50% + var(--x)),calc(-50% + var(--y))) scale(1.09)}
.center{position:absolute;top:50%;left:50%;display:flex;width:122px;height:122px;padding:16px;align-items:center;justify-content:center;flex-direction:column;border:1px solid #ffffff1c;border-radius:50%;text-align:center;background:linear-gradient(145deg,#24283c,#0d101d);transform:translate(-50%,-50%)}.center strong{color:#b7afff;font-size:10px;letter-spacing:.18em}.center span{overflow:hidden;width:88px;margin-top:6px;color:#8f94aa;font-size:10px;text-overflow:ellipsis;white-space:nowrap}
footer{min-height:50px;padding-top:16px;justify-content:space-between;border-top:1px solid #ffffff12;gap:12px}footer div{min-width:0;gap:8px}footer div i{width:7px;height:7px;flex:none;border-radius:50%}footer strong{overflow:hidden;font-size:12px;text-overflow:ellipsis;white-space:nowrap}footer small{color:#73798f;font-size:10px}.add{padding:9px 13px;border:1px solid #8b7cff47;border-radius:10px;color:#bdb7ff;background:#8b7cff1a;cursor:pointer;white-space:nowrap}
</style>
