# 电子病历语义解析与急诊医疗资源动态抢占调度

题目 4 完整实现。技术栈：**后端 Rust (Axum + Tokio) · 前端 Vue 3 + Vite + Element Plus**。

## 项目结构

```
code/
├── backend/                Rust 工程
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs         入口 + tick 仿真循环
│       ├── term.rs         医学习语词典 (18 个关键词)
│       ├── trie.rs         手工字典树 (HashMap<char, Node>)
│       ├── heap.rs         带位置索引的大顶堆
│       ├── resources.rs    资源状态表 (8 槽位)
│       ├── scheduler.rs    调度器 + 抢占 + 恶化
│       └── api.rs          HTTP 路由 + WebSocket
├── frontend/               Vue 3 工程
│   ├── package.json
│   ├── vite.config.js
│   ├── index.html
│   └── src/
│       ├── main.js
│       ├── styles.css      配色 (无 AI 风, 干净中性)
│       ├── api.js          后端封装
│       ├── App.vue         三栏主壳
│       └── components/
│           ├── SemanticPanel.vue    字典树匹配高亮
│           ├── QueueDashboard.vue   大顶堆实时看板
│           ├── ResourceGrid.vue     资源状态 (绿/黄/红闪烁)
│           ├── ControlBar.vue       启动/暂停/步长/清空
│           └── EventLog.vue         事件日志
└── docs/                   报告/截图
```

## 数据结构（手工实现）

### 1. 字典树 `trie.rs`
- `HashMap<char, TrieNode>` 存子节点，适配中文 unicode。
- `is_end + weight` 标记词条终止，可一次扫描匹配多词。
- 复杂度：插入 O(L)，扫描 O(N·L)。
- 单元测试：`insert_and_scan` 验证 `胸痛`、`胸痛伴`、`昏迷` 三词共现。

### 2. 带索引的大顶堆 `heap.rs`
- 数组实现，`data[0]` 永远是堆顶最高分。
- `HashMap<id, idx>` 位置映射，使 `increase_key` / `update_score` / `remove` 都能 O(log N)。
- 分数相同则按 `arrived_tick` 升序（FIFO 兜底）。
- 单元测试：`push_pop_order` / `increase_key_promotes` / `remove_maintains_heap` 全部通过。

### 3. 资源状态表 `resources.rs`
- 8 个槽位：3 抢救室 + 2 CT + 2 手术间 + 1 超声。
- 每槽位：`state` (Idle/Treating/Preempted)、`current_patient`、`preempted_by`、`suspended: VecDeque`、`remaining_ticks / total_ticks`。
- `tick()` 自动减少所有在治资源，结束返回 `TickOutcome`。

## 核心算法

### 语义分诊评分
- 在主诉文本上扫描字典树，命中词条累加 `weight`。
- 取最高权重词对应的资源类型作为 `desired_kind`（推荐资源）。
- 例如 `突发咯血并伴随昏迷` → 命中 `咯血 (10) + 昏迷 (14)` = **24 分**，建议 CT 或抢救室。

### 调度器 `scheduler.rs`
**入队** → `admit()`：
1. 解析主诉得分 `>= 1`。
2. 推入大顶堆，写入 `patients` 表。
3. 立即调用 `try_assign_top_collect`。

**每个 tick**：
1. 在队患者每 2 tick 自动 +1 分（`deteriorate` 事件），更新堆。
2. 所有在治资源 `remaining_ticks - 1`；归零则置 Idle（`finish` 事件）。
3. 资源释放后，若有 `suspended` 链表头患者 → 恢复治疗（`resume` 事件）。
4. 重试分配堆顶：
   - 找同类空闲 → 分配。
   - 找同类中剩余最少者 → 若 `top.score >= victim.score + 3` → 抢占（`preempt` + `suspend` 事件）。
   - 否则找全局剩余最少者，复用同样阈值 → 抢占。
   - 仍找不到则留在堆里等待恶化。

**抢占链**：高分患者 → 占用低分资源 → 低分患者写入 `slot.suspended` 链表 → 抢占者治疗完 → 自动恢复被挂起者。

## HTTP / WebSocket API

