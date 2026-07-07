// 与后端的薄封装。HTTP 走相对路径 (Vite 代理) , WebSocket 走 /ws 相对路径。

export async function parseComplaint(text) {
  const r = await fetch('/api/parse', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ text })
  })
  return r.json()
}

export async function admitPatient(name, complaint) {
  const r = await fetch('/api/admit', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ name, complaint })
  })
  return r.json()
}

export async function getSnapshot() {
  const r = await fetch('/api/snapshot')
  return r.json()
}

export async function postTick() {
  const r = await fetch('/api/tick', { method: 'POST' })
  return r.json()
}

export async function setAuto(running, tickMs) {
  const r = await fetch('/api/auto', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ running, tick_ms: tickMs })
  })
  return r.json()
}

export async function resetAll() {
  const r = await fetch('/api/reset', { method: 'POST' })
  return r.json()
}

export async function getTerms() {
  const r = await fetch('/api/terms')
  return r.json()
}

export function openSnapshotSocket(onMessage) {
  const proto = location.protocol === 'https:' ? 'wss' : 'ws'
  const ws = new WebSocket(`${proto}://${location.host}/ws`)
  ws.onmessage = (ev) => {
    try { onMessage(JSON.parse(ev.data)) } catch (_) { /* ignore */ }
  }
  return ws
}
