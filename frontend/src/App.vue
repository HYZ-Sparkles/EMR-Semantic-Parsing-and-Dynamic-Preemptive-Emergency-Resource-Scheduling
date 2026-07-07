<template>
  <div class="app-shell">
    <div class="app-header">
      <h1>电子病历语义解析与急诊医疗资源动态抢占调度</h1>
      <div class="meta">
        大顶堆 + 字典树 + 资源抢占 · 实时仿真
      </div>
    </div>
    <div class="app-body">
      <!-- 左列：录入 + 解析 -->
      <div class="col-stack" style="overflow: auto;">
        <SemanticPanel :snapshot="snapshot" @admitted="onChanged" />
        <ControlBar :snapshot="snapshot" @changed="onChanged" />
      </div>

      <!-- 中列：排队看板 -->
      <div class="col-stack" style="overflow: hidden;">
        <QueueDashboard :snapshot="snapshot" />
      </div>

      <!-- 右列：资源 + 日志 -->
      <div class="col-stack" style="overflow: auto;">
        <ResourceGrid :snapshot="snapshot" />
        <EventLog :snapshot="snapshot" />
      </div>
    </div>
  </div>
</template>

<script setup>
import { onMounted, onBeforeUnmount, ref } from 'vue'
import { getSnapshot, openSnapshotSocket } from './api.js'
import SemanticPanel from './components/SemanticPanel.vue'
import QueueDashboard from './components/QueueDashboard.vue'
import ResourceGrid from './components/ResourceGrid.vue'
import EventLog from './components/EventLog.vue'
import ControlBar from './components/ControlBar.vue'

const snapshot = ref({
  tick: 0, queue: [], patients: [], resources: [], events: [],
  auto_running: false, tick_ms: 2000
})

let ws = null
async function refresh() {
  snapshot.value = await getSnapshot()
}
async function onChanged() { /* WS 会推送, 这里空操作 */ }

onMounted(async () => {
  await refresh()
  ws = openSnapshotSocket((msg) => { snapshot.value = msg })
})
onBeforeUnmount(() => { if (ws) ws.close() })
</script>
