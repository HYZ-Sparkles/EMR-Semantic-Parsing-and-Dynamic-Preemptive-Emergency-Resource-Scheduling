<template>
  <div class="control-panel">
    <el-button
      :type="auto ? 'success' : 'default'"
      size="small"
      @click="toggleAuto"
    >{{ auto ? '暂停仿真' : '启动仿真' }}</el-button>
    <el-button size="small" @click="tickOnce">手动 +1 tick</el-button>
    <el-button size="small" type="danger" plain @click="reset">清空</el-button>
    <span class="stat">步长
      <el-select v-model="msModel" size="small" style="width: 90px;" @change="onChangeMs">
        <el-option label="0.5s" :value="500" />
        <el-option label="1s"   :value="1000" />
        <el-option label="2s"   :value="2000" />
        <el-option label="3s"   :value="3000" />
        <el-option label="5s"   :value="5000" />
      </el-select>
    </span>
    <span class="stat" style="margin-left: auto;">
      <span>队 <b>{{ s.queue.length }}</b></span>
      <span style="margin-left: 12px;">在治 <b>{{ counts.treating }}</b></span>
      <span style="margin-left: 12px;">挂起 <b>{{ counts.suspended }}</b></span>
      <span style="margin-left: 12px;">已完成 <b>{{ counts.finished }}</b></span>
    </span>
  </div>
</template>

<script setup>
import { computed, ref, watch } from 'vue'
import { setAuto, postTick, resetAll } from '../api.js'
import { ElMessage } from 'element-plus'

const props = defineProps({ snapshot: Object })

const auto = computed(() => !!props.snapshot?.auto_running)
const s = computed(() => props.snapshot || { queue: [], patients: [], resources: [] })
const msModel = ref(props.snapshot?.tick_ms || 2000)

watch(() => props.snapshot?.tick_ms, v => { if (v) msModel.value = v })

// 单次遍历统计各状态人数，替代原来的三次 filter
const counts = computed(() => {
  const acc = { treating: 0, suspended: 0, finished: 0 }
  for (const p of s.value.patients) {
    if (p.state === 'Treating') acc.treating++
    else if (p.state === 'Suspended') acc.suspended++
    else if (p.state === 'Finished') acc.finished++
  }
  return acc
})

async function toggleAuto() {
  try { await setAuto(!auto.value, msModel.value) }
  catch (e) { console.error(e); ElMessage.error('切换仿真状态失败') }
}
async function tickOnce() {
  try { await postTick() }
  catch (e) { console.error(e); ElMessage.error('推进 tick 失败') }
}
async function onChangeMs(v) {
  try { await setAuto(auto.value, v) }
  catch (e) { console.error(e); ElMessage.error('调整步长失败') }
}
async function reset() {
  try { await resetAll() }
  catch (e) { console.error(e); ElMessage.error('清空失败') }
}
</script>

<style scoped>
/* 中列底部工具条：按钮 + 步长 + 右侧统计，一行内 flex 排列，窄了自动换行 */
.control-panel {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 0;
  flex-wrap: wrap;
}
.control-panel .stat {        /* 统计文字（队/在治/挂起/已完成） */
  font-size: 12px;
  color: var(--text-2);
}
.control-panel .stat b { color: var(--text); }
</style>