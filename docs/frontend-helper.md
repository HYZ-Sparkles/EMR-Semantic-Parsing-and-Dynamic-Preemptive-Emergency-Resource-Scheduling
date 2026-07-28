# 前端阅读助手 (frontend-helper)

> 目的：在 **零 Vue 基础 + 弱 HTML/CSS** 的前提下，能读懂 [frontend/src](../frontend/src) 下所有 Vue 组件代码而不再撞墙。
>
> 读法：先看第 1–7 节建立心智，再按第 9 节顺序读文件；第 8 节是核心架构，看懂就等于看懂整个前端。

---

## 1. 总览：你只需要掌握的 6 块知识

| 序号 | 知识点 | 在本项目里的用处 |
|---|---|---|
| ① | HTML 标签、属性 | 读所有 `<template>` |
| ② | CSS 选择器、CSS 变量、flex 布局基本概念 | 读 `<style>`、理解 `class="..."` |
| ③ | `async`/`await`、`fetch`、`Promise` | 读 [api.js](../frontend/src/api.js) |
| ④ | ES6 基本语法：解构、模板字符串、`?.` 可选链、`||` 兜底 | 全文件通用 |
| ⑤ | Vue 3 Composition API 4 件套：`ref` / `computed` / `watch` / 生命周期钩子 | 所有 .vue 文件 |
| ⑥ | SFC 三段式 + props down / emit up | 所有 .vue 文件 |

下面 6 节逐个补齐；后面的章节反过来贴回本项目代码作为应用。

---

## 2. HTML 必须会（够读本项目）

### 2.1 几个会出现的高频标签

```html
<div>     通用块级容器（项目里 80% 的元素都是它）
<span>    通用行内容器（跟在文本旁的内联元素）
<h1> ~ <h6> 标题
<p>       段落
<input>   输入框（但本项目多用 <el-input>）
<button>  按钮（同上 <el-button>）
<ul> <li> 无序列表
<form>    表单
<i>       行内容器（这一项目里用作"空闲/治疗"占位文字）
```

### 2.2 几个常见的 HTML 属性

| 属性 | 含义 | 本项目例子 |
|---|---|---|
| `class="..."` | CSS 类名，空格分隔多个 | `class="panel-body"` |
| `id="..."` | 唯一标识 | `<div id="app">`（挂载点） |
| `style="..."` | 内联 CSS（一般不建议，本项目几乎不用） | — |
| `placeholder="..."` | 输入框提示 | `placeholder="王女士"` |
| `type="..."` | 输入类型 | `type="textarea"` |
| `lang="..."` | 语言 | `<html lang="zh-CN">` |
| `<script>` / `<link>` | 引入 JS / CSS | `<script type="module" src="/src/main.js">` |

### 2.3 一个最小 HTML 页面长这样

```html
<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">      <!-- 字符编码 -->
  <title>页面标题</title>
</head>
<body>
  <div id="app">                <!-- 空盒子，等下被 Vue 填满 -->
    Hello World
  </div>
  <script type="module" src="/src/main.js"></script>
</body>
</html>
```

本项目唯一一份 HTML 是 [frontend/index.html](../frontend/index.html)，只有上面这个套路，没有内容，所有内容由 Vue 渲染。

---

## 3. CSS 必须会（够读本项目）

### 3.1 选择器

```css
.app-header      选中 class="app-header" 的元素
#app             选中 id="app" 的元素
.dot.idle        选中同时有 dot 和 idle 类的元素
.flip-move       同上，仅一个 class

/* 嵌套 */
.resource-cell .label    选中 .resource-cell 后代里的 .label
```

### 3.2 几个属性

| 属性 | 含义 | 例子 |
|---|---|---|
| `color` | 文字颜色 | `color: var(--text-2);` |
| `background` / `background-color` | 背景色 | `background: #fff;` |
| `width` / `height` | 宽 / 高 | `width: 8px;` |
| `margin` / `padding` | 外距 / 内距 | `padding: 30px 0;` |
| `display: flex` | 弹性布局（核心） | 父容器开启 flex |
| `border-radius` | 圆角 | `border-radius: 50%;` 圆形 |
| `transition` | 动画过渡 | `transition: transform 0.4s ease;` |
| `opacity` | 透明度 0–1 | `opacity: 0.85;` |
| `font-size` | 字号 | `font-size: 12px;` |
| `text-align` | 文本对齐 | `text-align: center;` |

