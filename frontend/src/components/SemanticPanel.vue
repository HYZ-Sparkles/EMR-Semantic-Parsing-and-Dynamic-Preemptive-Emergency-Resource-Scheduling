<template>
  <div class="panel">
    <div class="panel-header">
      <span>语义解析 &amp; 录入</span>
      <span class="sub">字典树匹配 · 累计权重</span>
    </div>
    <div class="panel-body">
      <el-input
        v-model="text"
        type="textarea"
        :rows="3"
        placeholder="例：患者突发咯血并伴随昏迷 1 小时"
        @input="onInput"
      />
      <div class="parse-output" v-if="text">
        <span v-html="highlighted"></span>
      </div>
      <div v-if="parseResult">
        <p style="margin: 8px 0 0; color: var(--text-2); font-size: 12px;">
          建议资源：<b>{{ kindLabel(parseResult.desired_kind) }}</b>
          <span class="score-badge">危重分 {{ parseResult.score }}</span>
        </p>
        <ul class="matched-list">
          <li v-for="(m, i) in parseResult.matches" :key="i">
            {{ m.word }}<b>+{{ m.weight }}</b>
          </li>
        </ul>
      </div>
      <el-divider style="margin: 14px 0 10px;" />
      <el-form label-position="top" size="small">
        <el-form-item label="姓名">
          <el-input v-model="name" placeholder="王女士" />
        </el-form-item>
        <el-button
          type="primary"
          style="width: 100%;"
          :disabled="!text || !name"
          @click="onAdmit"
        >送入急诊</el-button>
      </el-form>
    </div>
  </div>
</template>

<script setup>
import { ref, watch } from 'vue'
import { parseComplaint, admitPatient } from '../api.js'
import { ElMessage } from 'element-plus'

const props = defineProps({ snapshot: Object })
const emit = defineEmits(['admitted'])

const text = ref('')
const name = ref('')
const parseResult = ref(null)
let timer = null

function onInput() {
  if (timer) clearTimeout(timer)
  timer = setTimeout(refresh, 250)
}

async function refresh() {
  if (!text.value.trim()) { parseResult.value = null; return }
  parseResult.value = await parseComplaint(text.value)
}

watch(text, () => { onInput() })

const highlighted = computedHighlighted()
function computedHighlighted() {
  return () => {
    const t = text.value || ''
    const m = parseResult.value?.matches || []
    if (m.length === 0) return escape(t)
    // 按 start 排序, 区间合并
    const sorted = [...m].sort((a, b) => a.start - b.start)
    const ranges = []
    for (const x of sorted) {
      const last = ranges[ranges.length - 1]
      if (last && x.start <= last.end) {
        last.end = Math.max(last.end, x.end)
        last.words.push(x.word)
      } else {
        ranges.push({ start: x.start, end: x.end, words: [x.word] })
      }
    }
    let out = '', cur = 0
    for (const r of ranges) {
      out += escape(t.slice(cur, r.start))
      out += `<mark>${escape(t.slice(r.start, r.end))}</mark>`
      cur = r.end
    }
    out += escape(t.slice(cur))
    return out
  }
}

function escape(s) { return s.replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c])) }

function kindLabel(k) {
  return ({
    RescueRoom: '抢救室',
    CtScanner: 'CT',
    OperatingRoom: '手术间',
    Ultrasound: '超声'
  })[k] || '自动分配'
}

async function onAdmit() {
  if (!name.value || !text.value) return
  const r = await admitPatient(name.value, text.value)
  if (r.patient) {
    ElMessage.success(`${r.patient.name} 已入队，分诊分 ${r.patient.score}`)
    emit('admitted', r)
    name.value = ''
  } else {
    ElMessage.warning('录入失败')
  }
}
</script>