| Method | Path          | 用途                                            |
|--------|---------------|-------------------------------------------------|
| POST   | /api/parse    | 解析一段主诉文本，返回命中词 + 累计分 + 资源建议 |
| POST   | /api/admit    | 录入患者，返回完整 patient 对象 + arrive 事件    |
| GET    | /api/snapshot | 完整状态 (tick, queue, patients, resources, events) |
| POST   | /api/tick     | 手动推进 1 个 tick                              |
| POST   | /api/auto     | `{running, tick_ms}` 启停/调速仿真               |
| POST   | /api/reset    | 清空所有状态                                    |
| GET    | /api/terms    | 医学词典                                        |
| WS     | /ws           | 推送完整 snapshot（广播通道）                   |

## 启动方法

需要 Rust 1.75+ 与 Node 18+。

```bash
# 终端 1：后端 (默认监听 7878)
cd backend
cargo run

# 终端 2：前端 (默认监听 5173，已配 Vite 代理 /api 和 /ws)
cd frontend
npm install
npm run dev
```

打开浏览器访问 **http://localhost:5173** 即可。

## 测试场景

```bash
# 编译并跑单测
cd backend && cargo test
```

端到端测试（Python，无需额外依赖）：

```bash
# 1. 解析
curl -X POST http://127.0.0.1:7878/api/parse \
  -H "Content-Type: application/json" \
  -d '{"text":"患者突发咯血并伴随昏迷"}'
# 期望: matches=[{咯血,10},{昏迷,14}], score=24, desired_kind=CtScanner 或 RescueRoom

# 2. 录入
curl -X POST http://127.0.0.1:7878/api/admit \
  -H "Content-Type: application/json" \
  -d '{"name":"测试患者","complaint":"突发咯血并伴随昏迷"}'

# 3. 启动自动仿真
curl -X POST http://127.0.0.1:7878/api/auto \
  -H "Content-Type: application/json" \
  -d '{"running":true,"tick_ms":1000}'

# 4. 看完整状态 (打开浏览器 http://localhost:5173 实时看板)
curl http://127.0.0.1:7878/api/snapshot
```

## 关键设计取舍

- **没有用现成的 PriorityQueue / TreeMap**：题目要求手工实现，所有数据结构（trie、heap、状态表）都是裸写。
- **抢占阈值 top.score >= victim.score + 3**：避免「无意义抖动」——只对显著更危重者抢占。
- **恶化每 2 tick +1 分**：模拟病情随时间线性恶化，配合抢占阈值会让等待者最终能抢到。
- **WebSocket 而非轮询**：题目要求实时看板，WS 一次连接即可拿到所有状态变化。
- **Element Plus + 自定义 CSS**：用组件库做表单/按钮/标签，状态可视化（绿/黄/红闪烁）全部手写 CSS，避免「AI 紫粉渐变」那种一眼假的配色。
- **事件长度上限 200**：避免长时间仿真后内存膨胀；前端只显示最近 30 条。

## 题目要求对照

| 要求                                            | 实现                                          |
|------------------------------------------------|-----------------------------------------------|
| 1.1 手工实现字典树                              | `trie.rs`，含测试                             |
| 1.2 带索引的动态大顶堆                          | `heap.rs`，含测试                             |
| 1.3 资源状态表 + 抢占挂起链表                   | `resources.rs` 中 `suspended: VecDeque`       |
| 2.1 语义分诊评分                                | `Scheduler::parse` + 字典树扫描               |
| 2.2 时间步 + 等待恶化                           | `Scheduler::step` 的恶化逻辑                  |
| 2.3 优先级抢占 + 挂起                           | `Scheduler::preempt`                          |
| 2.4 资源释放自动恢复                            | `step` 中 suspended 链表头 pop + resume 事件  |
| 3.1 10-20 个医学习语                            | `term.rs` 18 个，权重 1-20                    |
| 3.2 动态大顶堆                                  | `IndexedMaxHeap`                              |
| 3.3 3 种 5+ 资源                               | 抢救室(3) + CT(2) + OR(2) + US(1) = 8 槽位 4 类 |
| 3.4 完整流程                                    | 录入 → 标定 → 排队 → 抢占 → 恢复，全部跑通    |
| 4.1 语义解析高亮面板                            | `SemanticPanel.vue` + `<mark>` 标黄          |
| 4.2 急诊排队实时看板                            | `QueueDashboard.vue` + transition-group 动画  |
| 4.3 资源状态网格 (绿/黄/红闪烁)                | `ResourceGrid.vue` + `@keyframes blink`       |
