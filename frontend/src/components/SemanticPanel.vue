<template>
  <div class="panel">
    <div class="panel-header">
      <span>患者信息输入</span>
      <span class="sub">字典树匹配 · 累计权重</span>
    </div>
    <div class="panel-body">
      <el-input
        v-model="text"
        placeholder="例：患者突发咯血并伴随昏迷 1 小时"
        @input="onInput"
      />
      
      <div v-if="parseResult">
        <!-- 有命中词：显示建议资源 + 命中列表 -->
        <template v-if="parseResult.matches.length > 0">
          <p style="margin: 8px 0 0; color: var(--text-2); font-size: 12px;">
            建议资源：<b>{{ kindLabel(parseResult.desired_kind) }}</b>
            <span class="score-badge">危重分 {{ parseResult.score }}</span>
          </p>
          <ul class="matched-list">
            <li v-for="(m, i) in parseResult.matches" :key="i">
              {{ m.word }}<b>+{{ m.weight }}</b>
            </li>
          </ul>
        </template>
        <!-- 已请求但 0 命中：给出即时提示（与后端 admit 拒收文案保持一致） -->
        <p v-else class="no-match-hint">
          未识别到医学关键词，请补充主诉描述
        </p>
      </div>
      <el-divider style="margin: 14px 0 10px;" />
      <el-form label-position="top" size="small" @submit.prevent="onAdmit">
        <div class="row-2col">
          <el-form-item label="姓名" class="col-name">
            <el-input v-model="name" placeholder="王女士" @keyup.enter="onAdmit" />
          </el-form-item>
          <el-form-item label="选择__ticks后到达(默认0)" class="col-arrive">
            <el-radio-group v-model="arriveInTicks" class="arrive-group">
              <el-radio-button v-for="n in 6" :key="n-1" :value="n-1" :label="n-1">{{ n-1 }}</el-radio-button>
            </el-radio-group>
          </el-form-item>
        </div>
        <div class="row-buttons">
          <el-button
            type="primary"
            class="btn-primary"
            :disabled="!text || !name || submitting"
            :loading="submitting"
            @click="onAdmit"
          >登记</el-button>
          <el-button
            plain
            class="btn-secondary"
            :loading="submitting"
            @click="onRandomAdmit"
          >随机患者</el-button>
        </div>
      </el-form>
    </div>
  </div>
</template>

<script setup>
import { ref, onBeforeUnmount } from 'vue'
import { parseComplaint, admitPatient, randomAdmit } from '../api.js'
import { kindLabel } from '../utils/kinds.js'
import { ElMessage } from 'element-plus'

defineProps({ snapshot: Object })

const text = ref('')
const name = ref('')
const arriveInTicks = ref(0) // 0=当前时刻; 1-5 表示 N tick 后到达
const parseResult = ref(null)
const submitting = ref(false)    // 防止连续 Enter / 双击造成重复入队
let timer = null

// 输入防抖：250ms 内的连续输入只触发一次解析
function onInput() {
  if (timer) clearTimeout(timer)
  timer = setTimeout(refresh, 250)
}

async function refresh() {
  if (!text.value.trim()) { parseResult.value = null; return }
  try {
    const r = await parseComplaint(text.value)
    // 校验后端返回结构，避免模板访问 parseResult.matches.length 时崩溃
    parseResult.value = r && Array.isArray(r.matches) ? r : null
  } catch (e) {
    parseResult.value = null
    console.warn('解析失败', e)
  }
}

onBeforeUnmount(() => { if (timer) clearTimeout(timer) })

async function onAdmit() {
  // 并发护栏：上一次请求还没回来之前，重复点击 / 连按 Enter 直接丢弃。
  if (submitting.value) return
  if (!name.value || !text.value) return
  submitting.value = true
  try {
    const r = await admitPatient(name.value, text.value, arriveInTicks.value)
    if (r.patient) {
      ElMessage.success(`${r.patient.name} 已入队，分诊分 ${r.patient.score}`)
      name.value = ''
    } else if (r.event && r.event.kind === 'pending') {
      ElMessage.info(`${name.value} 已登记，${arriveInTicks.value} tick 后到达`)
      name.value = ''
    } else if (r.event && r.event.kind === 'reject') {
      ElMessage.warning(r.event.detail || '未识别到医学关键词')
    } else {
      ElMessage.warning('录入失败')
    }
  } catch (e) {
    console.error('入队请求失败', e)
    ElMessage.error('录入失败，请检查网络或后端状态')
  } finally {
    submitting.value = false   // 不论成功 / 失败 / reject，都释放锁
  }
}

