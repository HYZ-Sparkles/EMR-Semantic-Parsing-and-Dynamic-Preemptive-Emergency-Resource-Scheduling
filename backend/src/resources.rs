// 医疗资源状态表。每个资源槽位记录：
//   - 当前占用者 (患者 id) — 可空
//   - 被抢占挂起的"等待恢复者"链表 (FIFO)
//   - 当前治疗剩余 tick 数 (用于自动释放)
//   - 是否处于"被抢占"高亮态 (供前端闪烁)

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use crate::term::ResourceKind;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SlotState {
    Idle,            // 绿色：空闲
    Treating,        // 黄色：正常治疗中
    Preempted,       // 红色闪烁：特危患者占用，原患者挂起
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceSlot {
    pub id: String,                // 全局唯一资源 id
    pub kind: ResourceKind,
    pub label: String,             // 人类可读标签: "抢救室-1"
    pub state: SlotState,
    pub current_patient_id: Option<String>, // 当前使用该资源的患者 id
    pub current_score: Option<u32>,      // 当前患者的危重分 (便于 O(1) 找最低分 victim)
    pub preempted_by: Option<String>,    // 抢占者 id（= current_patient_id 在抢占刚发生时；前端 UI 用此区分"被谁抢了"）
    pub preempted_victim_id: Option<String>, // 被抢走的原患者 id（前端"抢占者：xxx"展示用）
    pub suspended: VecDeque<SuspendedPatient>, // 被挂起的患者链表 (FIFO)
    pub remaining_ticks: u32,      // 距离本次治疗结束的 tick 数
    pub total_ticks: u32,          // 本次治疗总时长 (用于进度条)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SuspendedPatient {
    pub patient_id: String,
    pub name: String,
    pub score: u32,
    pub preempted_at_tick: u64,
    pub remaining_ticks: u32, // 被打断时剩余的治疗 tick
    pub total_ticks: u32,
}

#[derive(Debug)]
pub struct ResourceTable {
    pub slots: Vec<ResourceSlot>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct TickOutcome {
    pub slot_id: String,
    pub finished_patient: Option<String>,
    pub was_preempted: bool,
}

impl ResourceTable {
    /// 静态构造：3 抢救室 + 2 CT + 2 手术间 + 1 超声 = 8 个槽位
    pub fn default_set() -> Self {
        let mut s = Vec::new();
        let mut push = |kind, label: &str, n: u32, prefix: &str| {
            for i in 1..=n {
                s.push(ResourceSlot {
                    id: format!("{}-{}", prefix, i),
                    kind,
                    label: format!("{}-{}", label, i),
                    state: SlotState::Idle,
                    current_patient_id: None,
                    current_score: None,
                    preempted_by: None,
                    preempted_victim_id: None,
                    suspended: VecDeque::new(),
                    remaining_ticks: 0,
                    total_ticks: 0,
                });
            }
        };
        push(ResourceKind::RescueRoom,   "抢救室",  3, "rr");
        push(ResourceKind::CtScanner,    "CT",      2, "ct");
        push(ResourceKind::OperatingRoom,"手术间",  2, "or");
        push(ResourceKind::Ultrasound,   "超声",    1, "us");
        Self { slots: s }
    }

    pub fn snapshot(&self) -> Vec<ResourceSlot> { self.slots.clone() }

    /// 找指定类别里第一个空闲槽位
    pub fn find_idle(&self, kind: ResourceKind) -> Option<usize> {
        self.slots.iter().position(|s| s.state == SlotState::Idle && s.kind == kind)
    }

    /// 在同类资源中找分数最低的在治患者 (抢占时优先选这种人作 victim)。
    /// 分数相同则按 remaining_ticks 升序 (先结束的先被替换) 兜底。
    pub fn find_lowest_score_in_kind(&self, kind: ResourceKind) -> Option<usize> {
        self.slots
            .iter()
            .enumerate()
            .filter(|(_, s)| s.state == SlotState::Treating && s.kind == kind)
            .map(|(i, s)|  (i, s.current_score.unwrap(), s.remaining_ticks))
            .min_by_key(|(_, sc, rt)| (*sc, *rt))
            .map(|(i, _, _)| i)
    }

    /// Tick 推进：所有在治资源 remaining_ticks -1，结束者返回患者 id
    pub fn tick(&mut self) -> Vec<TickOutcome> {
        let mut out = Vec::new();
        for s in self.slots.iter_mut() {
            if s.state == SlotState::Idle { continue; }
            if s.remaining_ticks > 0 { s.remaining_ticks -= 1; }
            if s.remaining_ticks == 0 {
                let pid = s.current_patient_id.take();
                let _ = s.current_score.take();
                let preempted_by = s.preempted_by.take();
                s.state = SlotState::Idle;
                out.push(TickOutcome { slot_id: s.id.clone(), finished_patient: pid, was_preempted: preempted_by.is_some() });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一个指定状态的 ResourceSlot 用于测试。`id` 在同一表内必须唯一。
    fn slot(id: &str, kind: ResourceKind, state: SlotState, score: Option<u32>, ticks: u32) -> ResourceSlot {
        ResourceSlot {
            id: id.into(),
            kind,
            label: id.into(),
            state,
            current_patient_id: score.map(|s| format!("{}-patient", s)),
            current_score: score,
            preempted_by: None,
            preempted_victim_id: None,
            suspended: VecDeque::new(),
            remaining_ticks: ticks,
            total_ticks: ticks,
        }
    }

    fn table(slots: Vec<ResourceSlot>) -> ResourceTable {
        ResourceTable { slots }
    }

    #[test]
    fn find_lowest_score_returns_none_for_empty_table() {
        let t = table(vec![]);
        assert!(t.find_lowest_score_in_kind(ResourceKind::RescueRoom).is_none());
    }

    #[test]
    fn find_lowest_score_returns_none_when_all_idle() {
        let t = table(vec![
            slot("a", ResourceKind::RescueRoom, SlotState::Idle, None, 0),
            slot("b", ResourceKind::RescueRoom, SlotState::Idle, None, 0),
        ]);
        assert!(t.find_lowest_score_in_kind(ResourceKind::RescueRoom).is_none());
    }

    #[test]
    fn find_lowest_score_picks_smallest_score_in_kind() {
        // 同类三个槽位, 分数 15 / 5 / 10, 应返回分数 5 (索引 1)
        let t = table(vec![
            slot("a", ResourceKind::RescueRoom, SlotState::Treating, Some(15), 5),
            slot("b", ResourceKind::RescueRoom, SlotState::Treating, Some(5),  5),
            slot("c", ResourceKind::RescueRoom, SlotState::Treating, Some(10), 5),
        ]);
        assert_eq!(t.find_lowest_score_in_kind(ResourceKind::RescueRoom), Some(1));
    }

    #[test]
    fn find_lowest_score_tie_breaks_by_remaining_ticks() {
        // 三个槽位分数都是 5, 剩余 tick 3 / 1 / 5, 应返回 tick=1 (索引 1)
        let t = table(vec![
            slot("a", ResourceKind::RescueRoom, SlotState::Treating, Some(5), 3),
            slot("b", ResourceKind::RescueRoom, SlotState::Treating, Some(5), 1),
            slot("c", ResourceKind::RescueRoom, SlotState::Treating, Some(5), 5),
        ]);
        assert_eq!(t.find_lowest_score_in_kind(ResourceKind::RescueRoom), Some(1));
    }

    #[test]
    fn find_lowest_score_ignores_other_kinds() {
        // 抢救室 (高) + CT (低) — 查 RescueRoom 时应只看抢救室
        let t = table(vec![
            slot("a", ResourceKind::RescueRoom, SlotState::Treating, Some(20), 5),
            slot("b", ResourceKind::CtScanner,  SlotState::Treating, Some(2),  5),
        ]);
        assert_eq!(t.find_lowest_score_in_kind(ResourceKind::RescueRoom), Some(0));
        assert_eq!(t.find_lowest_score_in_kind(ResourceKind::CtScanner),  Some(1));
    }

    #[test]
    fn find_lowest_score_ignores_preempted_and_idle() {
        // Preempted 槽位分数虽低也不应被选 (因为已经被抢占过); Idle 也不选
        let t = table(vec![
            slot("a", ResourceKind::RescueRoom, SlotState::Preempted, Some(1),  5),
            slot("b", ResourceKind::RescueRoom, SlotState::Idle,      None,   0),
            slot("c", ResourceKind::RescueRoom, SlotState::Treating,  Some(8), 5),
        ]);
        assert_eq!(t.find_lowest_score_in_kind(ResourceKind::RescueRoom), Some(2));
    }

    #[test]
    fn current_score_is_cleared_when_treatment_finishes() {
        // 验证 tick() 治疗结束时 current_score 被清空, 之后 find_lowest_score 不会再考虑这个槽位
        let mut t = table(vec![
            slot("a", ResourceKind::RescueRoom, SlotState::Treating, Some(7), 1),
        ]);
        // 推进 1 个 tick, 治疗应结束
        let outcomes = t.tick();
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].finished_patient.as_deref(), Some("7-patient"));
        // 槽位回到 Idle, current_score 应为 None
        assert_eq!(t.slots[0].state, SlotState::Idle);
        assert!(t.slots[0].current_score.is_none());
        assert!(t.find_lowest_score_in_kind(ResourceKind::RescueRoom).is_none());
    }
}
