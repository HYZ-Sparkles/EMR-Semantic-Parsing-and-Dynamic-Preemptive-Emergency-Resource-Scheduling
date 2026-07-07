<template>
  <div class="control-bar">
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
      <span style="margin-left: 12px;">在治 <b>{{ treatingCount }}</b></span>
      <span style="margin-left: 12px;">挂起 <b>{{ suspendedCount }}</b></span>
      <span style="margin-left: 12px;">已完成 <b>{{ finishedCount }}</b></span>
    </span>
  </div>
</template>

<script setup>
import { computed, ref, watch } from 'vue'
import { setAuto, postTick, resetAll } from '../api.js'

const props = defineProps({ snapshot: Object })
const emit = defineEmits(['changed'])

const auto = computed(() => !!props.snapshot?.auto_running)
const s = computed(() => props.snapshot || { queue: [], patients: [], resources: [] })
const msModel = ref(props.snapshot?.tick_ms || 2000)

watch(() => props.snapshot?.tick_ms, v => { if (v) msModel.value = v })

const treatingCount  = computed(() => s.value.patients.filter(p => p.state === 'Treating').length)
const suspendedCount = computed(() => s.value.patients.filter(p => p.state === 'Suspended').length)
const finishedCount  = computed(() => s.value.patients.filter(p => p.state === 'Finished').length)

async function toggleAuto() {
  await setAuto(!auto.value, msModel.value)
  emit('changed')
}
async function tickOnce() {
  await postTick()
  emit('changed')
}
async function onChangeMs(v) {
  if (auto.value) await setAuto(true, v)
  else await setAuto(false, v)
}
async function reset() {
  await resetAll()
  emit('changed')
}
</script>
