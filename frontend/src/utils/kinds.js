// 资源种类 -> 中文标签的统一映射。
// 原本 ControlBar / SemanticPanel / QueueDashboard 各自维护一份相同的 map，此处合并。
// 注意：ResourceGrid 使用带“室”后缀的更长文案（CT 室 / 超声室），语义不同，保留在该组件内。
export const RESOURCE_KIND_LABELS = {
  RescueRoom: '抢救室',
  CtScanner: 'CT',
  OperatingRoom: '手术间',
  Ultrasound: '超声'
}

// fallback 因上下文而异：解析/控制条用“自动分配”，排队看板用“任意”。
export function kindLabel(kind, fallback = '自动分配') {
  return RESOURCE_KIND_LABELS[kind] || fallback
}
