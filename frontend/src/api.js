// 与后端的薄封装。HTTP 走相对路径 (Vite 代理) , WebSocket 走 /ws 相对路径。

// 统一请求包装：校验 HTTP 状态码，失败时抛错，避免把 4xx/5xx 错误体当正常数据解析。
async function request(path, options) {
  const r = await fetch(path, options)
  if (!r.ok) throw new Error(`HTTP ${r.status} ${path}`)
  return r.json()
}

const JSON_POST = (body) => ({
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify(body)
})

export function parseComplaint(text) {
  return request('/api/parse', JSON_POST({ text }))
}

export function admitPatient(name, complaint, arriveInTicks = 0) {
  return request('/api/admit', JSON_POST({ name, complaint, arrive_in_ticks: arriveInTicks }))
}

export function randomAdmit() {
  return request('/api/random_admit', { method: 'POST' })
}

export function getSnapshot() {
  return request('/api/snapshot')
}

export function postTick() {
  return request('/api/tick', { method: 'POST' })
}

export function setAuto(running, tickMs) {
  return request('/api/auto', JSON_POST({ running, tick_ms: tickMs }))
}

export function resetAll() {
  return request('/api/reset', { method: 'POST' })
}

export function getTerms() {
  return request('/api/terms')
}

// 建立快照 WebSocket，带指数退避自动重连。
// 返回一个句柄，调用 close() 停止重连并关闭连接（供组件卸载时清理）。
export function openSnapshotSocket(onMessage) {
  const proto = location.protocol === 'https:' ? 'wss' : 'ws'
  const url = `${proto}://${location.host}/ws`

  let ws = null
  let closedByUser = false
  let retry = 0
  let reconnectTimer = null

  function connect() {
    ws = new WebSocket(url)
    ws.onopen = () => { retry = 0 }
    ws.onmessage = (ev) => {
      try { onMessage(JSON.parse(ev.data)) }
      catch (e) { console.warn('WS 消息解析失败', e, ev.data) }
    }
    ws.onclose = () => {
      if (closedByUser) return
      const delay = Math.min(1000 * 2 ** retry, 10000)   // 1s,2s,4s,8s,10s...
      retry++
      reconnectTimer = setTimeout(connect, delay)
    }
    ws.onerror = () => { try { ws.close() } catch (_) { /* onclose 接管重连 */ } }
  }
  connect()

  return {
    close() {
      closedByUser = true
      if (reconnectTimer) clearTimeout(reconnectTimer)
      if (ws) ws.close()
    }
  }
}
