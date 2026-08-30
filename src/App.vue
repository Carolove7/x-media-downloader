<template>
  <div class="app-root">
    <!-- 账户设置 -->
    <section class="card">
      <div class="card-title">账户设置</div>
      <div class="config-body">
        <div class="row">
          <span class="row-label">账户 ID:</span>
          <div class="row-value">
            <div class="combo" ref="comboRef">
              <input
                v-model.trim="user_id"
                type="text"
                placeholder="不含 @ 的账户 handle"
                @keydown.down.prevent="moveActive(1)"
                @keydown.up.prevent="moveActive(-1)"
                @keydown.enter.prevent="applyActive"
                @keydown.esc="closeCombo"
              />
              <button
                type="button"
                class="combo-toggle"
                :class="{ open: comboOpen }"
                tabindex="-1"
                aria-label="展开历史账户"
                @click="toggleCombo"
              >
                <svg width="10" height="6" viewBox="0 0 10 6" aria-hidden="true">
                  <path d="M1 1l4 4 4-4" fill="none" stroke="currentColor" stroke-width="1.6"
                    stroke-linecap="round" stroke-linejoin="round" />
                </svg>
              </button>
              <div v-if="comboOpen" class="combo-list">
                <div
                  v-for="(id, i) in recent_ids"
                  :key="id"
                  class="combo-item"
                  :class="{ active: i === activeIndex }"
                  @mouseenter="activeIndex = i"
                  @mousedown.prevent="selectId(id)"
                >{{ id }}</div>
                <div v-if="!recent_ids.length" class="combo-empty">暂无历史记录</div>
              </div>
            </div>
          </div>
        </div>

        <label class="row">
          <span class="row-label">保存位置:</span>
          <div class="row-value path-row">
            <input v-model.trim="save_path" type="text" />
            <button class="btn-ghost" @click="browseFolder">选择…</button>
          </div>
        </label>

        <label class="row">
          <span class="row-label">auth_token:</span>
          <div class="row-value"><input v-model.trim="auth_token" type="text" /></div>
        </label>

        <label class="row">
          <span class="row-label">ct0:</span>
          <div class="row-value"><input v-model.trim="ct0" type="text" /></div>
        </label>

        <div class="row">
          <span class="row-label">时间范围:</span>
          <div class="row-value date-row">
            <div class="date-group">
              <span class="date-tag">起始</span>
              <select v-model="sy" :disabled="noLimit" @change="refreshDays('s')"><option v-for="y in years" :key="y" :value="y">{{ y }}</option></select>
              <select v-model="sm" :disabled="noLimit" @change="refreshDays('s')"><option v-for="m in months" :key="m" :value="m">{{ m }}</option></select>
              <select v-model="sd" :disabled="noLimit"><option v-for="d in startDays" :key="d" :value="d">{{ d }}</option></select>
            </div>
            <span class="date-sep">至</span>
            <div class="date-group">
              <span class="date-tag">结束</span>
              <select v-model="ey" :disabled="noLimit" @change="refreshDays('e')"><option v-for="y in years" :key="y" :value="y">{{ y }}</option></select>
              <select v-model="em" :disabled="noLimit" @change="refreshDays('e')"><option v-for="m in months" :key="m" :value="m">{{ m }}</option></select>
              <select v-model="ed" :disabled="noLimit"><option v-for="d in endDays" :key="d" :value="d">{{ d }}</option></select>
            </div>
            <label class="limit-check">
              <input type="checkbox" v-model="noLimit" />
              <span>不限时间</span>
            </label>
          </div>
        </div>
      </div>
    </section>

    <!-- 操作区 -->
    <section class="card">
      <div class="action-bar">
        <button class="btn-primary" :disabled="running" @click="startDownload">开始下载</button>
        <button class="btn-danger" :disabled="!running || cancelling" @click="cancelDownload">取消下载</button>

        <span class="action-label">类型</span>
        <div class="segment">
          <button v-for="opt in ['全部媒体', '仅图片', '仅视频']" :key="opt" class="segment-btn"
            :class="{ active: media_filter_label === opt }" @click="media_filter_label = opt">{{ opt }}</button>
        </div>

        <span class="action-label">线程数</span>
        <select v-model.number="concurrency" class="concurrency-select">
          <option v-for="n in 32" :key="n" :value="n">{{ n }}</option>
        </select>

        <button class="btn-ghost" @click="openFolder">打开目录</button>
      </div>

      <div class="progress-row">
        <div class="progress-track">
          <div class="progress-fill" :style="{ width: progressPercent + '%' }"></div>
        </div>
        <span class="progress-text">{{ progressText }}</span>
        <span class="status" :style="{ color: statusColor }">状态: {{ statusText }}</span>
      </div>

      <div class="stats-grid">
        <div class="stat" v-for="s in statCards" :key="s.label">
          <div class="stat-strip" :style="{ background: s.color }"></div>
          <div class="stat-value" :style="{ color: s.color }">{{ s.value }}</div>
          <div class="stat-unit">{{ s.unit }}</div>
          <div class="stat-label">{{ s.label }}</div>
        </div>
      </div>
    </section>

    <!-- 运行日志 -->
    <section class="card log-card">
      <div class="log-head">
        <span class="log-title">运行日志</span>
        <div class="log-actions">
          <button class="btn-ghost" @click="copyLog">复制</button>
          <button class="btn-ghost" @click="clearLog">清空</button>
        </div>
      </div>
      <div class="log-area" ref="logArea">
        <div v-for="(line, i) in logLines" :key="i" class="log-line" :class="'log-' + line.tag">
          <span class="log-ts">{{ line.ts }}</span>{{ line.msg }}
        </div>
      </div>
    </section>
  </div>
