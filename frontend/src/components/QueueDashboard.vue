<template>
  <div class="panel">
    <div class="panel-header">
      <span>急诊排队实时看板</span>
      <span class="sub">大顶堆 · {{ queue.length }} 人在队 · 时刻 {{ tick }}</span>
    </div>
    <div class="panel-body">
      <div v-if="queue.length === 0" class="empty-state">
        当前队列为空
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
                {{ kindLabel(p.desired_kind, '任意') }}
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
import { kindLabel } from '../utils/kinds.js'

const props = defineProps({ snapshot: Object })

const queue = computed(() => props.snapshot?.queue || [])
const tick = computed(() => props.snapshot?.tick || 0)

function tagType(p) {
  if (p.score >= 15) return 'danger'
  if (p.score >= 8)  return 'warning'
  return 'info'
}
</script>

<style scoped>
/* 面板自身：作为 .col-stack 的 flex 子项占满中间列高度，
 * 内部 .panel-body 的 flex:1 + overflow:auto 接管滚动。
 * 组件自治，不依赖 App.vue 对它的 :deep 选择器。 */
.panel {
  flex: 1 1 auto;
  min-height: 0;
}

/* 队列条目：#排名 | 姓名+标签/主诉/等待信息 | 分数 */
.queue-item {
  display: flex;
  align-items: center;
  padding: 10px 12px;
  background: var(--queue-bg);
  border: 1px solid var(--border);
  border-left: 4px solid var(--text-3);  /* 左侧色条，队首会变橙 */
  border-radius: 6px;
  margin-bottom: 8px;
  transition: transform 0.2s, background 0.2s;
}
.queue-item.top {                 /* 队首（下一个被调度）高亮 */
  background: var(--queue-top);
  border-left-color: var(--warning);
}
.queue-item .rank {
  font-weight: 700;
  font-size: 16px;
  color: var(--text-2);
  width: 32px;
}
.queue-item.top .rank { color: var(--danger); }
.queue-item .name {
  font-weight: 600;
  flex: 1;
  margin-right: 8px;
}
.queue-item .score {              /* 右侧危重分，红色大字 */
  font-weight: 700;
  font-size: 16px;
  color: var(--danger);
  min-width: 36px;
  text-align: right;
}
.queue-item .complaint {          /* 主诉原文，灰色小字 */
  font-size: 12px;
  color: var(--text-3);
  width: 100%;
  margin-top: 2px;
}
.queue-item .meta-row {           /* 等待时长 / 命中词数 一行 */
  width: 100%;
  display: flex;
  justify-content: space-between;
  font-size: 12px;
  color: var(--text-2);
  margin-top: 2px;
}

/* 列表重排时的滑动动画（transition-group name="flip"） */
.flip-move { transition: transform 0.4s ease; }

/* 空态：垂直水平居中于面板里。改这里控制颜色/字号/留白 */
.empty-state {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  text-align: center;
  color: var(--text-3);
  font-size: 14px;
  letter-spacing: 0.5px;
  opacity: 0.85;             /* 比正文弱化一点 */
}
</style>
