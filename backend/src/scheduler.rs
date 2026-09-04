// 调度器 + 时间步仿真。
// 职责：
//   1) 接收新患者 (parse 后)。预约->加入预约队列；到来->记录信息，尝试直接分配或抢占资源，否则入堆。
//   2) 仿真 tick：判断预约患者到来，把在队患者按"等待时间"恶化，推进在治资源，挂起队列的恢复或者大顶堆排队患者分配医疗资源。
//   3) 抢占决策：没有空闲槽位时，弹堆顶，从同类资源中危险分数最小的患者进行占用，要求 top.score 严格大于 victim.score 才抢占
//   4) 资源释放时优先恢复挂起者中危重分最高的（同分取最晚被抢占者）
//   5) 生成 Event 列表供前端展示

use std::sync::Arc;
use parking_lot::RwLock;
use rand::seq::SliceRandom;
use rand::Rng;

use crate::heap::{IndexedMaxHeap, Entry};
use crate::resources::{ResourceTable, SlotState, SuspendedPatient};
use crate::term::{ResourceKind, TERMS};
use crate::trie::{Trie,Match};

/// 常用姓氏 + 常用名组合，用于 admit_random 随机生成患者姓名
const SURNAMES: &[&str] = &[
    "王", "李", "张", "刘", "陈", "杨", "黄", "赵", "吴", "周",
];
const GIVEN_NAMES: &[&str] = &[
    "伟", "芳", "娜", "敏", "静", "丽", "强", "磊", "军", "洋",
    "勇", "艳", "杰", "娟", "涛", "明", "超", "秀英", "霞", "平",
];

#[derive(Clone, Debug, serde::Serialize)]
pub struct Patient {
    pub id: String,
    pub name: String,
    pub complaint: String,
    pub score: u32,           // 当前权重
    pub initial_score: u32,   // 入队时打分
    pub matched: Vec<Match>,
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
    pub deterioration_every: u64,// 恶化所需步数
    pub deterioration_point: u32, // 恶化一次增加的危险分
    pub pending: Vec<PendingArrival>, // 登记但尚未到达的患者
}

