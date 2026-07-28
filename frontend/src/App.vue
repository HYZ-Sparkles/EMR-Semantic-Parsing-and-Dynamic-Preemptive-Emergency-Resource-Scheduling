<template>
  <div class="app-shell">
    <div class="app-header">
      <h1>电子病历语义解析与急诊医疗资源动态抢占调度</h1>
      <div class="meta">
        大顶堆 + 字典树 + 资源抢占 · 实时仿真
      </div>
    </div>
    <div class="app-body">
      <!-- 左列：录入 + 解析 —— SemanticPanel 占主要高度，内部滚动；ReferenceTable 自然高度 -->
      <div class="col-stack col-left">
        <SemanticPanel :snapshot="snapshot" />
        <ReferenceTable />
      </div>

      <!-- 中列：排队看板 + 控制台 —— QueueDashboard 占主要高度，控制台固定在底部 -->
      <div class="col-stack">
        <QueueDashboard :snapshot="snapshot" />
        <ControlPanel :snapshot="snapshot" />
      </div>

      <!-- 右列：资源 + 日志 —— 各占 50% 高度，内容超出时面板内独立滚动 -->
      <div class="col-stack col-right">
        <ResourceGrid :snapshot="snapshot" />
        <EventLog :snapshot="snapshot" />
      </div>
    </div>
  </div>
</template>

<script setup>
import { onMounted, onBeforeUnmount, ref } from 'vue'
import { ElMessage } from 'element-plus'
import { getSnapshot, openSnapshotSocket } from './api.js'
import SemanticPanel from './components/SemanticPanel.vue'
import QueueDashboard from './components/QueueDashboard.vue'
import ResourceGrid from './components/ResourceGrid.vue'
import EventLog from './components/EventLog.vue'
import ControlPanel from './components/ControlPanel.vue'
import ReferenceTable from './components/ReferenceTable.vue'

const snapshot = ref({
  tick: 0, queue: [], patients: [], resources: [], events: [],
  auto_running: false, tick_ms: 2000
})

let ws = null
async function refresh() {
  try {
    snapshot.value = await getSnapshot()
  } catch (e) {
    console.error('加载快照失败', e)
    ElMessage.error('加载快照失败，请确认后端已启动')
  }
}

onMounted(async () => {
  await refresh()
  ws = openSnapshotSocket((msg) => { snapshot.value = msg })
})
onBeforeUnmount(() => { if (ws) ws.close() })
</script>

<style scoped>
/* 整页骨架：页头固定高度，主体撑满剩余空间。
 * 窗口低于 min-width / min-height 时，分别出横向/纵向滚动条。 */
.app-shell {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-width: 1280px;   /* 视口 < 1280px → 整体水平滚动 */
  min-height: 600px;   /* 视口 < 600px  → 整体垂直滚动 */
}

/* 顶部标题栏 */
.app-header {
  background: var(--primary);
  color: var(--on-primary);
  padding: 14px 24px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  box-shadow: 0 1px 0 var(--shadow-1);   /* 页头底部细阴影 */
}
.app-header h1 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  letter-spacing: 0.5px;
}
.app-header .meta {
  font-size: 12px;
  opacity: 0.85;
}

/* 主体三列布局：左 360 / 中自适应 / 右 380，改列宽动这里 */
.app-body {
  flex: 1;
  display: grid;
  grid-template-columns: 360px 1fr 380px;
  /* 关键：grid 行显式撑满 .app-body 剩余高度。
   * 不写这条，隐式 auto 会按"行内最高子项"算高度——右列收缩到内容最小，
   * 导致整行行高被锁死，左列 SemanticPanel 的 height 失效。 */
  grid-template-rows: minmax(0, 1fr);
  gap: 16px;
  padding: 16px;
  overflow: hidden;
}

/* 每一列内部：纵向堆叠多个面板，面板间距 16px */
.col-stack {
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-height: 0;
  height: 100%;                   /* 关键：grid 子项必须显式 100% 才能继承 .app-body 的高度，
                                   * 否则会被 min-height:0 + 内容撑开，面板 height 失效 */
  overflow: hidden;
}

/* 右列：两个面板严格 50/50，平分列高；超出时各自内部滚动 */
.col-right {
  gap: 12px;
  overflow: hidden;             /* 面板自己滚，整列不滚 */
}
/* :deep 穿透组件作用域。ResourceGrid 根元素是 .panel；EventLog 多根，第一个根也是 .panel。*/
.col-right :deep(.panel) {
  flex: 1 1 50%;
  min-height: 0;                /* 关键：允许 flex 子项收缩到小于内容高度，否则会撑破 50% */
}

/* 左列：SemanticPanel 占主要高度（自适应到占满除 ReferenceTable 之外的空间），
 * ReferenceTable 保持自然高度。这样 SemanticPanel 内部能滚，ReferenceTable 不会被裁。*/
.col-left :deep(.panel) {
  flex: 1;
  min-height: 0;
}
</style>
