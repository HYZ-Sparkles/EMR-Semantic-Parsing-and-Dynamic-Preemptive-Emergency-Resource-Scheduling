// 调度器 + 时间步仿真。
// 职责：
//   1) 接收新患者 (parse 后) -> 入堆
//   2) 仿真 tick：推进在治资源、把在队患者按"等待时间"恶化、把堆顶尝试分配资源
//   3) 抢占决策：没有空闲槽位时，弹堆顶，若堆顶分数 > 任意在治患者最低分则抢占
//   4) 资源释放时恢复被挂起的患者
//   5) 生成 Event 列表供前端展示

use std::sync::Arc;
use parking_lot::RwLock;

use crate::heap::{IndexedMaxHeap, Entry};
use crate::resources::{ResourceTable, SlotState, SuspendedPatient};
use crate::term::{ResourceKind, TERMS, max_weight};
use crate::trie::Trie;

#[derive(Clone, Debug, serde::Serialize)]
pub struct Patient {
    pub id: String,
    pub name: String,
    pub complaint: String,
    pub score: u32,           // 当前权重
    pub initial_score: u32,   // 入队时打分
    pub matched: Vec<crate::trie::Match>,
    pub arrived_tick: u64,
    pub desired_kind: Option<ResourceKind>, // 推荐资源类别 (取最高权重词)
    pub state: PatientState,
    pub treating_slot: Option<String>,      // 正在使用哪个资源
    pub waiting_for_slot: Option<String>,   // 抢占后挂起在哪个资源上
    pub ticks_waited: u32,                  // 累计等待 tick (用于恶化)
    pub history: Vec<HistoryEvent>,         // 该患者生命周期事件
}

