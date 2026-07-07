<template>
  <div class="panel">
    <div class="panel-header">
      <span>医疗资源状态</span>
      <span class="sub">
        <span class="dot idle"></span>空闲
        <span class="dot busy"></span>治疗
        <span class="dot preempt"></span>抢占
      </span>
    </div>
    <div class="panel-body">
      <div
        v-for="(group, idx) in groups"
        :key="idx"
        class="resource-section"
      >
        <h4>{{ group.title }} · {{ group.list.length }} 台</h4>
        <div class="resource-grid">
          <div
            v-for="r in group.list"
            :key="r.id"
            class="resource-cell"
            :class="stateClass(r)"
          >
            <div class="label">{{ r.label }}</div>
            <div class="who">
              <template v-if="r.state === 'Idle'">
                <i>空闲</i>
              </template>
              <template v-else>
                <div>{{ patientName(r.current_patient) }}</div>
                <div v-if="r.preempted_by" style="opacity: 0.85;">
                  抢占者：{{ patientName(r.preempted_by) }}
                </div>
              </template>
            </div>
            <div v-if="r.state !== 'Idle' && r.suspended.length" class="susp">
              挂起：{{ r.suspended.map(s => s.name).join('、') }}
            </div>
            <div v-if="r.state !== 'Idle'" class="progress">
              <span :style="{ width: progressPct(r) + '%' }"></span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'

const props = defineProps({ snapshot: Object })

const KIND_NAME = {
  RescueRoom: '抢救室',
  CtScanner: 'CT 室',
  OperatingRoom: '手术间',
  Ultrasound: '超声室',
}

const groups = computed(() => {
  const list = props.snapshot?.resources || []
  const buckets = new Map()
  for (const r of list) {
    if (!buckets.has(r.kind)) buckets.set(r.kind, [])
    buckets.get(r.kind).push(r)
  }
  // 按固定顺序输出
  return ['RescueRoom', 'CtScanner', 'OperatingRoom', 'Ultrasound']
    .filter(k => buckets.has(k))
    .map(k => ({ title: KIND_NAME[k], list: buckets.get(k) }))
})

function stateClass(r) {
  if (r.state === 'Idle') return 'idle'
  if (r.state === 'Preempted') return 'preempted'
  return 'treating'
}

function progressPct(r) {
  if (!r.total_ticks) return 0
  return Math.max(0, Math.min(100, ((r.total_ticks - r.remaining_ticks) / r.total_ticks) * 100))
}

function patientName(pid) {
  if (!pid) return '-'
  const p = (props.snapshot?.patients || []).find(x => x.id === pid)
  return p ? p.name : pid.slice(0, 4)
}
</script>

<style scoped>
.dot {
  display: inline-block;
  width: 8px; height: 8px;
  border-radius: 50%;
  margin: 0 2px 0 8px;
  vertical-align: middle;
}
.dot.idle   { background: var(--idle-fg); }
.dot.busy   { background: var(--busy-fg); }
.dot.preempt{ background: var(--preempt-fg); }
</style>
