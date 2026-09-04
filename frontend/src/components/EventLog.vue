<template>
  <div class="panel">
    <div class="panel-header">
      <span>事件日志</span>
      <span class="sub">最近 30 条 · tick {{ tick }}</span>
    </div>
    <div class="panel-body event-list">
      <div v-if="!events.length" style="color: var(--text-3); padding: 80px; text-align: center;font-size: 14px;">
        暂无事件
      </div>
      <div v-for="(e, i) in reversedEvents" :key="i" class="row">
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
// 倒序展示（最新在上）；用 computed 缓存，避免每次重渲都复制并反转数组
const reversedEvents = computed(() => events.value.slice().reverse())
const tick = computed(() => props.snapshot?.tick || 0)

function kindLabel(k) {
  return ({
    arrive: '入队',
    pending: '登记',
    assign: '分配',
    preempt: '抢占',
    suspend: '挂起',
    resume: '恢复',
    finish: '完成',
    deteriorate: '恶化',
    reject: '拒收'
  })[k] || k
}
</script>

<style scoped>
/* 日志用等宽字体，整体次级灰 */
.event-list {
  font-size: 12px;
  font-family: ui-monospace, "Cascadia Code", Consolas, monospace;
  color: var(--text-2);
}
/* 每条：t时刻 | 类型徽章 | 内容 */
.event-list .row {
  padding: 6px 8px;
  border-bottom: 1px solid var(--border);
  display: flex;
  gap: 6px;
}
.event-list .row .t {          /* 时刻列 */
  color: var(--text-3);
  flex-shrink: 0;
}
.event-list .row .k {          /* 事件类型徽章，颜色按 kind 区分（见下） */
  font-weight: 700;
  width: 76px;
  flex-shrink: 0;
  text-align: center;
  border-radius: 3px;
  padding: 1px 0;
  color: var(--on-primary);
  font-size: 11px;
}
.event-list .row .k.arrive    { background: var(--primary); }
.event-list .row .k.pending   { background: var(--info); }
.event-list .row .k.assign    { background: var(--success); }
.event-list .row .k.preempt   { background: var(--danger); }
.event-list .row .k.suspend   { background: var(--danger-2); }
.event-list .row .k.resume    { background: var(--info); }
.event-list .row .k.finish    { background: var(--text-2); }
.event-list .row .k.deteriorate{ background: var(--warning); }
</style>