### 3.3 CSS 变量（Vue 主题切换利器）

本项目大量用 `var(--xxx)` 形式，**所有变量都定义在 [frontend/src/styles.css](../frontend/src/styles.css)**：

```css
/* styles.css 里类似这样定义 */
:root {
  --idle-fg:     #4caf50;   /* 绿：空闲 */
  --busy-fg:     #ff9800;   /* 黄：治疗中 */
  --preempt-fg:  #f44336;   /* 红：抢占 */
  --text-1:      #222;
  --text-2:      #666;
  --text-3:      #aaa;
}
```

然后任何 .vue / .css 里都可以 `color: var(--text-2);`。

**这意味着**：要换主题色（比如把黄换成橘红），只改 styles.css 一处，**所有组件都会变**。

### 3.4 flex 布局（看懂三列布局就够）

```css
.app-body {
  display: flex;            /* 开启横向排版 */
  gap: 12px;                /* 子元素之间的间距 */
}
.app-body > div {
  flex: 1;                  /* 每个子元素平均分摊 */
}
```

本项目 [App.vue](../frontend/src/App.vue) 的三列布局就是这种结构：

```
[ SemanticPanel + ControlBar ] [ QueueDashboard ] [ ResourceGrid + EventLog ]
        左列 (flex:1)               中列            右列
```

**只要你能认出 `display: flex` 和 `flex: 1`，本项目里其它 CSS 都是微调**，可以跳过。

### 3.5 `<style scoped>` 是什么？

Vue 单文件组件的 `<style>` 默认是**全局生效**（污染其他组件）。加 `scoped` 后：

```vue
<style scoped>
.dot { width: 8px; }
</style>
```

Vue 会偷偷给 `.dot` 加一个 `[data-v-xxx]` 属性选择器，**只有这个组件内的 `.dot` 才生效**。本项目 [ResourceGrid.vue](../frontend/src/components/ResourceGrid.vue) 就是这个用法。

---

## 4. JavaScript 必须会

### 4.1 `async / await`：异步同步化写法

```js
async function f() {
  const r = await fetch('/api/x')   // 等网络返回再继续
  const data = await r.json()       // 等 JSON 解析
  return data
}
```

**关键点**：

- `async function`：函数返回 Promise，调用方可以用 `await` 等。
- `await xxx`：等右边的 Promise 完成，把"结果"拿出来。

本项目所有调后端的函数都长这样（见 [api.js](../frontend/src/api.js)）。

### 4.2 `fetch`：浏览器内置发 HTTP 请求

```js
fetch(url, {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ name, complaint })
})
```

返回 Promise。`await fetch(...)` 就拿到 `Response` 对象，再 `await r.json()` 拿到数据。

### 4.3 几个 ES6+ 语法

| 语法 | 例子 | 含义 |
|---|---|---|
| `const x = ...` | `const x = ref(0)` | 常量声明（比 `let` `var` 都严格） |
| 解构 `const { a, b } = obj` | `const { name, complaint } = req` | 从对象/数组里挑字段 |
| 模板字符串 `` `Hello ${name}` `` | `` `已入队，分诊分 ${r.patient.score}` `` | 字符串插值 |
| 箭头函数 `() => {...}` | `() => props.snapshot?.events || []` | 简写函数 |
| `?.` 可选链 | `props.snapshot?.queue` | 左侧为 null/undefined 时短路返回 undefined |
| `??` | `a ?? 0` | 左侧为 null/undefined 时用右侧 |
| `\|\|` 兜底 | `props.snapshot?.queue \|\| []` | 左侧为 falsy 时用右侧 |
| `...` 展开 | `[...arr].reverse()` | 复制数组/对象 |
| `import x from 'y'` | `import App from './App.vue'` | ES Module 导入 |
| `export const x = ...` | `export async function foo() {}` | 模块导出 |

这些在项目里 100% 会见到的，提前熟悉即可。

### 4.4 WebSocket 客户端（本项目用到）

```js
const ws = new WebSocket('ws://localhost:7878/ws')   // 建连
ws.onmessage = (ev) => {                              // 收到消息时
  const msg = JSON.parse(ev.data)
  // ...
}
ws.close()                                            // 关连
```

