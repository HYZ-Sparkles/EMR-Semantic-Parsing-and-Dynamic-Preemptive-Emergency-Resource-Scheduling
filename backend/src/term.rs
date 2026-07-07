// 急诊医学关键词词典。每个词条带危险分，分越高越危险。
// 同时记录建议的资源类别（按优先级推荐）。
#![allow(dead_code)]

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ResourceKind {
    RescueRoom,   // 抢救室
    CtScanner,    // CT 机
    OperatingRoom,// 手术间
    Ultrasound,   // 超声
}

#[derive(Clone, Debug)]
pub struct Term {
    pub word: &'static str,
    pub weight: u32,
    pub kind: ResourceKind,
}

pub const TERMS: &[Term] = &[
    // 致命级 (15-20)
    Term { word: "心脏骤停", weight: 20, kind: ResourceKind::RescueRoom },
    Term { word: "室颤",     weight: 19, kind: ResourceKind::RescueRoom },
    Term { word: "心肌梗死", weight: 19, kind: ResourceKind::RescueRoom },
    Term { word: "脑疝",     weight: 18, kind: ResourceKind::OperatingRoom },
    Term { word: "主动脉夹层", weight: 18, kind: ResourceKind::CtScanner },
    // 重症级 (10-14)
    Term { word: "昏迷",     weight: 14, kind: ResourceKind::RescueRoom },
    Term { word: "休克",     weight: 13, kind: ResourceKind::RescueRoom },
    Term { word: "严重外伤", weight: 13, kind: ResourceKind::OperatingRoom },
    Term { word: "脑卒中",   weight: 12, kind: ResourceKind::CtScanner },
    Term { word: "胸痛",     weight: 11, kind: ResourceKind::RescueRoom },
    Term { word: "呼吸困难", weight: 11, kind: ResourceKind::RescueRoom },
    Term { word: "咯血",     weight: 10, kind: ResourceKind::CtScanner },
    // 中等 (5-9)
    Term { word: "头痛",     weight: 6,  kind: ResourceKind::CtScanner },
    Term { word: "腹痛",     weight: 6,  kind: ResourceKind::Ultrasound },
    Term { word: "高热",     weight: 5,  kind: ResourceKind::RescueRoom },
    Term { word: "呕吐",     weight: 4,  kind: ResourceKind::Ultrasound },
    Term { word: "腹泻",     weight: 3,  kind: ResourceKind::Ultrasound },
    // 轻症 (1-4)
    Term { word: "头晕",     weight: 4,  kind: ResourceKind::CtScanner },
    Term { word: "心悸",     weight: 5,  kind: ResourceKind::RescueRoom },
];

pub fn max_weight() -> u32 { 20 }