/// 延迟到达的患者：登记时即解析 (用于提前拒绝无关键词的主诉)，
/// 但入堆延后到 arrived_tick 才会发生。
#[derive(Clone, Debug)]
pub struct PendingArrival {
    name: String,
    complaint: String,
    parsed: ParseResult,
    arrived_tick: u64,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct ParseResult {
    pub matched: Vec<Match>,
    pub score: u32,
    pub desired_kind: Option<ResourceKind>,
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
            deterioration_point: 1,
            pending: Vec::new(),
        }
    }

    /// 解析一段主诉文本，返回所有匹配词与累计分数
    pub fn parse(&self, text: &str) -> ParseResult {
        let hits = self.trie.scan(text);
        let mut score = 0u32;
        let mut best_kind: Option<ResourceKind> = None;
        let mut best_w = 0u32;
        for h in &hits {
            score = score.saturating_add(h.weight); // 饱和加法
            if h.weight > best_w {
                best_w = h.weight;
                best_kind = TERMS.iter().find(|t| t.word == h.word).map(|t| t.kind);
            }
        }
        ParseResult { matched: hits, score, desired_kind: best_kind }
    }

    /// 患者到达 (经 parse 后)。成功时返回 (Some(patient), arrive_event)；
    /// 主诉里没识别到任何医学关键词时返回 (None, reject_event)，事件 push 到 self.events
    /// 供前端做即时提示和事件日志留痕，Patient 结构本身不增加字段。
    /// arrive_in_ticks: 患者将于 N tick 后到达 (0 = 立即入队)，到点由 step() 物化。
    pub fn admit(&mut self, name: &str, complaint: &str, arrive_in_ticks: u32) -> (Option<Patient>, SimEvent) {
        let parsed = self.parse(complaint);
        if parsed.matched.is_empty() {
            let reject_ev = SimEvent {
                tick: self.tick,
                kind: "reject".into(),
                patient_id: None,
                patient_name: Some(name.to_string()),
                slot_id: None,
                detail: "未识别到医学关键词，请补充主诉描述".into(),
                score: None,
            };
            self.events.push(reject_ev.clone());
            return (None, reject_ev);
        }
        if arrive_in_ticks > 0 {
            // 延迟到达：先登记，等 step() 推进到目标 tick 再入堆
            let target = self.tick + arrive_in_ticks as u64;
            let ev = SimEvent {
                tick: self.tick,
                kind: "pending".into(),
                patient_id: None,
                patient_name: Some(name.to_string()),
                slot_id: None,
                detail: format!("主诉 {} -> 解析得分 {}，将于 {} tick 后到达 (tick {})", complaint, parsed.score, arrive_in_ticks, target),
                score: Some(parsed.score),
            };
            self.events.push(ev.clone());
            self.pending.push(PendingArrival {
                name: name.to_string(),
                complaint: complaint.to_string(),
                parsed,
                arrived_tick: target,
            });
            return (None, ev);
        }
        self.do_admit(name, complaint, parsed)
    }

    /// 随机生成一名患者：从 TERMS 抽 2 或 3 个词拼接主诉、姓+名随机组合、0-5 tick 后到达。
    /// 然后走与 admit 完全一致的登记 / 入堆流程。
    pub fn admit_random(&mut self) -> (Option<Patient>, SimEvent) {
        let mut rng = rand::thread_rng();
        let n = if rng.gen_bool(0.5) { 2 } else { 3 };
        let picks: Vec<_> = TERMS.choose_multiple(&mut rng, n).collect();
        let mut complaint = picks[0].word.to_string();
        for t in &picks[1..] {
            complaint.push_str(" 并伴随 ");
            complaint.push_str(t.word);
        }
        let surname = SURNAMES.choose(&mut rng).unwrap();
        let given = GIVEN_NAMES.choose(&mut rng).unwrap();
        let name = format!("{}{}", surname, given);
        let arrive_in_ticks = rng.gen_range(0..=5);
        self.admit(&name, &complaint, arrive_in_ticks)
    }

    /// 把一条已经解析过的患者立即物化 (入堆 + 患者表 + arrive 事件 + 尝试分配)。
    /// 既被 admit(arrive_in_ticks=0) 调用，也被 step() 中 materialize_pending 调用。
    fn do_admit(&mut self, name: &str, complaint: &str, parsed: ParseResult) -> (Option<Patient>, SimEvent) {
        let id = uuid::Uuid::new_v4().to_string(); // 随机数，几乎不可能重复id出现
        let mut p = Patient {
            id: id.clone(),
            name: name.to_string(),
            complaint: complaint.to_string(),
            score: parsed.score,
            initial_score: parsed.score,
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
            detail: format!("主诉 {} -> 解析得分 {}", complaint, p.score),
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
        (Some(pat), ev)
    }

    /// 把已到达 (arrived_tick <= self.tick) 的 pending 患者物化入堆。
    /// 由 step() 在 tick+=1 之后、恶化逻辑之前调用。
    fn materialize_pending(&mut self, out: &mut Vec<SimEvent>) {
        if self.pending.is_empty() { return; }
        let mut i = 0;
        while i < self.pending.len() {
            if self.pending[i].arrived_tick <= self.tick {
                let pend = self.pending.remove(i);
                let (_, ev) = self.do_admit(&pend.name, &pend.complaint, pend.parsed);
                out.push(ev);
            } else {
                i += 1;
            }
        }
    }

    /// 推进 1 个仿真 tick
    pub fn step(&mut self) -> Vec<SimEvent> {
        self.tick += 1;
        let mut out = Vec::new();
        // 0) 物化到点到达的 pending 患者 (arrive 事件在此处产生)
        self.materialize_pending(&mut out);
        // 事件统一回写到 self.events (供 snapshot 展示)
        // 1) 在队患者和被抢占患者按**各自**等待时长恶化 — 与调度器总 tick 解耦
        //    规则：每 tick 自增 ticks_waited；ticks_waited % deterioration_every == 0 时恶化 +deterioration_point。
        //    这样两位在不同 tick 入队的患者都会在「自己等了 deterioration_every 的整数倍 tick」时恶化，
        //    而不是被调度器总 tick 边界强制同步恶化。
        let every = self.deterioration_every as u32;
        // queued 患者：走堆 (堆里只有 Queued 患者 — 见 preempt 里只 remove/不 push 的不变式)
        let snapshot = self.heap.snapshot();
        for e in &snapshot {
            let mut new_score: Option<u32> = None;
            if let Some(p) = self.patients.get_mut(&e.id) {
                if p.state == PatientState::Queued {
                    p.ticks_waited += 1;
                    if p.ticks_waited % every == 0 {
                        p.score += self.deterioration_point;
                        let ev = SimEvent {
                            tick: self.tick,
                            kind: "deteriorate".into(),
                            patient_id: Some(p.id.clone()),
                            patient_name: Some(p.name.clone()),
                            slot_id: None,
                            detail: format!("等待 {} tick，病情恶化 +{}", p.ticks_waited, self.deterioration_point),
                            score: Some(p.score),
                        };
                        p.history.push(HistoryEvent { tick: self.tick, kind: "deteriorate".into(), detail: ev.detail.clone() });
                        out.push(ev);
                        new_score = Some(p.score);
                    }
                }
            }
            if let Some(s) = new_score {
                self.heap.update_score(&e.id, s);
            }
        }

        // suspended 患者：不在堆里，需另外遍历 self.patients
        let suspended_ids: Vec<String> = self.patients.iter()
            .filter(|(_, p)| p.state == PatientState::Suspended)
            .map(|(id, _)| id.clone())
            .collect();
        for id in suspended_ids {
            let mut new_score: Option<u32> = None;
            let mut sid_to_sync: Option<String> = None;
            if let Some(p) = self.patients.get_mut(&id) {
                p.ticks_waited += 1;
                if p.ticks_waited % every == 0 {
                    p.score += self.deterioration_point;
                    sid_to_sync = p.waiting_for_slot.clone();
                    let detail = format!("挂起等待 {} tick，病情恶化 +{}", p.ticks_waited, self.deterioration_point);
                    let ev = SimEvent {
                        tick: self.tick,
                        kind: "deteriorate".into(),
                        patient_id: Some(p.id.clone()),
                        patient_name: Some(p.name.clone()),
                        slot_id: p.waiting_for_slot.clone(),
                        detail: detail.clone(),
                        score: Some(p.score),
                    };
                    p.history.push(HistoryEvent { tick: self.tick, kind: "deteriorate".into(), detail: detail });
                    out.push(ev);
                    new_score = Some(p.score);
                }
            }
            if let (Some(s), Some(sid)) = (new_score, sid_to_sync) {
                if let Some(slot) = self.resources.slots.iter_mut().find(|sl| sl.id == sid) {
                    for sp in slot.suspended.iter_mut() {
                        if sp.patient_id == id {
                            sp.score = s;
                        }
                    }
                }
            }
        }

        // 2) 推进在治资源
        let outcomes = self.resources.tick();
        for o in outcomes {
            // 从 slot_id 反查人类可读的 label（"rr-1" -> "抢救室-1"）。
            // tick() 已经把资源切到 Idle，但 label 字段仍保留，直接 clone 出来即可。
            let slot_label = self.resources.slots.iter()
                .find(|s| s.id == o.slot_id)
                .map(|s| s.label.clone())
                .unwrap_or(o.slot_id.clone());
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
                        detail: format!("完成治疗，释放资源 {}", slot_label),
                        score: Some(p.score),
                    };
                    p.history.push(HistoryEvent { tick: self.tick, kind: "finish".into(), detail: ev.detail.clone() });
                    out.push(ev);
                }
            }
            // 3) 资源释放后恢复被挂起的患者
            if let Some(slot) = self.resources.slots.iter_mut().find(|s| s.id == o.slot_id) {
                if let Some(susp) = slot.pop_most_critical() {
                    slot.state = SlotState::Treating;
                    slot.current_patient_id = Some(susp.patient_id.clone());
                    slot.current_score = Some(susp.score);
                    slot.remaining_ticks = susp.remaining_ticks.max(1);
                    slot.total_ticks = susp.total_ticks;
                    slot.preempted_by = None;
                    slot.preempted_victim_id = None;
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
        if self.events.len() > 30 {
            let drop = self.events.len() - 30;
            self.events.drain(0..drop);
        }
        out
    }

    fn try_assign_top_collect(&mut self, out: &mut Vec<SimEvent>) {
        // 不断尝试直到堆顶没法分到资源 (资源耗尽 or 堆空)
        loop {
            let top = match self.heap.peek() { Some(e) => e.clone(), None => break };
            let desired = self.patients.get(&top.id).and_then(|p| p.desired_kind);
            // 1) 先找同类空闲
            let slot_idx = self.resources.find_idle(desired.unwrap());
            if let Some(idx) = slot_idx {
                self.heap.pop();
                let slot = &mut self.resources.slots[idx];
                let top_score = self.patients.get(&top.id).map(|p| p.score).unwrap();
                let dur = Self::treatment_ticks(top_score);
                slot.state = SlotState::Treating;
                slot.current_patient_id = Some(top.id.clone());
                slot.current_score = Some(top_score);
                slot.remaining_ticks = dur;
                slot.total_ticks = dur;
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
            // 2) 同类资源中抢占：选分数最低的 victim ，
            //    要求 top.score 严格大于 victim.score+3 才抢占，设置抖动阈值。
            if let Some(victim_idx) = self.resources.find_lowest_score_in_kind(desired.unwrap()) {
                let victim_score = self.resources.slots[victim_idx].current_score.unwrap();
                if top.score > victim_score+3 {
                    self.preempt(victim_idx, &top.id, out);
                    continue;
                }
            }
            break;
        }
    }

    fn preempt(&mut self, victim_idx: usize, pid: &str, out: &mut Vec<SimEvent>) {
        // 1) 把 victim 当前的治疗上下文 (剩余 tick、总 tick) 写入该槽位的 suspended 列表。
        //    victim 此刻不在线程队列里，只是把状态机置为 Suspended 以待恢复。
        let victim_slot = &mut self.resources.slots[victim_idx];
        let victim_pid = victim_slot.current_patient_id.clone().unwrap();
        let victim_remaining = victim_slot.remaining_ticks;
        let victim_total = victim_slot.total_ticks;
        // 追加到列表尾部即可，恢复顺序不由入列顺序决定，而是 pop_most_critical() 按当前分数挑人。
        let victim_name = self.patients.get(&victim_pid).map(|p| p.name.clone()).unwrap();
        let victim_score = self.patients.get(&victim_pid).map(|p| p.score).unwrap();
        victim_slot.suspended.push(SuspendedPatient {
            patient_id: victim_pid.clone(),
            name: victim_name.clone(),
            score: victim_score,
            preempted_at_tick: self.tick,
            remaining_ticks: victim_remaining,
            total_ticks: victim_total,
        });
        // 2) 抢占者上线
        let top_score = self.heap.peek().map(|e| e.score).unwrap();
        let dur = Self::treatment_ticks(top_score);
        let top_id_owned = pid.to_string();
        victim_slot.state = SlotState::Preempted;
        victim_slot.current_patient_id = Some(top_id_owned.clone());
        victim_slot.current_score = Some(top_score);
        victim_slot.remaining_ticks = dur;
        victim_slot.total_ticks = dur;
        // 抢占者 = 新上线的 top_id；被抢走的原患者 = victim_pid。
        // 这两个字段语义不同：preempted_by 给 UI 区分"刚被抢占"，
        // preempted_victim_id 给 UI 找到"原来的患者是谁"。
        victim_slot.preempted_by = Some(top_id_owned.clone());
        victim_slot.preempted_victim_id = Some(victim_pid.clone());
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

    fn treatment_ticks(score: u32) -> u32 {
        // 越危重治疗周期越长
        (score/3).max(1)
    }
}

pub type Shared = Arc<RwLock<Scheduler>>;