async function onRandomAdmit() {
  if (submitting.value) return
  submitting.value = true
  try {
    const r = await randomAdmit()
    if (r.patient) {
      ElMessage.success(`${r.patient.name} 已入队，分诊分 ${r.patient.score}`)
    } else if (r.event && r.event.kind === 'pending') {
      ElMessage.info(`${r.event.patient_name || '患者'} 已登记，分诊分 ${r.event.score ?? '?'}，将延迟到达`)
    } else if (r.event && r.event.kind === 'reject') {
      ElMessage.warning(r.event.detail || '未识别到医学关键词')
    } else {
      ElMessage.warning('随机录入失败')
    }
  } catch (e) {
    console.error('随机入队请求失败', e)
    ElMessage.error('随机录入失败，请检查网络或后端状态')
  } finally {
    submitting.value = false
  }
}
</script>

<style scoped>
/* 面板自身：不写死 height，让 App.vue 的左列 :deep 规则分配高度。
 * 这里只负责建立纵向 flex 容器 + min-height:0，
 * 让 .panel-body 的 flex:1 + overflow:auto 真正接管剩余空间滚动。 */
.panel {
  display: flex;
  flex-direction: column;
  min-height: 0;
}

/* 危重分小徽章（跟在“建议资源”后面） */
.score-badge {
  display: inline-block;
  background: var(--danger);
  color: var(--on-danger);
  padding: 2px 8px;
  border-radius: 12px;
  font-size: 12px;
  margin-left: 4px;
}
/* 命中医学词的胶囊标签，自动换行排列 */
.matched-list {
  list-style: none;
  padding: 0;
  margin: 8px 0 0;
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.matched-list li {
  background: var(--tag-bg);
  color: var(--primary-2);
  padding: 2px 8px;
  border-radius: 10px;
  font-size: 12px;
}
.matched-list li b { color: var(--danger); margin-left: 4px; }  /* +权重 数字 */

/* 未识别到医学词时的提示：与后端 admit 拒收文案同步 */
.no-match-hint {
  margin: 8px 0 0;
  padding: 6px 10px;
  background: var(--hint-bg);
  color: var(--warning);
  border: 1px solid var(--hint-border);
  border-radius: 4px;
  font-size: 12px;
}

/* 两列布局：姓名 60% + 到达时间 40%，节省垂直空间 */
.row-2col {
  display: flex;
  gap: 10px;
  align-items: flex-start;
}
.row-2col .col-name { flex: 0 0 50%; }
.row-2col .col-arrive { flex: 1; min-width: 0; }
/* 让 el-radio-button-group 撑满 + 单格更紧凑 */
.arrive-group { display: flex; width: 100%; }
.arrive-group :deep(.el-radio-button) { flex: 1; }
.arrive-group :deep(.el-radio-button__inner) {
  width: 100%;
  padding: 4px 0;
  font-size: 12px;
}
/* 选中的 radio 用主色高亮 */
.arrive-group :deep(.el-radio-button__inner) {
  border-color: var(--border);
}
.arrive-group :deep(.el-radio-button.is-active .el-radio-button__inner) {
  background: var(--primary);
  border-color: var(--primary);
  color: var(--on-primary);
  box-shadow: -1px 0 0 0 var(--primary);
}

/* 操作按钮行：主按钮 70% + 副按钮 30% */
.row-buttons {
  display: flex;
  gap: 10px;
  margin-top: 4px;
}
.row-buttons .btn-primary { flex: 0 0 70%; }
.row-buttons .btn-secondary { flex: 1; }
</style>