**`onmessage` 是事件回调函数，**不是 `await`**。**消息到达时浏览器自动调用。本项目 [App.vue:52](../frontend/src/App.vue#L52) 里就是这么用的：

```js
ws = openSnapshotSocket((msg) => { snapshot.value = msg })
// openSnapshotSocket 内部就是 ws.onmessage = (ev) => onMessage(JSON.parse(ev.data))
```

---

## 5. Vue 3 核心 4 件套

**只需要这 4 个 API 就能读完本项目所有 Vue 代码。**

### 5.1 `ref()`：响应式数据

```js
import { ref } from 'vue'
const count = ref(0)        // 创建
count.value                 // 读
count.value = 1             // 写
console.log(count.value)    // 1
```

**模板里自动解包**，不需要 `.value`：

```vue
<template>
  <p>{{ count }}</p>           <!-- 显示 1 -->
  <button @click="count++">    <!-- 等同于 count.value++（v-on 自动 .value） -->
</template>
```

**"响应式"的本质**：count.value 一变，**用到 count 的模板和 computed 自动重渲染**。你不需要手动 `document.xxx.textContent = ...`。

本项目**只有 `App.vue` 里一个 ref**：`snapshot`。

### 5.2 `computed()`：派生值

```js
import { ref, computed } from 'vue'
const a = ref(5)
const b = ref(3)
const sum = computed(() => a.value + b.value)
console.log(sum.value)       // 8
```

- 自动追踪 `a` `b` 两个依赖
- 依赖变了 → 自动重新计算
- 没用到就不算（懒求值）

本项目大部分数据都是这种"取 snapshot 里某个字段、给个默认值"的 computed：

```js
const queue = computed(() => props.snapshot?.queue || [])
```

### 5.3 `watch()`：副作用

```js
import { ref, watch } from 'vue'
const count = ref(0)
watch(count, (newVal, oldVal) => {
  console.log(`${oldVal} → ${newVal}`)
})
// 之后 count.value++ 时会打印 "0 → 1"
```

- **区别于 computed**：watch 主动跑"命令式代码"（副作用），computed 计算"派生值"
- 本项目用一次：[ControlBar.vue:39](../frontend/src/components/ControlBar.vue#L39) 把后端推来的 `tick_ms` 同步到下拉框 `msModel`

### 5.4 生命周期：`onMounted` / `onBeforeUnmount`

```js
import { onMounted, onBeforeUnmount } from 'vue'

onMounted(() => {
  // 组件挂载到页面后跑一次；这里是发请求、开 WS 的最佳时机
})

onBeforeUnmount(() => {
  // 组件即将卸载；这里是关 WS、清定时器的好时机
  // 否则会泄漏！浏览器继续发消息，但组件已经不见了
})
```

本项目 [App.vue:50-54](../frontend/src/App.vue#L50-L54)：

```js
onMounted(async () => {
  await refresh()              // 首屏拉数据
  ws = openSnapshotSocket(...) // 开 WS
})
onBeforeUnmount(() => { if (ws) ws.close() })  // 关 WS
```

---

## 6. SFC 三段式（Single File Component）

每个 `.vue` 文件都是这样：

```vue
<template>
  <!-- HTML：决定长什么样 -->
  <p>{{ count }}</p>
</template>

<script setup>
  /* JS：决定做什么 */
  import { ref } from 'vue'
  const count = ref(0)
</script>

<style>
  /* CSS：决定怎么画 */
  .foo { color: red; }
</style>
```

### 6.1 `<script setup>` 是关键

Vue 3 的语法糖。**所有顶层 `const` 自动暴露给模板**，不用 `return`。

```vue
<script setup>
  const x = ref(0)           // 模板里直接 {{ x }} 即可
</script>
<template>
  <p>{{ x }}</p>
</template>
```

老写法（不推荐）：

```js
<script>
export default {
  setup() {
    const x = ref(0)
    return { x }              // 必须 return
  }
}
</script>
```

本项目所有 .vue 都用 `<script setup>`。

---

## 7. 项目里用到的所有 Vue 指令 / 语法速查

按出现频率从高到低排列。**任何一行代码不熟就来这里查。**

| 写法 | 含义 | 出处 |
|---|---|---|
| `{{ x }}` | 文本插值（自动调用 toString） | 全项目 |
| `:prop="..."` | 绑定属性到表达式（不是字符串） | [App.vue](../frontend/src/App.vue) 给子组件传 snapshot |
| `@event="..."` | 监听事件 | `App.vue` 监听子组件 emit |
| `v-model="x"` | **双向绑定**（表单控件） | SemanticPanel `text` / `name`；ControlBar `msModel` |
| `v-if="cond"` / `v-else` | 条件渲染 | ResourceGrid `Idle` / `Treating` 切换；EventLog 空状态 |
| `<template v-if="...">` | 无 DOM 容器的条件 | ResourceGrid 多个 `template` 切显示 |
| `v-for="(item, i) in list"` | 列表循环 | EventLog/QueueDashboard/ResourceGrid 全部 |
| `:key="id"` | `v-for` 必带，配合 Vue diff 算法 | 所有 `v-for` 同行 |
| `:class="{ name: cond }"` | 条件性加 class | EventLog `:class="e.kind"`；QueueDashboard `:class="{ top: i === 0 }"` |
| `:style="{ width: x + '%' }"` | 内联 style 对象式 | ResourceGrid 进度条 |
| `v-html="htmlString"` | 渲染 HTML 字串（⚠️ 高危） | SemanticPanel 高亮 |
| `<transition-group name="x" tag="div">` | 列表过渡动画包裹器 | QueueDashboard |
| `<el-input>` `<el-button>` 等 | Element Plus 组件（UI 库） | 全部组件 |
| `defineProps({...})` | 声明接收父组件传的 props | 所有子组件 |
| `defineEmits([...])` | 声明可触发的事件名 | 子组件用 |
| `props.xxx` | 读父传下来的值 | 所有子组件 |
| `emit('changed')` | 通知父组件（事件冒泡） | ControlBar |
| `<script setup>` | Vue 3 简洁写法 | 所有 .vue |

> **本项目里没有使用**：Pinia / Vue Router / Vuex / 自定义指令 / 插槽 / mixin / Suspense。**你没必要学这些**。

---

## 8. 架构核心：单向数据流（看完就懂整个前端）

**全项目的真理由后端 Rust 持有；前端只是一个可视化层。**

```
              ┌──────────────────────────────────────────────┐
              │           后端 Rust (真理)                      │
              │                                              │
              │  Scheduler { tick, heap, patients, resources }│
              │       ▲                                       │
              │       │ Arc<RwLock>                          │
              │       │                                       │
              │  前端 HTTP 请求 ◄──────┐                      │
              │       │              │                      │
              │  broadcast::channel │                      │
              │       │              │                      │
              │  WS 推送 snapshot ───┼──┐                   │
              └──────────────────────┼──┼────────────────────┘
                                     │  │
                          Vite proxy │  │ HTTP / WebSocket
                                     │  │
              ┌──────────────────────┴──┴────────────────────┐
              │  前端 Vue 3                                  │
              │                                              │
              │  App.vue                                     │
              │  ┌───────────────────────────────────────┐   │
              │  │  snapshot = ref({...})  ← 真理的镜像    │   │
              │  └───────────────┬───────────────────────┘   │
              │                  │ :snapshot props down       │
              │      ┌───────────┼───────────┐               │
              │      │           │           │               │
              │      ▼           ▼           ▼               │
              │  SemanticPanel  Queue  Resource  EventLog     │
              │     (主动        (派生     (派生     (派生      │
              │      POST)      only)     only)    only)      │
              │      │                                       │
              │      └─ emit('admitted') → 父 onChanged      │
              │                                              │
              └──────────────────────────────────────────────┘
```

### 8.1 数据流速记

| 触发事件 | 链路 |
|---|---|
| 用户输入主诉并等待 250ms | SemanticPanel 调 api.parseComplaint → 后端 trie → 返回 |
| 用户点「送入急诊」 | SemanticPanel 调 api.admitPatient → 后端 admit → 返回 |
| 后端有任意变化（admit/tick/reset/auto） | 任意 handler 调 broadcast_snapshot → WS 推所有 client |
| 前端 WS 收到 msg | App.vue 的 `snapshot.value = msg` |
| `snapshot` 变了 | 所有 computed 自动重算，所有 v-for 重渲染 |

### 8.2 "snapshot 是唯一 ref" 的具体含义

```js
// App.vue 第 39 行
const snapshot = ref({
  tick: 0, queue: [], patients: [], resources: [], events: [],
  auto_running: false, tick_ms: 2000
})
```

这一份是**整个前端的唯一变量**：后端的所有状态在这里。每变一次，所有子组件 rerender。

子组件里通常长这样：

```js
const props = defineProps({ snapshot: Object })
const queue = computed(() => props.snapshot?.queue || [])
```

**只读不写**——子组件永远不会改 `snapshot.value`。要改只能通过 api.js 调后端，让后端 WS 推新的过来。

唯一一个子组件有"本地可变状态"的是 SemanticPanel：`text` `name` `parseResult`（输入框数据是组件私有的，不需要进 snapshot）。

---

## 9. 文件逐个阅读指南（按依赖顺序）

按这个顺序读，每读一文件就用上面学到的概念去看。

### 9.1 顺序表

```
入口
  1. index.html       9 行   浏览器第一个加载的 HTML
  2. main.js          8 行   创建 Vue 实例，挂到 #app
  3. vite.config.js  12 行   dev server + 代理配置（可跳过细看）
  4. styles.css       几十行  CSS 变量定义（用时再回来查）

数据层（无 Vue）
  5. api.js          58 行   后端 HTTP/WS 封装

外壳
  6. App.vue         55 行   根组件：开 WS、传 snapshot 给子组件

子组件（按复杂度从低到高）
  7. EventLog.vue        43 行   v-for 列表（最简单）
  8. ControlBar.vue      62 行   v-model + watch + emit + 调 api
  9. QueueDashboard.vue  71 行   v-for + transition-group
 10. ResourceGrid.vue   105 行   嵌套 v-for + 分桶 + scoped style
 11. SemanticPanel.vue 124 行   v-html 高亮 + 节流 + Local ref + emit
```

### 9.2 每个文件重点看哪几行

#### [frontend/index.html](../frontend/index.html)

- 整文件就一个 `<div id="app">` + 一个 `<script>`，没有内容

#### [frontend/src/main.js](../frontend/src/main.js)

- 7 行：4 个 import + `createApp(App).use(ElementPlus).mount('#app')`
- `.use(ElementPlus)` 注册全局组件 → 其他 .vue 不用导入就能用 `<el-input>` 等

#### [frontend/src/api.js](../frontend/src/api.js)

- 7 个 async function，每个都是 `fetch + JSON.stringify + r.json()`
- 1 个 WebSocket 工厂：`openSnapshotSocket(onMessage)`，把消息回调
- **100% 没有 Vue**——纯 JS

#### [frontend/src/App.vue](../frontend/src/App.vue)

- 第 32 行：`import { getSnapshot, openSnapshotSocket } from './api.js'`
- 第 39-42 行：唯一的 `ref` `snapshot`（模板里叫 snapshot、脚本里是 snapshot.value）
- 第 50-54 行：`onMounted` + `onBeforeUnmount`：HTTP GET 一次 → 开 WS → 关时 close
- 第 12-25 行：模板把 `:snapshot` 传给 5 个子组件，监听 `@admitted` `@changed` 但回调里是空操作（WS 自己推送）

#### [frontend/src/components/EventLog.vue](../frontend/src/components/EventLog.vue)

- 整文件就一 `v-for` + 一 `kindLabel` 翻译函数
- 一行 computed：`const events = computed(() => props.snapshot?.events || [])`
- 重点看 `v-for` 的 `:key="i"` 和 `:class="e.kind"`

#### [frontend/src/components/ControlBar.vue](../frontend/src/components/ControlBar.vue)

- 4 个 computed：`auto`、`s`、`treatingCount` 等
- 1 个 v-model：`msModel = ref(...)`、`<el-select v-model="msModel">`
- 1 个 watch：同步后端 tick_ms 回来
- 3 个 async 函数：`toggleAuto`、`tickOnce`、`reset`，都调 api.js 后 emit('changed')

#### [frontend/src/components/QueueDashboard.vue](../frontend/src/components/QueueDashboard.vue)

- `<transition-group name="flip">` 包裹 `v-for`
- `:class="{ top: i === 0 }"` 给队首加 `top` 类
- 模板里 `:style` 没用，但看到 `props.snapshot.queue.length`

#### [frontend/src/components/ResourceGrid.vue](../frontend/src/components/ResourceGrid.vue)

- 外层 `v-for="group in groups"` + 内层 `v-for="r in group.list"` 双层循环
- `groups` computed 用 Map 分桶：把 8 个 slot 按 kind 分 4 组
- `<template v-if/v-else>` 切换"空闲 / 治疗中"两种 DOM
- `:style="{ width: progressPct(r) + '%' }"` 进度条
- `<style scoped>` 里的 `.dot` 只在本组件生效

#### [frontend/src/components/SemanticPanel.vue](../frontend/src/components/SemanticPanel.vue)

- 3 个本地 ref：`text`（输入框内容）、`name`（姓名）、`parseResult`（后端解析结果）
- v-model 绑 `<el-input>`
- **节流**：`onInput()` 用 `setTimeout(refresh, 250)` 实现 250ms 防抖
- 高亮：`computedHighlighted()` 返回一个函数，模板里 `v-html="highlighted()"` 用
- XSS 防御：`escape()` 把 `<>&"` 转实体
- emit('admitted') 通知父

---

## 10. 三个容易踩坑的点

### 10.1 `v-html` 危险但本项目用了

```vue
<span v-html="highlighted()"></span>
```

`v-html` 把字符串当 HTML 渲染。如果字符串里有用户输入但没转义，会出现 XSS。

本项目用 `escape()` 函数防御：先 escape 再嵌入 `<mark>`。

**永远不要直接把用户输入拼进 v-html 字符串**，转义是必须的。

### 10.2 Element Plus 组件的 prop 是驼峰还是短横？

Element Plus 一些 prop 用"短横线"传入：

```vue
<el-input :rows="3" type="textarea" />
<el-select v-model="msModel">
  <el-option label="1s" :value="1000" />
</el-select>
```

`:rows` 不是 `:rows`。`:value` 是 `:value` 不是 `:modelValue`（虽然有些 prop v-model 解析为 `:modelValue`）。

如果某个 prop 不生效：**先看 Element Plus 文档**（项目用到的不多，主要靠试错）。

### 10.3 ws.close() 必须写，否则切页面泄漏

本项目 `App.vue` 是根组件（`main.js` 里 mount），正常情况下不会卸载——所以 `onBeforeUnmount` 几乎不会触发。但**这是 Vue 单页应用的标准模式**：

- 假设根组件被卸载（例如 SPA 路由切走）
- `onBeforeUnmount` 关 WS
- 否则浏览器继续在后台维持连接，浪费资源

如果写在某个**路由子组件**里，这段代码就是必须的。

---

## 11. 速查表：碰见陌生写法怎么找

| 看见的写法 | 在本文找哪一节 |
|---|---|
| `<el-button @click="x">` | §7 指令速查（@event），§9.2 App.vue |
| `:snapshot="snapshot"` | §7（:prop），§8 单向数据流 |
| `defineProps({...})` | §7，§6.1 |
| `import { ref, computed } from 'vue'` | §5.1 §5.2 |
| `props.snapshot?.xxx` | §4.3（?.），§9 各组件 |
| `computed(() => …)` | §5.2 |
| `async function f() { ... }` | §4.1 |
| `fetch('/api/x', {...})` | §4.2，§9.2 api.js |
| `JSON.stringify({...})` | §4.3 |
| `v-for="(x, i) in list" :key="i"` | §7 |
| `class="..."` 里有逗号 | §3.5 scoped / 没看懂就 §3.1 |
| `<style scoped>` | §3.5 |
| `var(--idle-fg)` | §3.3 |
| `display: flex` | §3.4 |
| `<transition-group>` | §7 |

---

## 12. 进一步阅读

| 想 | 推荐 |
|---|---|
| 看官方 Vue 文档 | [https://cn.vuejs.org/guide/introduction.html](https://cn.vuejs.org/guide/introduction.html) |
| 看官方 Element Plus 文档 | [https://element-plus.org/zh-CN/](https://element-plus.org/zh-CN/) |
| 看 Vite 文档（为什么快） | [https://cn.vitejs.dev/guide/why.html](https://cn.vitejs.dev/guide/why.html) |
| 看本项目的README / 整体架构 | [../README.md](../README.md) |
| 看后端代码的对应前端调用 | 任何 .vue 里的 `api.js` 引用 → 对照 [../backend/src/api.rs](../backend/src/api.rs) |
