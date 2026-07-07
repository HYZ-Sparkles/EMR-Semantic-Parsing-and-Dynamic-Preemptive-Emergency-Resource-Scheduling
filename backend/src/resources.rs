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
    pub current_patient: Option<String>, // 当前使用该资源的患者 id
    pub preempted_by: Option<String>,    // 抢占者 (用于 UI 提示)
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
                    current_patient: None,
                    preempted_by: None,
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

    /// 找所有槽位里分数最低的"在治"患者 (用于决定被谁抢占)
    pub fn find_lowest_treating(&self) -> Option<usize> {
        // 跳过 preemption 状态 (它本身就是抢占者)
        self.slots
            .iter()
            .enumerate()
            .filter(|(_, s)| s.state == SlotState::Treating)
            .min_by_key(|(_, s)| s.remaining_ticks)
            .map(|(i, _)| i)
    }

    /// 在同类资源中找最"快结束"的占用 (用于非致命抢占的优化)
    pub fn find_lowest_in_kind(&self, kind: ResourceKind) -> Option<usize> {
        self.slots
            .iter()
            .enumerate()
            .filter(|(_, s)| s.state == SlotState::Treating && s.kind == kind)
            .min_by_key(|(_, s)| s.remaining_ticks)
            .map(|(i, _)| i)
    }

    /// Tick 推进：所有在治资源 remaining_ticks -1，结束者返回患者 id
    pub fn tick(&mut self) -> Vec<TickOutcome> {
        let mut out = Vec::new();
        for s in self.slots.iter_mut() {
            if s.state == SlotState::Idle { continue; }
            if s.remaining_ticks > 0 { s.remaining_ticks -= 1; }
            if s.remaining_ticks == 0 {
                let pid = s.current_patient.take();
                let preempted_by = s.preempted_by.take();
                s.state = SlotState::Idle;
                out.push(TickOutcome { slot_id: s.id.clone(), finished_patient: pid, was_preempted: preempted_by.is_some() });
            }
        }
        out
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct TickOutcome {
    pub slot_id: String,
    pub finished_patient: Option<String>,
    pub was_preempted: bool,
}