</template>

<script setup>
import { reactive, ref, computed, onMounted, onUnmounted, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

// ---------------- 状态 ----------------
const user_id = ref('ekin9527')
const save_path = ref('E:/x')
const auth_token = ref('')
const ct0 = ref('')
const concurrency = ref(16)
const media_filter_label = ref('全部媒体')
const recent_ids = ref([])

const running = ref(false)
const cancelling = ref(false)
const statusText = ref('就绪')
const statusColor = ref('var(--secondary)')

const stat = reactive({ down: 0, skip: 0, fail: 0 })
const speedValue = ref('0')
const speedUnit = ref('B/s')
const progress = reactive({ current: 0, total: 0 })

const logLines = ref([])
const logArea = ref(null)
let unlisteners = []

// ---------------- 账户 ID 下拉框（自定义组合框，非 datalist 气泡）----------------
const comboRef = ref(null)
const comboOpen = ref(false)
const activeIndex = ref(-1)

function openCombo() {
  // 即使无历史记录也展开，显示「暂无历史记录」，保证它始终是真正的下拉列表
  comboOpen.value = true
  activeIndex.value = -1
}
function closeCombo() {
  comboOpen.value = false
  activeIndex.value = -1
}
function toggleCombo() {
  comboOpen.value ? closeCombo() : openCombo()
}
function selectId(id) {
  user_id.value = id
  closeCombo()
}
function moveActive(delta) {
  if (!comboOpen.value) openCombo()
  const n = recent_ids.value.length
  if (!n) return
  activeIndex.value = activeIndex.value < 0
    ? (delta > 0 ? 0 : n - 1)
    : (activeIndex.value + delta + n) % n
}
function applyActive() {
  if (comboOpen.value && activeIndex.value >= 0 && recent_ids.value[activeIndex.value]) {
    selectId(recent_ids.value[activeIndex.value])
  }
}
function onDocumentMouseDown(ev) {
  if (comboRef.value && !comboRef.value.contains(ev.target)) closeCombo()
}

// ---------------- 时间范围选择 ----------------
const noLimit = ref(true)
const now = new Date()
const years = Array.from({ length: now.getFullYear() - 1990 + 1 }, (_, i) => String(1990 + i))
const months = Array.from({ length: 12 }, (_, i) => String(i + 1))
const sy = ref('1990')
const sm = ref('1')
const sd = ref('1')
const ey = ref(String(now.getFullYear()))
const em = ref(String(now.getMonth() + 1))
const ed = ref(String(now.getDate()))
const startDays = ref([])
const endDays = ref([])

function daysInMonth(y, m) {
  return new Date(Number(y), Number(m), 0).getDate()
}
function refreshDays(which) {
  const dim = daysInMonth(which === 's' ? sy.value : ey.value, which === 's' ? sm.value : em.value)
  const list = Array.from({ length: dim }, (_, i) => String(i + 1))
  if (which === 's') {
    startDays.value = list
    if (Number(sd.value) > dim) sd.value = String(dim)
  } else {
    endDays.value = list
    if (Number(ed.value) > dim) ed.value = String(dim)
  }
}
refreshDays('s')
refreshDays('e')

// 对应 Python _compute_time_range()：越界收敛到 [1990-01-01, 当天]，起止倒序自动交换
function toDate(y, m, d) {
  y = Number(y); m = Number(m); d = Number(d)
  m = Math.max(1, Math.min(12, m))
  const dim = daysInMonth(y, m)
  d = Math.max(1, Math.min(dim, d))
  let dt = new Date(y, m - 1, d)
  const min = new Date(1990, 0, 1)
  const today = new Date()
  today.setHours(0, 0, 0, 0)
  if (dt < min) dt = min
  if (dt > today) dt = today
  return dt
}
function computeTimeRange() {
  if (noLimit.value) return ''
  let s = toDate(sy.value, sm.value, sd.value)
  let e = toDate(ey.value, em.value, ed.value)
  if (s > e) [s, e] = [e, s]
  const fmt = (dt) =>
    `${dt.getFullYear()}-${String(dt.getMonth() + 1).padStart(2, '0')}-${String(dt.getDate()).padStart(2, '0')}`
  return `${fmt(s)}:${fmt(e)}`
}

// 对应 Python remember_recent_user_ids()
function rememberRecentIds(history, current) {
  const result = []
  for (const value of [current, ...(history || [])]) {
    if (typeof value !== 'string') continue
    const v = value.trim()
    if (v && !result.includes(v)) result.push(v)
    if (result.length >= 7) break
  }
  return result
}

// ---------------- 速度格式化（对应 format_download_speed） ----------------
function formatSpeed(bps) {
  let speed = Math.max(0, Number(bps) || 0)
  const units = ['B/s', 'KB/s', 'MB/s', 'GB/s']
  let i = 0
  while (speed >= 1024 && i < units.length - 1) {
    speed /= 1024
    i++
  }
  if (i === 0) return [String(Math.floor(speed)), units[i]]
  return [speed.toFixed(1), units[i]]
}

// ---------------- 日志 ----------------
function detectTag(msg) {
  if (msg.startsWith('[成功]')) return 'success'
  if (msg.startsWith('[跳过]')) return 'skip'
  if (msg.startsWith('[错误]') || msg.startsWith('[失败]')) return 'error'
  if (msg.startsWith('[重试]') || msg.startsWith('[警告]')) return 'warn'
  if (msg.startsWith('[提示]') || msg.startsWith('[信息]')) return 'info'
  return 'normal'
}
function printLog(msg, tag = null) {
  const ts = new Date().toTimeString().slice(0, 8)
  logLines.value.push({ ts, msg, tag: tag || detectTag(msg) })
  if (logLines.value.length > 5000) logLines.value.splice(0, logLines.value.length - 5000)
  nextTick(() => {
    if (logArea.value) logArea.value.scrollTop = logArea.value.scrollHeight
  })
}
function clearLog() {
  logLines.value = []
}
// 原生弹窗（对应 Python 版 messagebox）
async function dialog(title, message, level = 'warning') {
  try {
    await invoke('show_message', { title, message, level })
  } catch (e) {
    printLog(`[错误] ${e}`, 'error')
  }
}

async function copyLog() {
  const text = logLines.value.map((l) => `[${l.ts}] ${l.msg}`).join('\n')
  try {
    await navigator.clipboard.writeText(text)
    printLog('[提示] 日志已复制到剪贴板。', 'info')
  } catch {
    dialog('错误', '复制日志失败: 剪贴板不可用', 'error')
  }
}

// ---------------- 进度/统计 ----------------
const progressPercent = computed(() => {
  if (!progress.total) return 0
  return Math.min((progress.current / progress.total) * 100, 100)
})
const progressText = computed(() => {
  if (!progress.total) return '0%'
  const pct = Math.floor(progressPercent.value)
  return `${pct}%  (${progress.current}/${progress.total})`
})
const statCards = computed(() => [
  { label: '已下载', value: stat.down, unit: '项', color: 'var(--success)' },
  { label: '已跳过(重复)', value: stat.skip, unit: '项', color: 'var(--warn)' },
  { label: '失败', value: stat.fail, unit: '项', color: 'var(--danger)' },
  { label: '实时速度', value: speedValue.value, unit: speedUnit.value, color: 'var(--accent)' },
])

// ---------------- 配置加载/保存 ----------------
onMounted(async () => {
  try {
    const cfg = await invoke('get_config')
    if (cfg.user_id) user_id.value = cfg.user_id
    if (cfg.save_path) save_path.value = cfg.save_path
    if (cfg.auth_token) auth_token.value = cfg.auth_token
    if (cfg.ct0) ct0.value = cfg.ct0
    if (cfg.concurrency >= 1 && cfg.concurrency <= 32) concurrency.value = cfg.concurrency
    if (cfg.media_filter_label) media_filter_label.value = cfg.media_filter_label
    if (Array.isArray(cfg.recent_user_ids)) recent_ids.value = cfg.recent_user_ids
    if (cfg.time_range && cfg.time_range.includes(':')) {
      try {
        const [s, e] = cfg.time_range.split(':')
        const [sy_, sm_, sd_] = s.split('-').map(Number)
        const [ey_, em_, ed_] = e.split('-').map(Number)
        sy.value = String(sy_); sm.value = String(sm_); sd.value = String(sd_)
        ey.value = String(ey_); em.value = String(em_); ed.value = String(ed_)
        noLimit.value = false
        refreshDays('s'); refreshDays('e')
      } catch { /* 回填失败保持默认 */ }
    }
  } catch { /* 首次运行无配置 */ }

  printLog('欢迎使用。', 'info')
  printLog('基于媒体唯一 ID 去重，重复运行不会重复下载；回车开始 / Esc 取消。', 'info')

  // 后端事件
  unlisteners.push(
    await listen('xdl-log', (e) => printLog(e.payload)),
    await listen('xdl-stats', (e) => {
      stat.down = e.payload.down
      stat.skip = e.payload.skip
      stat.fail = e.payload.fail
    }),
    await listen('xdl-progress', (e) => {
      progress.current = e.payload.current
      progress.total = e.payload.total
    }),
    await listen('xdl-speed', (e) => {
      const [v, u] = formatSpeed(e.payload)
      speedValue.value = v
      speedUnit.value = u
    }),
    await listen('xdl-done', (e) => onFinished(e.payload.cancelled)),
  )

  // 快捷键：回车开始 / Esc 取消（下拉展开时回车用于选中，不触发下载）
  window.addEventListener('keydown', onKeydown)
  document.addEventListener('mousedown', onDocumentMouseDown)
})

onUnmounted(() => {
  unlisteners.forEach((fn) => fn())
  window.removeEventListener('keydown', onKeydown)
  document.removeEventListener('mousedown', onDocumentMouseDown)
})

function onKeydown(ev) {
  if (ev.key === 'Enter' && !running.value && !comboOpen.value) startDownload()
  else if (ev.key === 'Escape') cancelDownload()
}

// ---------------- 操作 ----------------
function browseFolder() {
  invoke('browse_folder', { defaultPath: save_path.value }).then((folder) => {
    if (folder) save_path.value = folder
  })
}

async function openFolder() {
  const base = save_path.value.replace(/[\\/]+$/, '') // 去掉末尾多余分隔符，避免 explorer 跳到"文档"
  const uid = user_id.value.replace(/^@+/, '').trim()
  const folder = uid ? `${base}/${uid}` : base
  try {
    await invoke('open_folder', { path: folder })
  } catch (e) {
    // 对应 Python: showinfo("该目录尚不存在") / showerror(f"无法打开目录: {e}")
    const level = String(e).includes('尚不存在') ? 'info' : 'error'
    dialog('提示', String(e).includes('尚不存在') ? '该目录尚不存在，请先执行下载。' : `无法打开目录: ${e}`, level)
  }
}

async function startDownload() {
  if (running.value) return
  if (!user_id.value) return dialog('提示', '请输入账户 ID！')
  if (!save_path.value) return dialog('提示', '请输入保存位置！')
  if (!auth_token.value) return dialog('提示', '请输入 Cookie 中的 auth_token！')
  if (!ct0.value) return dialog('提示', '请输入 Cookie 中的 ct0！')

  const time_range = computeTimeRange()
  const typeMap = { 全部媒体: 'all', 仅图片: 'image', 仅视频: 'video' }
  const media_filter = typeMap[media_filter_label.value] || 'all'

  // 持久化配置（字段与 Python 版 config.json 完全一致）
  try {
    await invoke('save_config', {
      cfg: {
        auth_token: auth_token.value,
        ct0: ct0.value,
        user_id: user_id.value,
        save_path: save_path.value,
        concurrency: concurrency.value,
        media_filter_label: media_filter_label.value,
        time_range: time_range,
        recent_user_ids: rememberRecentIds(recent_ids.value, user_id.value),
      },
    })
    recent_ids.value = rememberRecentIds(recent_ids.value, user_id.value)
  } catch (e) {
    printLog(`[错误] 保存配置失败: ${e}`, 'error')
  }

  running.value = true
  cancelling.value = false
  statusText.value = '正在下载…'
  statusColor.value = 'var(--accent)'
  progress.current = 0
  progress.total = 0
  stat.down = 0
  stat.skip = 0
  stat.fail = 0
  speedValue.value = '0'
  speedUnit.value = 'B/s'
  logLines.value = []

  try {
    await invoke('start_download', {
      params: {
        auth_token: auth_token.value,
        ct0: ct0.value,
        user_id: user_id.value,
        save_path: save_path.value,
        concurrency: concurrency.value,
        media_filter,
        time_range,
      },
    })
  } catch (e) {
    running.value = false
    printLog(`[错误] 启动下载失败: ${e}`, 'error')
    statusText.value = '就绪'
    statusColor.value = 'var(--secondary)'
  }
}

async function cancelDownload() {
  if (!running.value || cancelling.value) return
  cancelling.value = true
  statusText.value = '正在取消…'
  statusColor.value = 'var(--warn)'
  printLog('\n>>> 收到取消指令，正在中断队列…', 'warn')
  try {
    await invoke('cancel_download')
  } catch { /* 忽略 */ }
}

function onFinished(cancelled) {
  running.value = false
  cancelling.value = false
  speedValue.value = '0'
  speedUnit.value = 'B/s'
  if (cancelled) {
    statusText.value = '已取消'
    statusColor.value = 'var(--danger)'
  } else {
    statusText.value = '已完成'
    statusColor.value = 'var(--success)'
  }
}
</script>

<style scoped>
.app-root {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 0;
  flex: 1;
  overflow: hidden;
}

/* 上方卡片固定高度, 日志卡自适应剩余空间且可收缩, 避免窗口缩小时内容溢出重叠 */
.card:not(.log-card) {
  flex-shrink: 0;
}

.config-body {
  padding: 0 18px 14px;
}

.row {
  display: flex;
  align-items: center;
  padding: 8px 0;
  gap: 12px;
}

.row-label {
  width: 90px;
  flex-shrink: 0;
  font-weight: bold;
  color: var(--text);
}

.row-value {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 8px;
}

.row-value input[type='text'] {
  flex: 1;
  min-width: 0;
}

.path-row button {
  flex-shrink: 0;
}

/* 账户 ID 下拉框：真正的弹出列表，取代 datalist 气泡 */
.combo {
  position: relative;
  flex: 1;
  min-width: 0;
}

.combo input[type='text'] {
  width: 100%;
  padding-right: 30px;
}

.combo-toggle {
  position: absolute;
  top: 1px;
  right: 1px;
  bottom: 1px;
  width: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  color: var(--secondary);
  border-radius: 0 8px 8px 0;
}

.combo-toggle:hover {
  background: var(--btn-ghost-hover);
  color: var(--text);
}

.combo-toggle.open svg {
  transform: rotate(180deg);
}

.combo-list {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  right: 0;
  z-index: 50;
  background: var(--entry-bg);
  border: 1px solid var(--entry-border);
  border-radius: 8px;
  padding: 4px;
  max-height: 190px;
  overflow-y: auto;
}

.combo-item {
  padding: 7px 10px;
  border-radius: 6px;
  font-family: Consolas, 'Courier New', monospace;
  font-size: 13px;
  color: var(--text);
  cursor: pointer;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.combo-item.active {
  background: var(--accent);
  color: #fff;
}

.combo-empty {
  padding: 8px 10px;
  font-size: 12px;
  color: var(--secondary);
}

.date-row {
  flex-wrap: wrap;
  row-gap: 8px;
  gap: 8px;
}

/* 起止日期分组，各自成框，避免 6 个下拉框挤在一起 */
.date-group {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 2px 6px 2px 4px;
  background: var(--entry-bg);
  border: 1px solid var(--entry-border);
  border-radius: 8px;
}

.date-group select {
  width: 56px;
  padding: 5px 2px;
  text-align: center;
  font-family: 'Microsoft YaHei', sans-serif;
  background: transparent;
  border: none;
}

.date-group select:hover:not(:disabled) {
  background: var(--btn-ghost-hover);
  border-radius: 5px;
}

.date-tag {
  font-size: 12px;
  color: var(--secondary);
  margin-right: 2px;
}

.date-sep {
  font-size: 12px;
  color: var(--secondary);
}

.limit-check {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  cursor: pointer;
  margin-left: 8px;
  color: var(--text);
  user-select: none;
}

.limit-check input {
  accent-color: var(--accent);
  width: 15px;
  height: 15px;
  cursor: pointer;
}

/* 操作区 */
.action-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 16px 18px 12px;
  flex-wrap: wrap;
  row-gap: 10px;
}

.action-label {
  font-weight: bold;
  margin-left: 4px;
}

.segment {
  display: flex;
  background: var(--btn-ghost-bg);
  border-radius: 10px;
  overflow: hidden;
}

.segment-btn {
  background: var(--card-bg);
  color: var(--text);
  font-size: 12px;
  font-weight: normal;
  padding: 7px 12px;
  border-radius: 0;
}

.segment-btn:first-child {
  border-radius: 10px 0 0 10px;
}

.segment-btn:last-child {
  border-radius: 0 10px 10px 0;
}

.segment-btn.active {
  background: var(--accent);
  color: #fff;
  font-weight: bold;
}

.segment-btn:not(.active):hover {
  background: var(--btn-ghost-hover);
}

.concurrency-select {
  width: 64px;
  padding: 6px;
  font-family: 'Microsoft YaHei', sans-serif;
}

.status {
  margin-left: auto;
  font-size: 12px;
  font-weight: bold;
  color: var(--secondary);
  white-space: nowrap;
  flex-shrink: 0;
}

.progress-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 0 18px 14px;
}

