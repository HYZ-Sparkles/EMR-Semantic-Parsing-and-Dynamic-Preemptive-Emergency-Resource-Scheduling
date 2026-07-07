<template>
  <div class="panel">
    <div class="panel-header">
      <span>急诊排队实时看板</span>
      <span class="sub">大顶堆 · {{ queue.length }} 人在队 · 时刻 {{ tick }}</span>
    </div>
    <div class="panel-body">
      <div v-if="queue.length === 0" class="empty">
        <p style="color: var(--text-3); text-align: center; padding: 30px 0;">
          当前队列为空
        </p>
      </div>
      <transition-group name="flip" tag="div">
        <div
          v-for="(p, i) in queue"
          :key="p.id"
          class="queue-item"
          :class="{ top: i === 0 }"
        >
          <div class="rank">#{{ i + 1 }}</div>
          <div style="flex: 1; min-width: 0;">
            <div style="display: flex; align-items: center; gap: 6px;">
              <span class="name">{{ p.name }}</span>
              <el-tag size="small" :type="tagType(p)" effect="plain">
                {{ kindLabel(p.desired_kind) }}
              </el-tag>
            </div>
            <div class="complaint">{{ p.complaint }}</div>
            <div class="meta-row">
              <span>等待 {{ p.ticks_waited }} tick</span>
              <span v-if="p.matched.length">
                命中 {{ p.matched.length }} 个医学词
              </span>
            </div>
          </div>
          <div class="score">{{ p.score }}</div>
        </div>
      </transition-group>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'

const props = defineProps({ snapshot: Object })

const queue = computed(() => props.snapshot?.queue || [])
const tick = computed(() => props.snapshot?.tick || 0)

function kindLabel(k) {
  return ({
    RescueRoom: '抢救室',
    CtScanner: 'CT',
    OperatingRoom: '手术间',
    Ultrasound: '超声'
  })[k] || '任意'
}

function tagType(p) {
  if (p.score >= 15) return 'danger'
  if (p.score >= 8)  return 'warning'
  return 'info'
}
</script>

<style>
.flip-move { transition: transform 0.4s ease; }
.empty { color: var(--text-3); }
</style>
