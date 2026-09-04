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
                <div>{{ patientName(r.current_patient_id) }}</div>
                <!-- 仅在 Preempted 态下展示“原患者被抢走”的信息 -->
                <div v-if="r.preempted_victim_id" style="opacity: 0.85;">
                  抢占自：{{ patientName(r.preempted_victim_id) }}
                </div>
              </template>
            </div>
            <div v-if="r.state !== 'Idle' && r.suspended?.length" class="susp">
              挂起：{{ suspendedNames(r).join('、') }}
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

// 患者 id -> 患者 的索引，避免 patientName 每次 O(n) 线性查找
const patientsById = computed(() => {
  const map = new Map()
  for (const p of props.snapshot?.patients || []) map.set(p.id, p)
  return map
})

function patientName(pid) {
  if (!pid) return '-'
  const p = patientsById.value.get(pid)
  return p ? p.name : pid.slice(0, 4)
}

// 挂起名单按"恢复优先级"展示（危重分高者在前，同分取最晚被抢占者），
// 与后端 ResourceSlot::pop_most_critical 的选人口径保持一致——
// 后端返回的数组是入列顺序，直接展示会和实际恢复顺序相反。
function suspendedNames(r) {
  return [...(r.suspended || [])]
    .sort((a, b) => b.score - a.score || b.preempted_at_tick - a.preempted_at_tick)
    .map(s => s.name)
}
</script>

<style scoped>
/* 表头图例的小圆点 */
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

/* 一类资源为一段（抢救室/CT/手术间/超声） */
.resource-section {
  margin-bottom: 18px;
}
.resource-section h4 {
  margin: 0 0 8px;
  font-size: 13px;
  color: var(--text-2);
  font-weight: 600;
  letter-spacing: 0.5px;
  text-transform: uppercase;
}
/* 每段内 2 列网格；改每行台数动 columns */
.resource-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 8px;
}
/* 单台资源卡片；底色随状态切换（见下三态） */
.resource-cell {
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 10px;
  background: var(--idle);
  color: var(--idle-fg);
  font-size: 12px;
  position: relative;
  min-height: 90px;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
}
.resource-cell.idle { background: var(--idle); color: var(--idle-fg); }
.resource-cell.treating { background: var(--busy); color: var(--busy-fg); }
.resource-cell.preempted {          /* 被抢占：红底 + 闪烁提示 */
  background: var(--preempt);
  color: var(--preempt-fg);
  animation: blink 1.2s ease-in-out infinite;
}
@keyframes blink {                  /* 抢占态呼吸边框 */
  0%, 100% { box-shadow: 0 0 0 0 var(--preempt-glow-0); }
  50%      { box-shadow: 0 0 0 3px var(--preempt-glow-50); }
}
.resource-cell .label {             /* 资源名（如 CT-1） */
  font-weight: 600;
  font-size: 13px;
}
.resource-cell .who {               /* 当前占用患者 */
  margin-top: 4px;
  font-size: 12px;
  line-height: 1.4;
  word-break: break-all;
}
.resource-cell .susp {              /* 挂起排队名单 */
  margin-top: 4px;
  font-size: 11px;
  color: var(--text-2);
  border-top: 1px dashed currentColor;
  padding-top: 4px;
}
.resource-cell .progress {          /* 治疗进度条外框 */
  margin-top: 6px;
  height: 4px;
  background: var(--on-primary-40);   /* 资源卡底色之上的半透槽 */
  border-radius: 2px;
  overflow: hidden;
}
.resource-cell .progress > span {   /* 进度条填充，宽度由内联 style 控制 */
  display: block;
  height: 100%;
  background: currentColor;
  opacity: 0.6;
  transition: width 0.3s linear;
}
</style>