.progress-track {
  flex: 1;
  height: 10px;
  background: var(--strip-inactive);
  border-radius: 5px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: var(--accent);
  border-radius: 5px;
  transition: width 0.25s ease;
}

.progress-text {
  font-weight: bold;
  color: var(--secondary);
  white-space: nowrap;
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 6px;
  padding: 0 18px 16px;
}

.stat {
  background: var(--card-bg);
  border: 1px solid var(--border);
  border-radius: 10px;
  overflow: hidden;
  text-align: center;
}

.stat-strip {
  height: 4px;
}

.stat-value {
  font-size: 23px;
  font-weight: bold;
  margin-top: 5px;
  line-height: 1.3;
}

.stat-unit {
  font-size: 10px;
  color: var(--secondary);
  line-height: 1.3;
}

.stat-label {
  font-size: 11px;
  color: var(--secondary);
  padding-bottom: 9px;
}

/* 日志区 */
.log-card {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 180px;
}

.log-head {
  display: flex;
  align-items: center;
  padding: 14px 18px 8px;
}

.log-title {
  font-size: 13px;
  font-weight: bold;
  color: var(--secondary);
}

.log-actions {
  margin-left: auto;
  display: flex;
  gap: 6px;
}

.log-actions button {
  padding: 5px 12px;
  font-size: 12px;
  border-radius: 10px;
}

.log-area {
  flex: 1;
  margin: 0 18px 16px;
  background: var(--log-bg);
  border-radius: 10px;
  padding: 10px 12px;
  overflow-y: auto;
  font-family: Consolas, 'Courier New', monospace;
  font-size: 13px;
  white-space: pre-wrap;
  word-break: break-all;
}

.log-line {
  line-height: 1.7;
}

.log-ts {
  color: var(--secondary);
  margin-right: 8px;
}

.log-success { color: #00ba7c; }
.log-skip { color: #ffd400; }
.log-error { color: #ff6b8a; }
.log-warn { color: #ffd400; }
.log-info { color: #8b98a5; }
.log-normal { color: #d4d4d4; }
</style>
