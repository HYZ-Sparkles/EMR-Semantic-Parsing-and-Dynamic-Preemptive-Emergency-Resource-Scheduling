<template>
  <div class="ref-title">危险分参考表<span class="sub">共 {{ terms.length }} 个关键词</span></div>
  <div class="ref-table">
    <table>
      <thead>
        <tr><th>关键词</th><th>危险分</th><th>建议资源</th></tr>
      </thead>
      <tbody>
        <tr v-for="(t, i) in terms" :key="i">
          <td>{{ t.word }}</td>
          <td><b>{{ t.weight }}</b></td>
          <td>{{ kindLabel(t.kind) }}</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { getTerms } from '../api.js'
import { kindLabel } from '../utils/kinds.js'

// 危险分参考表：所有关键词，按分数降序；无 props，靠 onMounted 自行拉取
const terms = ref([])
onMounted(async () => {
  try {
    const r = await getTerms()
    terms.value = (r.terms || []).sort((a, b) => b.weight - a.weight)
  } catch (e) {
    console.error('加载关键词表失败', e)
  }
})
</script>

<style scoped>
/* 危险分参考表标题 */
.ref-title {
  display: flex;
  align-items: baseline;
  gap: 8px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
}
.ref-title .sub {
  font-size: 11px;
  font-weight: 400;
  color: var(--text-2);
}
/* 内部空间小：固定最大高度，超出可上下滚动 */
.ref-table {
  max-height: 200px;
  overflow-y: auto;
  border: 1px solid var(--border);
  border-radius: 6px;
}
.ref-table table {
  width: 100%;
  border-collapse: collapse;
  font-size: 10px;
}
.ref-table th,
.ref-table td {
  padding: 5px 10px;
  text-align: left;
  border-bottom: 1px solid var(--border);
}
/* 表头吸顶，滚动时始终可见 */
.ref-table thead th {
  position: sticky;
  top: 0;
  background: var(--queue-bg);   /* 与队列条目同色，形成视觉一致 */
  color: var(--text-2);
  font-weight: 600;
  z-index: 1;
}
.ref-table tbody tr:last-child td {
  border-bottom: none;
}
.ref-table td b {
  color: var(--danger);           /* 危险分沿用站点统一红色 */
}
</style>