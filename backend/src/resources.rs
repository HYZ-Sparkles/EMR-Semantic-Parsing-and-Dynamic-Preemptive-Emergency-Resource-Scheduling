// 医疗资源状态表。每个资源槽位记录：
//   - 当前占用者 (患者 id) — 可空
//   - 被抢占挂起的"等待恢复者"列表 (恢复时取危重分最高者)
//   - 当前治疗剩余 tick 数 (用于自动释放)
//   - 是否处于"被抢占"高亮态 (供前端闪烁)

use serde::{Deserialize, Serialize};
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
    // 被挂起的患者列表，按抢占发生的先后顺序追加。
    // 恢复时不是 FIFO 也不是纯 LIFO：抢占者分数必然高于被它挤掉的人，所以越晚入列一般越危重，
    // 但挂起期间所有人仍在恶化（score 会被同步改写），早挂起的人可能反超。
    // 因此统一由 pop_most_critical() 按当前分数选人。
    pub suspended: Vec<SuspendedPatient>,
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

impl ResourceSlot {
    /// 取出该槽位上"最该恢复治疗"的挂起患者：危重分最高者优先，
    /// 同分则取最晚被抢占的（抢占链上越靠后的人被挤下来时分数越高，风险也越大）。
    /// 槽位没有挂起患者时返回 None。
    pub fn pop_most_critical(&mut self) -> Option<SuspendedPatient> {
        // max_by_key 在并列时返回最后一个，恰好满足"同分取最晚入列者"。
        let idx = self.suspended
            .iter()
            .enumerate()
            .max_by_key(|(_, s)| (s.score, s.preempted_at_tick))
            .map(|(i, _)| i)?;
        Some(self.suspended.remove(idx))
    }
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
                    suspended: Vec::new(),
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
            .filter(|(_, s)| s.kind == kind)
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
            suspended: Vec::new(),
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

    /// 构造一个挂起患者条目。
    fn susp(id: &str, score: u32, at: u64) -> SuspendedPatient {
        SuspendedPatient {
            patient_id: id.into(),
            name: id.into(),
            score,
            preempted_at_tick: at,
            remaining_ticks: 3,
            total_ticks: 5,
        }
    }

    #[test]
    fn pop_most_critical_returns_none_when_empty() {
        let mut s = slot("a", ResourceKind::RescueRoom, SlotState::Treating, Some(9), 5);
        assert!(s.pop_most_critical().is_none());
    }

    #[test]
    fn pop_most_critical_prefers_later_preemption_in_a_chain() {
        // 典型抢占链: P1(5) 先被挤下来, P2(10) 后被挤下来。后者更危重, 应先恢复。
        let mut s = slot("a", ResourceKind::RescueRoom, SlotState::Preempted, Some(15), 5);
        s.suspended.push(susp("p1", 5, 0));
        s.suspended.push(susp("p2", 10, 8));
        assert_eq!(s.pop_most_critical().unwrap().patient_id, "p2");
        // 再弹一次拿到剩下的 p1, 列表随之清空
        assert_eq!(s.pop_most_critical().unwrap().patient_id, "p1");
        assert!(s.suspended.is_empty());
    }

    #[test]
    fn pop_most_critical_follows_score_not_insertion_order() {
        // 挂起期间 p1 持续恶化反超了后入列的 p2, 此时应恢复 p1 而不是"最后入列者"。
        let mut s = slot("a", ResourceKind::RescueRoom, SlotState::Preempted, Some(20), 5);
        s.suspended.push(susp("p1", 15, 0));
        s.suspended.push(susp("p2", 10, 8));
        assert_eq!(s.pop_most_critical().unwrap().patient_id, "p1");
    }

    #[test]
    fn pop_most_critical_tie_breaks_by_latest_preemption() {
        // 分数相同则取最晚被抢占的那个
        let mut s = slot("a", ResourceKind::RescueRoom, SlotState::Preempted, Some(20), 5);
        s.suspended.push(susp("early", 12, 2));
        s.suspended.push(susp("late", 12, 9));
        assert_eq!(s.pop_most_critical().unwrap().patient_id, "late");
    }
}