#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
pub enum PatientState {
    Queued,        // 排队中
    Treating,      // 治疗中
    Suspended,     // 被抢占挂起
    Finished,      // 已完成 (从内存中清除 / 留作统计)
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct HistoryEvent {
    pub tick: u64,
    pub kind: String,        // 'deteriorate' / 'preempt' / 'resume' / 'finish' / 'assign'
    pub detail: String,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct SimEvent {
    pub tick: u64,
    pub kind: String,            // 'arrive' / 'assign' / 'preempt' / 'suspend' / 'resume' / 'finish' / 'deteriorate'
    pub patient_id: Option<String>,
    pub patient_name: Option<String>,
    pub slot_id: Option<String>,
    pub detail: String,
    pub score: Option<u32>,
}

pub struct Scheduler {
    pub trie: Trie,
    pub heap: IndexedMaxHeap,
    pub patients: std::collections::HashMap<String, Patient>,
    pub resources: ResourceTable,
    pub tick: u64,
    pub events: Vec<SimEvent>,
    pub auto_running: bool,
    pub tick_ms: u64,            // 仿真步长 (毫秒)
    pub deterioration_every: u64,// 每 N 个 tick 给在队患者加 1 分
    pub deterioration_step: u32,
}

impl Scheduler {
    pub fn new() -> Self {
        let mut trie = Trie::new();
        for t in TERMS {
            trie.insert(t.word, t.weight);
        }
        Self {
            trie,
            heap: IndexedMaxHeap::default(),
            patients: Default::default(),
            resources: ResourceTable::default_set(),
            tick: 0,
            events: Vec::new(),
            auto_running: false,
            tick_ms: 2000,
            deterioration_every: 2,
            deterioration_step: 1,
        }
    }

    /// 解析一段主诉文本，返回所有匹配词与累计分数
    pub fn parse(&self, text: &str) -> ParseResult {
        let hits = self.trie.scan(text);
        let mut score = 0u32;
        let mut best_kind: Option<ResourceKind> = None;
        let mut best_w = 0u32;
        for h in &hits {
            score = score.saturating_add(h.weight);
            if h.weight > best_w {
                best_w = h.weight;
                best_kind = TERMS.iter().find(|t| t.word == h.word).map(|t| t.kind);
            }
        }
        ParseResult { matched: hits, score, desired_kind: best_kind }
    }

    /// 患者到达 (经 parse 后)
    pub fn admit(&mut self, name: &str, complaint: &str) -> Option<(Patient, SimEvent)> {
        let parsed = self.parse(complaint);
        if parsed.matched.is_empty() {
            // 没有任何医学关键词，但仍允许以 1 分入队
        }
        let id = uuid::Uuid::new_v4().to_string();
        let mut p = Patient {
            id: id.clone(),
            name: name.to_string(),
            complaint: complaint.to_string(),
            score: parsed.score.max(1),
            initial_score: parsed.score.max(1),
            matched: parsed.matched,
            arrived_tick: self.tick,
            desired_kind: parsed.desired_kind,
            state: PatientState::Queued,
            treating_slot: None,
            waiting_for_slot: None,
            ticks_waited: 0,
            history: vec![],
        };
        p.history.push(HistoryEvent { tick: self.tick, kind: "arrive".into(), detail: format!("入队，初始分 {}", p.score) });
        let e = Entry {
            id: id.clone(),
            score: p.score,
            arrived_tick: self.tick,
            heap_idx: 0,
        };
        let ev = SimEvent {
            tick: self.tick,
            kind: "arrive".into(),
            patient_id: Some(id.clone()),
            patient_name: Some(p.name.clone()),
            slot_id: None,
            detail: format!("主诉「{}」 -> 解析得分 {}", complaint, p.score),
            score: Some(p.score),
        };
        self.heap.push(e);
        self.patients.insert(id.clone(), p);
        self.events.push(ev.clone());
        // 立即尝试分配 — 收集新事件
        let mut new_events = Vec::new();
        self.try_assign_top_collect(&mut new_events);
        for e in new_events { self.events.push(e); }
        let pat = self.patients.get(&id).cloned().unwrap();
        Some((pat, ev))
    }

    /// 推进 1 个仿真 tick
    pub fn step(&mut self) -> Vec<SimEvent> {
        self.tick += 1;
        let mut out = Vec::new();
        // 事件统一回写到 self.events (供 snapshot 展示)
        // 1) 在队患者按等待时长恶化
        let snapshot = self.heap.snapshot();
        for e in &snapshot {
            if self.tick % self.deterioration_every == 0 {
                if let Some(p) = self.patients.get_mut(&e.id) {
                    if p.state == PatientState::Queued {
                        p.ticks_waited += self.deterioration_every as u32;
                        p.score = (p.score + self.deterioration_step).min(max_weight() * 2);
                        let ev = SimEvent {
                            tick: self.tick,
                            kind: "deteriorate".into(),
                            patient_id: Some(p.id.clone()),
                            patient_name: Some(p.name.clone()),
                            slot_id: None,
                            detail: format!("等待 {} tick，病情恶化 +{}", p.ticks_waited, self.deterioration_step),
                            score: Some(p.score),
                        };
                        p.history.push(HistoryEvent { tick: self.tick, kind: "deteriorate".into(), detail: ev.detail.clone() });
                        out.push(ev);
                    }
                }
                self.heap.update_score(&e.id, self.patients.get(&e.id).map(|p| p.score).unwrap_or(0));
            }
        }
        // 2) 推进在治资源
        let outcomes = self.resources.tick();
        for o in outcomes {
            if let Some(pid) = o.finished_patient.clone() {
                if let Some(p) = self.patients.get_mut(&pid) {
                    p.state = PatientState::Finished;
                    p.treating_slot = None;
                    let ev = SimEvent {
                        tick: self.tick,
                        kind: "finish".into(),
                        patient_id: Some(p.id.clone()),
                        patient_name: Some(p.name.clone()),
                        slot_id: Some(o.slot_id.clone()),
                        detail: format!("完成治疗，释放资源 {}", o.slot_id),
                        score: Some(p.score),
                    };
                    p.history.push(HistoryEvent { tick: self.tick, kind: "finish".into(), detail: ev.detail.clone() });
                    out.push(ev);
                }
            }
            // 3) 资源释放后恢复被挂起的患者
            if let Some(slot) = self.resources.slots.iter_mut().find(|s| s.id == o.slot_id) {
                if let Some(susp) = slot.suspended.pop_front() {
                    slot.state = SlotState::Treating;
                    slot.current_patient = Some(susp.patient_id.clone());
                    slot.remaining_ticks = susp.remaining_ticks.max(1);
                    slot.total_ticks = susp.total_ticks;
                    slot.preempted_by = None;
                    if let Some(p) = self.patients.get_mut(&susp.patient_id) {
                        p.state = PatientState::Treating;
                        p.treating_slot = Some(slot.id.clone());
                        p.waiting_for_slot = None;
                        let ev = SimEvent {
                            tick: self.tick,
                            kind: "resume".into(),
                            patient_id: Some(p.id.clone()),
                            patient_name: Some(p.name.clone()),
                            slot_id: Some(slot.id.clone()),
                            detail: format!("恢复治疗，资源 {}", slot.label),
                            score: Some(p.score),
                        };
                        p.history.push(HistoryEvent { tick: self.tick, kind: "resume".into(), detail: ev.detail.clone() });
                        out.push(ev);
                    }
                }
            }
        }
        // 4) 重新尝试分配
        self.try_assign_top_collect(&mut out);
        // 全部入 self.events
        for e in &out { self.events.push(e.clone()); }
        // 控制事件长度 (避免内存膨胀)
        if self.events.len() > 200 {
            let drop = self.events.len() - 200;
            self.events.drain(0..drop);
        }
        out
    }

    /// 尝试把堆顶分配到资源；若没有空闲则考虑抢占。返回是否发生了动作
    fn try_assign_top(&mut self) {
        let mut sink = Vec::new();
        self.try_assign_top_collect(&mut sink);
    }

    fn try_assign_top_collect(&mut self, out: &mut Vec<SimEvent>) {
        // 不断尝试直到堆顶没法分到资源 (资源耗尽 or 堆空)
        loop {
            let top = match self.heap.peek() { Some(e) => e.clone(), None => break };
            // 该患者可能已经不在 Queued 状态 (e.g. 已治) — 跳过
            let pstate = self.patients.get(&top.id).map(|p| p.state.clone());
            if pstate.as_ref() != Some(&PatientState::Queued) {
                // 已不在队：从堆里移除
                self.heap.remove(&top.id);
                continue;
            }
            let desired = self.patients.get(&top.id).and_then(|p| p.desired_kind);
            // 1) 先找同类空闲
            let slot_idx = if let Some(k) = desired {
                self.resources.find_idle(k)
                    .or_else(|| self.resources.find_idle(self.fallback_kind(k)))
            } else {
                self.resources.find_idle(ResourceKind::RescueRoom)
                    .or_else(|| self.resources.find_idle(ResourceKind::CtScanner))
                    .or_else(|| self.resources.find_idle(ResourceKind::OperatingRoom))
                    .or_else(|| self.resources.find_idle(ResourceKind::Ultrasound))
            };
            if let Some(idx) = slot_idx {
                self.heap.pop();
                let slot = &mut self.resources.slots[idx];
                let dur = Self::treatment_ticks(slot.kind, self.patients.get(&top.id).map(|p| p.score).unwrap_or(1));
                slot.state = SlotState::Treating;
                slot.current_patient = Some(top.id.clone());
                slot.remaining_ticks = dur;
                slot.total_ticks = dur;
                slot.preempted_by = None;
                let pid = top.id.clone();
                if let Some(p) = self.patients.get_mut(&pid) {
                    p.state = PatientState::Treating;
                    p.treating_slot = Some(slot.id.clone());
                    let ev = SimEvent {
                        tick: self.tick,
                        kind: "assign".into(),
                        patient_id: Some(p.id.clone()),
                        patient_name: Some(p.name.clone()),
                        slot_id: Some(slot.id.clone()),
                        detail: format!("进入 {} (预计 {} tick)", slot.label, dur),
                        score: Some(p.score),
                    };
                    p.history.push(HistoryEvent { tick: self.tick, kind: "assign".into(), detail: ev.detail.clone() });
                    out.push(ev);
                }
                continue;
            }
            // 2) 没有空闲：考虑抢占同类里"剩余最少"的那个 (最低分)
            if let Some(k) = desired {
                if let Some(victim_idx) = self.resources.find_lowest_in_kind(k) {
                    // 如果堆顶分 > 受害者分 (剩余 tick 越少代表分越低/越快好) -> 抢占
                    let victim = &self.resources.slots[victim_idx];
                    let victim_score = victim.current_patient.as_ref()
                        .and_then(|pid| self.patients.get(pid))
                        .map(|p| p.score)
                        .unwrap_or(0);
                    if top.score > victim_score {
                        self.preempt(victim_idx, &top.id, out);
                        continue;
                    }
                }
            }
            // 3) 抢全局最低 (只对 top score 显著高于才抢, 否则公平)
            if let Some(victim_idx) = self.resources.find_lowest_treating() {
                let victim = &self.resources.slots[victim_idx];
                let victim_score = victim.current_patient.as_ref()
                    .and_then(|pid| self.patients.get(pid))
                    .map(|p| p.score)
                    .unwrap_or(0);
                // 抢的阈值：top 至少比 victim 高 3 分 (避免无意义抖动)
                if top.score >= victim_score + 3 {
                    self.preempt(victim_idx, &top.id, out);
                    continue;
                }
            }
            break;
        }
    }

    fn fallback_kind(&self, k: ResourceKind) -> ResourceKind {
        match k {
            ResourceKind::CtScanner => ResourceKind::RescueRoom,
            ResourceKind::Ultrasound => ResourceKind::RescueRoom,
            ResourceKind::OperatingRoom => ResourceKind::RescueRoom,
            ResourceKind::RescueRoom => ResourceKind::CtScanner,
        }
    }

    fn preempt(&mut self, victim_idx: usize, top_id: &str, out: &mut Vec<SimEvent>) {
        // 1) 把 victim 状态从堆 -> 实际：在治，资源记录
        let victim_slot = &mut self.resources.slots[victim_idx];
        let victim_pid = victim_slot.current_patient.clone().unwrap();
        let victim_remaining = victim_slot.remaining_ticks;
        let victim_total = victim_slot.total_ticks;
        // 挂起到 suspended 链表头 (新被挂起的会最先恢复)
        let victim_name = self.patients.get(&victim_pid).map(|p| p.name.clone()).unwrap_or_default();
        let victim_score = self.patients.get(&victim_pid).map(|p| p.score).unwrap_or(0);
        victim_slot.suspended.push_back(SuspendedPatient {
            patient_id: victim_pid.clone(),
            name: victim_name.clone(),
            score: victim_score,
            preempted_at_tick: self.tick,
            remaining_ticks: victim_remaining,
            total_ticks: victim_total,
        });
        // 2) 抢占者上线
        let top_score = self.heap.peek().map(|e| e.score).unwrap_or(0);
        let dur = Self::treatment_ticks(victim_slot.kind, top_score);
        let top_id_owned = top_id.to_string();
        victim_slot.state = SlotState::Preempted;
        victim_slot.current_patient = Some(top_id_owned.clone());
        victim_slot.remaining_ticks = dur;
        victim_slot.total_ticks = dur;
        victim_slot.preempted_by = Some(top_id_owned.clone());
        // 3) 更新 victim 患者 -> Suspended
        if let Some(p) = self.patients.get_mut(&victim_pid) {
            p.state = PatientState::Suspended;
            p.treating_slot = None;
            p.waiting_for_slot = Some(victim_slot.id.clone());
            let ev = SimEvent {
                tick: self.tick,
                kind: "suspend".into(),
                patient_id: Some(p.id.clone()),
                patient_name: Some(p.name.clone()),
                slot_id: Some(victim_slot.id.clone()),
                detail: format!("被高分患者抢占，挂起等待恢复"),
                score: Some(p.score),
            };
            p.history.push(HistoryEvent { tick: self.tick, kind: "preempt".into(), detail: ev.detail.clone() });
            out.push(ev);
        }
        // 4) 抢占者 -> 移除堆，进入治疗
        self.heap.remove(&top_id_owned);
        if let Some(p) = self.patients.get_mut(&top_id_owned) {
            p.state = PatientState::Treating;
            p.treating_slot = Some(victim_slot.id.clone());
            let ev = SimEvent {
                tick: self.tick,
                kind: "preempt".into(),
                patient_id: Some(p.id.clone()),
                patient_name: Some(p.name.clone()),
                slot_id: Some(victim_slot.id.clone()),
                detail: format!("抢占 {}，原患者 {} 挂起", victim_slot.label, victim_name),
                score: Some(p.score),
            };
            p.history.push(HistoryEvent { tick: self.tick, kind: "assign".into(), detail: ev.detail.clone() });
            out.push(ev);
        }
    }

    fn treatment_ticks(kind: ResourceKind, score: u32) -> u32 {
        // 越危重治疗周期越短 (假设越危重需要越快干预)
        let base = match kind {
            ResourceKind::RescueRoom => 6,
            ResourceKind::CtScanner => 4,
            ResourceKind::OperatingRoom => 8,
            ResourceKind::Ultrasound => 3,
        };
        let reduce = (score / 5).min(3);
        (base - reduce).max(2)
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct ParseResult {
    pub matched: Vec<crate::trie::Match>,
    pub score: u32,
    pub desired_kind: Option<ResourceKind>,
}

pub type Shared = Arc<RwLock<Scheduler>>;
