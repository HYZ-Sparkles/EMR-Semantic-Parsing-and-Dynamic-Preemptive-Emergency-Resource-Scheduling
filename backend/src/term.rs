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
    // 致命级 (16-20) —— 立刻危及生命，需紧急干预
    Term { word: "心脏骤停", weight: 20, kind: ResourceKind::RescueRoom },
    Term { word: "室颤",     weight: 19, kind: ResourceKind::RescueRoom },
    Term { word: "心肌梗死", weight: 19, kind: ResourceKind::RescueRoom },
    Term { word: "主动脉夹层", weight: 18, kind: ResourceKind::CtScanner },
    Term { word: "脑疝",     weight: 18, kind: ResourceKind::OperatingRoom },
    Term { word: "脑卒中",   weight: 17, kind: ResourceKind::CtScanner },
    Term { word: "严重外伤", weight: 17, kind: ResourceKind::OperatingRoom },

    // 重症级 (10-15) —— 可能快速恶化，需密切监测
    Term { word: "休克",     weight: 15, kind: ResourceKind::RescueRoom },
    Term { word: "呼吸困难", weight: 14, kind: ResourceKind::RescueRoom },
    Term { word: "胸痛",     weight: 13, kind: ResourceKind::RescueRoom },
    Term { word: "咯血",     weight: 12, kind: ResourceKind::CtScanner },
    Term { word: "昏迷",     weight: 14, kind: ResourceKind::RescueRoom },

    // 中等级 (5-9) —— 需要尽快处理，但通常不即刻危及生命
    Term { word: "腹痛",     weight: 9,  kind: ResourceKind::Ultrasound }, 
    Term { word: "高热",     weight: 8,  kind: ResourceKind::RescueRoom },
    Term { word: "心悸",     weight: 8,  kind: ResourceKind::RescueRoom }, 
    Term { word: "头痛",     weight: 6,  kind: ResourceKind::CtScanner }, 

    // 轻症 (1-4) —— 可门诊处理
    Term { word: "呕吐",     weight: 4,  kind: ResourceKind::Ultrasound }, 
    Term { word: "腹泻",     weight: 3,  kind: ResourceKind::Ultrasound }, 
    Term { word: "头晕",     weight: 4,  kind: ResourceKind::CtScanner }, 
];

pub fn max_weight() -> u32 { 20 }
