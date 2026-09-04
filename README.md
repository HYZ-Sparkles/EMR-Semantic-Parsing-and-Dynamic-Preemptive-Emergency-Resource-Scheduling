# 电子病历语义解析与急诊医疗资源动态抢占调度

技术栈：**后端 Rust (Axum + Tokio) · 前端 Vue 3 + Vite + Element Plus**。

## 项目结构

```
code/
├── backend/                Rust 后端
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs         入口 + tick 仿真循环
│       ├── term.rs         医学习语词典 (19 个关键词, 按四档分诊权重)
│       ├── trie.rs         手工字典树 (HashMap<char, Node>)
│       ├── heap.rs         带位置索引的大顶堆
│       ├── resources.rs    资源状态表 (8 槽位)
│       ├── scheduler.rs    调度器 + 抢占 + 恶化
│       └── api.rs          HTTP 路由 + WebSocket
├── frontend/               Vue 3 前端
│   ├── package.json
│   ├── vite.config.js
│   ├── index.html
│   └── src/
│       ├── main.js
│       ├── styles.css      配色干净中性
│       ├── api.js          后端封装
│       ├── App.vue         三栏主壳
│       ├── utils/
│       │   └── kinds.js        资源类别枚举映射（前端镜像 term.rs）
│       └── components/
│           ├── SemanticPanel.vue    字典树匹配高亮
│           ├── QueueDashboard.vue   大顶堆实时看板
│           ├── ResourceGrid.vue     资源状态 (绿/黄/红闪烁)
│           ├── ReferenceTable.vue   危险分参考表
│           ├── ControlPanel.vue     启动/暂停/步长/清空 + 统计
│           └── EventLog.vue         事件日志
└── docs/                   个人学习笔记
```

## 数据结构（手工实现）

### 1. 字典树 `trie.rs`
- `HashMap<char, TrieNode>` 存子节点，适配中文 unicode。
- `is_end + weight` 标记词条终止，可一次扫描匹配多词。
- 复杂度：插入 O(L)，扫描 O(N·L)。
- 词典按四档分诊权重组织：**致命级 (16-20)** / **重症级 (10-15)** / **中等级 (5-9)** / **轻症 (1-4)**，共 **19** 个词条。

### 2. 带索引的大顶堆 `heap.rs`
- 数组实现，`data[0]` 永远是堆顶最高分。
- `HashMap<id, idx>` 位置映射，使 `update_score` / `remove` 都能 O(log N)。
- 分数相同则按 `arrived_tick` 升序（FIFO 兜底）。
- 单元测试：`push_pop_order` / `remove_maintains_heap` 全部通过。

### 3. 资源状态表 `resources.rs`
- 8 个槽位：3 抢救室 + 2 CT + 2 手术间 + 1 超声。
- 每槽位：`state` (Idle/Treating/Preempted)、`current_patient_id`、`current_score`、`preempted_by`、`suspended: Vec`、`remaining_ticks / total_ticks`。
- `tick()` 自动减少所有在治资源，结束返回 `TickOutcome`。

## 核心算法

### 语义分诊评分
- 在主诉文本上扫描字典树，命中词条累加 `weight`。
- 取最高权重词对应的资源类型作为 `desired_kind`（推荐资源）。
- 例如 `突发咯血并伴随昏迷` → 命中 `咯血 (12) + 昏迷 (14)` = **26 分**，建议 CT 或抢救室。

### 调度器 `scheduler.rs`
**入队** → `admit()`：
1. 解析主诉。若**未识别到任何医学关键词** → 立刻返回 `reject` 事件给前端提示，不入队。
2. 推入大顶堆，写入 `patients` 表。
3. 立即调用 `try_assign_top_collect`。

**每个 tick**：
1. 在队患者 + 被挂起患者每 2 tick 自动 +1 分（`deteriorate` 事件，挂起者的分数同步写回 `slot.suspended` 列表以保证 resume 时正确），更新堆。
2. 所有在治资源 `remaining_ticks - 1`；归零则置 Idle（`finish` 事件），同时清空 `current_score`。
3. 资源释放后，若该槽位有挂起患者 → 由 `pop_most_critical()` 取**当前分数最高**者恢复治疗（`resume` 事件），其最新分数写回 `slot.current_score`。
4. 重试分配堆顶：
   - 找同类空闲 → 分配。
   - 否则在同类在治患者中找**分数最低**的 victim，若 `top.score > victim.score + 3` → 抢占（`preempt` + `suspend` 事件）。
   - 仍找不到则留在堆里等待恶化。

**抢占链**：高分患者 → 占用低分资源 → 低分患者写入 `slot.suspended` 列表 → 抢占者治疗完 → 自动恢复挂起者中分数最高的一个。

> 为什么不是 FIFO / 纯 LIFO：抢占者的分数必然高于被它挤掉的人，所以同一槽位上越晚被挂起的一般越危重（FIFO 会先恢复最不危重的那个，明显不合理）；但挂起期间所有人仍在持续恶化，早挂起的人有可能反超。因此统一按**当前危重分**取最大，同分再取最晚被抢占者（退化为 LIFO）。

## HTTP / WebSocket API

| Method | Path               | 用途                                            |
|--------|--------------------|-------------------------------------------------|
| POST   | /api/parse         | 解析一段主诉文本，返回命中词 + 累计分 + 资源建议 |
| POST   | /api/admit         | 录入患者：成功返回 patient + arrive 事件；主诉未命中关键词时返回 reject 事件 |
| POST   | /api/random_admit  | 随机名 + 随机 2-3 病情 + 0-5 tick 后到达，便于压力测试 |
| GET    | /api/snapshot      | 完整状态 (tick, queue, patients, resources, events, auto_running, tick_ms) |
| POST   | /api/tick          | 手动推进 1 个 tick                              |
| POST   | /api/auto          | `{running, tick_ms?}` 启停/调速仿真             |
| POST   | /api/reset         | 清空所有状态                                    |
| GET    | /api/terms         | 医学词典 (词条 + 权重 + 资源类别)               |
| WS     | /ws                | 推送完整 snapshot（广播通道）                   |

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

## 关键设计取舍

- 手工实现关键的数据结构。
- **抢占阈值 top.score > victim.score + 3**：避免「无意义抖动」——只对显著更危重者抢占。
- **恶化每 2 tick +1 分**：模拟病情随时间线性恶化，对**在队患者**与**被挂起患者**都生效。
- **拒绝无关键词主诉**：主诉里没有命中任何医学关键词时直接 reject 并发 `reject` 事件。
- **WebSocket 而非轮询**：题目要求实时看板，WS 一次连接即可拿到所有状态变化。
- **Element Plus + 自定义 CSS**：用组件库做表单/按钮/标签，状态可视化（绿/黄/红闪烁）全部手写 CSS。
- **事件长度上限 30**：scheduler 内部裁剪，避免长时间仿真后内存膨胀；前端 EventLog 也只显示最近 30 条。