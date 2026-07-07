<template>
  <div class="panel">
    <div class="panel-header">
      <span>事件日志</span>
      <span class="sub">最近 30 条 · tick {{ tick }}</span>
    </div>
    <div class="panel-body event-list">
      <div v-if="!events.length" style="color: var(--text-3); padding: 20px; text-align: center;">
        暂无事件
      </div>
      <div v-for="(e, i) in [...events].reverse()" :key="i" class="row">
        <span class="t">t{{ e.tick }}</span>
        <span class="k" :class="e.kind">{{ kindLabel(e.kind) }}</span>
        <span style="flex: 1;">
          <b v-if="e.patient_name">{{ e.patient_name }}</b>
          <span style="color: var(--text-2);"> {{ e.detail }}</span>
        </span>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'

const props = defineProps({ snapshot: Object })
const events = computed(() => props.snapshot?.events || [])
const tick = computed(() => props.snapshot?.tick || 0)

function kindLabel(k) {
  return ({
    arrive: '入队',
    assign: '分配',
    preempt: '抢占',
    suspend: '挂起',
    resume: '恢复',
    finish: '完成',
    deteriorate: '恶化'
  })[k] || k
}
</script>
