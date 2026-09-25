<template>
  <div class="app-root">
    <!-- WinUI 3 顶部应用标题栏 -->
    <header class="win-app-header">
      <div class="win-app-brand">
        <svg class="win-app-icon" viewBox="0 0 24 24" width="18" height="18" fill="none">
          <rect width="24" height="24" rx="6" fill="#60cdff" fill-opacity="0.16" />
          <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z" fill="#60cdff"/>
        </svg>
        <span class="win-app-title">X 媒体下载器</span>
        <span class="win-badge">WinUI 3</span>
      </div>
    </header>

    <!-- 卡片 1：账户与下载设置 (WinUI 3 现代质感卡片) -->
    <section class="win-card">
      <div class="win-card-header">
        <div class="win-header-left">
          <div class="win-card-title-row">
            <svg class="win-section-icon" viewBox="0 0 20 20" width="16" height="16" fill="currentColor">
              <path d="M10 2a4 4 0 100 8 4 4 0 000-8zm-2 4a2 2 0 114 0 2 2 0 01-4 0zm-4 9a4 4 0 014-4h4a4 4 0 014 4v1H4v-1zm2 0a2 2 0 012-2h4a2 2 0 012 2v0H6v0z"/>
            </svg>
            <span class="win-card-title">账户与检索配置</span>
          </div>
          <span class="win-card-subtitle">配置 X 账户凭据、存储路径及时间范围</span>
        </div>
      </div>

      <div class="win-card-body">
        <!-- 账户 ID -->
        <div class="win-form-row">
          <span class="win-label">账户 ID:</span>
          <div class="win-control-wrapper">
            <div class="win-combo" ref="comboRef">
              <input
                v-model.trim="user_id"
                class="win-textbox"
                type="text"
                placeholder="不含 @ 的账户 handle"
                @keydown.down.prevent="moveActive(1)"
                @keydown.up.prevent="moveActive(-1)"
                @keydown.enter.prevent="applyActive"
                @keydown.esc="closeCombo"
              />
              <button
                type="button"
                class="win-combo-toggle"
                :class="{ open: comboOpen }"
                tabindex="-1"
                aria-label="展开历史账户"
                @click="toggleCombo"
              >
                <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
                  <path d="M2.5 4.5L6 8L9.5 4.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
              </button>

              <!-- WinUI 3 Flyout 下拉层 -->
              <div v-if="comboOpen" class="win-flyout">
                <div
                  v-for="(id, i) in recent_ids"
                  :key="id"
                  class="win-flyout-item"
                  :class="{ active: i === activeIndex }"
                  @mouseenter="activeIndex = i"
                  @mousedown.prevent="selectId(id)"
                >
                  <span class="win-flyout-indicator"></span>
                  <span class="win-flyout-text">{{ id }}</span>
                </div>
                <div v-if="!recent_ids.length" class="win-flyout-empty">暂无历史记录</div>
              </div>
            </div>
          </div>
        </div>

        <!-- 保存位置 -->
        <div class="win-form-row zebra">
          <span class="win-label">保存位置:</span>
          <div class="win-control-wrapper win-path-group">
            <input v-model.trim="save_path" class="win-textbox" type="text" placeholder="选择媒体文件保存目录..." />
            <button class="win-btn win-btn-standard" @click="browseFolder">
              <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
                <path d="M1.5 2.5A1.5 1.5 0 013 1h3.086a1.5 1.5 0 011.06.44l1.414 1.414A1.5 1.5 0 009.62 3.25H13A1.5 1.5 0 0114.5 4.75v8.5A1.5 1.5 0 0113 14.75H3A1.5 1.5 0 011.5 13.25v-10.75zm1.5-.5a.5.5 0 00-.5.5v10.75a.5.5 0 00.5.5h10a.5.5 0 00.5-.5v-8.5a.5.5 0 00-.5-.5H9.621a2.5 2.5 0 01-1.768-.732L6.44 2.104A.5.5 0 006.086 2H3z"/>
              </svg>
              <span>选择…</span>
            </button>
          </div>
        </div>

        <!-- auth_token -->
        <div class="win-form-row">
          <span class="win-label">auth_token:</span>
          <div class="win-control-wrapper">
            <input v-model.trim="auth_token" class="win-textbox" type="text" placeholder="Cookie 中的 auth_token" />
          </div>
        </div>

        <!-- ct0 -->
        <div class="win-form-row zebra">
          <span class="win-label">ct0:</span>
          <div class="win-control-wrapper">
            <input v-model.trim="ct0" class="win-textbox" type="text" placeholder="Cookie 中的 ct0" />
          </div>
        </div>

        <!-- 时间范围 -->
        <div class="win-form-row">
          <span class="win-label">时间范围:</span>
          <div class="win-control-wrapper win-date-container">
            <div class="win-date-picker-group">
              <span class="win-date-tag">起始</span>
              <select v-model="sy" class="win-select" :disabled="noLimit" @change="refreshDays('s')">
                <option v-for="y in years" :key="y" :value="y">{{ y }}</option>
              </select>
              <select v-model="sm" class="win-select" :disabled="noLimit" @change="refreshDays('s')">
                <option v-for="m in months" :key="m" :value="m">{{ m }}</option>
              </select>
              <select v-model="sd" class="win-select" :disabled="noLimit">
                <option v-for="d in startDays" :key="d" :value="d">{{ d }}</option>
              </select>
            </div>

            <span class="win-date-separator">至</span>

            <div class="win-date-picker-group">
              <span class="win-date-tag">结束</span>
              <select v-model="ey" class="win-select" :disabled="noLimit" @change="refreshDays('e')">
                <option v-for="y in years" :key="y" :value="y">{{ y }}</option>
              </select>
              <select v-model="em" class="win-select" :disabled="noLimit" @change="refreshDays('e')">
                <option v-for="m in months" :key="m" :value="m">{{ m }}</option>
              </select>
              <select v-model="ed" class="win-select" :disabled="noLimit">
                <option v-for="d in endDays" :key="d" :value="d">{{ d }}</option>
              </select>
            </div>

            <!-- WinUI 3 Checkbox -->
            <label class="win-checkbox">
              <input type="checkbox" v-model="noLimit" />
              <span class="win-checkbox-box">
                <svg v-if="noLimit" width="10" height="8" viewBox="0 0 10 8" fill="none">
                  <path d="M1 3.5L3.5 6L9 1" stroke="#000" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
              </span>
              <span class="win-checkbox-label">不限时间</span>
            </label>
          </div>
        </div>
      </div>
    </section>

    <!-- 卡片 2：操作与监控 (参考图吧工具箱指标卡片质感) -->
    <section class="win-card">
      <div class="win-action-bar">
        <!-- 开始下载 (WinUI 3 Accent Button) -->
        <button class="win-btn win-btn-accent" :disabled="running" @click="startDownload">
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
            <path d="M8 12l4-4h-2.5V2h-3v6H4l4 4z"/>
            <path d="M2 13.5v1h12v-1H2z"/>
          </svg>
          <span>开始下载</span>
        </button>

        <!-- 取消下载 (WinUI 3 Destructive Button) -->
        <button class="win-btn win-btn-danger" :disabled="!running || cancelling" @click="cancelDownload">
          <svg width="13" height="13" viewBox="0 0 16 16" fill="currentColor">
            <rect x="3" y="3" width="10" height="10" rx="1.5"/>
          </svg>
          <span>取消下载</span>
        </button>

        <span class="win-divider"></span>

        <!-- 类型选择 (WinUI 3 SegmentedControl) -->
        <span class="win-section-label">类型</span>
        <div class="win-segmented">
          <button
            v-for="opt in ['全部媒体', '仅图片', '仅视频']"
            :key="opt"
            class="win-segmented-item"
            :class="{ active: media_filter_label === opt }"
            @click="media_filter_label = opt"
          >
            {{ opt }}
          </button>
        </div>

        <span class="win-divider"></span>

        <!-- 线程数选择 -->
        <span class="win-section-label">线程数</span>
        <select v-model.number="concurrency" class="win-select win-concurrency-select">
          <option v-for="n in 32" :key="n" :value="n">{{ n }}</option>
        </select>

        <!-- 打开目录 -->
        <button class="win-btn win-btn-standard" @click="openFolder">
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
            <path d="M1.5 2.5A1.5 1.5 0 013 1h3.086a1.5 1.5 0 011.06.44l1.414 1.414A1.5 1.5 0 009.62 3.25H13A1.5 1.5 0 0114.5 4.75v8.5A1.5 1.5 0 0113 14.75H3A1.5 1.5 0 011.5 13.25v-10.75z"/>
          </svg>
          <span>打开目录</span>
        </button>
      </div>

      <!-- WinUI 3 细长进度条与状态条 -->
      <div class="win-progress-section">
        <div class="win-progress-track">
          <div class="win-progress-fill" :style="{ width: progressPercent + '%' }"></div>
        </div>
        <div class="win-progress-info">
          <span class="win-progress-text">{{ progressText }}</span>
          <!-- WinUI 3 InfoBadge 状态标识 -->
          <div class="win-status-badge">
            <span class="win-status-dot" :style="{ backgroundColor: statusColor }"></span>
            <span class="win-status-text" :style="{ color: statusColor }">{{ statusText }}</span>
          </div>
        </div>
      </div>

      <!-- 统计指标看板 (类似参考图的硬件信息磁贴质感) -->
      <div class="win-stats-grid">
        <!-- 已下载 -->
        <div class="win-stat-tile">
          <div class="win-stat-icon-box" style="background: var(--win-success-subtle); color: var(--win-success);">
            <svg width="18" height="18" viewBox="0 0 16 16" fill="currentColor">
              <path d="M8 12l4-4h-2.5V2h-3v6H4l4 4z"/>
              <path d="M2 13.5v1h12v-1H2z"/>
            </svg>
          </div>
          <div class="win-stat-content">
            <span class="win-stat-category">已下载媒体</span>
            <div class="win-stat-val-row">
              <span class="win-stat-value" style="color: var(--win-success);">{{ stat.down }}</span>
              <span class="win-stat-unit">项</span>
            </div>
          </div>
        </div>

        <!-- 已跳过 -->
        <div class="win-stat-tile">
          <div class="win-stat-icon-box" style="background: var(--win-warning-subtle); color: var(--win-warning);">
            <svg width="18" height="18" viewBox="0 0 16 16" fill="currentColor">
              <path d="M4.5 3a.5.5 0 00-.5.5v9a.5.5 0 00.757.429l6-4.5a.5.5 0 000-.858l-6-4.5A.5.5 0 004.5 3zm7 0a.5.5 0 00-.5.5v9a.5.5 0 001 0v-9a.5.5 0 00-.5-.5z"/>
            </svg>
          </div>
          <div class="win-stat-content">
            <span class="win-stat-category">已跳过 (重复)</span>
            <div class="win-stat-val-row">
              <span class="win-stat-value" style="color: var(--win-warning);">{{ stat.skip }}</span>
              <span class="win-stat-unit">项</span>
            </div>
          </div>
        </div>

        <!-- 失败 -->
        <div class="win-stat-tile">
          <div class="win-stat-icon-box" style="background: var(--win-danger-bg); color: var(--win-danger);">
            <svg width="18" height="18" viewBox="0 0 16 16" fill="currentColor">
              <path d="M8 1a7 7 0 100 14A7 7 0 008 1zm0 10.5a.75.75 0 110-1.5.75.75 0 010 1.5zm.75-7.75v5h-1.5v-5h1.5z"/>
            </svg>
          </div>
          <div class="win-stat-content">
            <span class="win-stat-category">下载失败</span>
            <div class="win-stat-val-row">
              <span class="win-stat-value" style="color: var(--win-danger);">{{ stat.fail }}</span>
              <span class="win-stat-unit">项</span>
            </div>
          </div>
        </div>

        <!-- 实时速度 -->
        <div class="win-stat-tile">
          <div class="win-stat-icon-box" style="background: var(--win-accent-subtle); color: var(--win-accent);">
            <svg width="18" height="18" viewBox="0 0 16 16" fill="currentColor">
              <path d="M8 2a6 6 0 00-6 6c0 1.887.87 3.57 2.235 4.675.244.198.59.186.82-.045.247-.247.23-.65-.035-.87A4.75 4.75 0 013.25 8a4.75 4.75 0 119.5 0c0 1.488-.678 2.818-1.74 3.702-.262.219-.281.62-.036.868.228.23.575.242.82.045A6.002 6.002 0 008 2zm1.28 4.72a.75.75 0 00-1.06 0L6.47 8.47a.75.75 0 101.06 1.06l1.75-1.75a.75.75 0 000-1.06z"/>
            </svg>
          </div>
          <div class="win-stat-content">
            <span class="win-stat-category">实时下载速度</span>
            <div class="win-stat-val-row">
              <span class="win-stat-value" style="color: var(--win-accent);">{{ speedValue }}</span>
              <span class="win-stat-unit">{{ speedUnit }}</span>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- 卡片 3：运行日志控制台 -->
    <section class="win-card win-log-card">
      <div class="win-log-header">
        <div class="win-header-left">
          <div class="win-card-title-row">
            <svg class="win-section-icon" viewBox="0 0 16 16" width="15" height="15" fill="currentColor">
              <path d="M2.5 1.5A1.5 1.5 0 001 3v10a1.5 1.5 0 001.5 1.5h11A1.5 1.5 0 0015 13V3a1.5 1.5 0 00-1.5-1.5h-11zm2.146 4.146a.5.5 0 01.708 0L7.5 7.793l2.146-2.147a.5.5 0 01.708.708l-2.5 2.5a.5.5 0 01-.708 0l-2.5-2.5a.5.5 0 010-.708zM4 11h8a.5.5 0 010 1H4a.5.5 0 010-1z"/>
            </svg>
            <span class="win-card-title">运行日志控制台</span>
          </div>
          <span class="win-card-subtitle">实时输出 GraphQL 解析与文件落盘状态</span>
        </div>
        <div class="win-log-actions">
          <button class="win-btn win-btn-standard win-btn-sm" @click="copyLog">
            <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
              <path d="M4 1.5A1.5 1.5 0 015.5 0h6A1.5 1.5 0 0113 1.5v1H5.5A2.5 2.5 0 003 5v7H2.5A1.5 1.5 0 011 10.5v-9z"/>
              <path d="M4.5 4A1.5 1.5 0 016 2.5h6A1.5 1.5 0 0113.5 4v10a1.5 1.5 0 01-1.5 1.5H6A1.5 1.5 0 014.5 14V4zm1.5-.5a.5.5 0 00-.5.5v10a.5.5 0 00.5.5h6a.5.5 0 00.5-.5V4a.5.5 0 00-.5-.5H6z"/>
            </svg>
            <span>复制</span>
          </button>
          <button class="win-btn win-btn-standard win-btn-sm" @click="clearLog">
            <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
              <path d="M5.5 5.5A.5.5 0 016 6v6a.5.5 0 01-1 0V6a.5.5 0 01.5-.5zm2.5 0a.5.5 0 01.5.5v6a.5.5 0 01-1 0V6a.5.5 0 01.5-.5zm3 .5a.5.5 0 00-1 0v6a.5.5 0 001 0V6z"/>
              <path fill-rule="evenodd" d="M14.5 3a1 1 0 01-1 1H13v9a2 2 0 01-2 2H5a2 2 0 01-2-2V4h-.5a1 1 0 01-1-1V2a1 1 0 011-1H6a1 1 0 011-1h2a1 1 0 011 1h3.5a1 1 0 011 1v1zM4.118 4L4 4.059V13a1 1 0 001 1h6a1 1 0 001-1V4.059L11.882 4H4.118zM2.5 3V2h11v1h-11z"/>
            </svg>
            <span>清空</span>
          </button>
        </div>
      </div>

      <div class="win-log-terminal" ref="logArea">
        <div v-for="(line, i) in logLines" :key="i" class="win-log-line" :class="'win-log-' + line.tag">
          <span class="win-log-ts">{{ line.ts }}</span>
          <span class="win-log-msg">{{ line.msg }}</span>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup>
import { reactive, ref, computed, onMounted, onUnmounted, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

// ---------------- 状态定义 ----------------
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
const statusColor = ref('var(--win-text-secondary)')

const stat = reactive({ down: 0, skip: 0, fail: 0 })
const speedValue = ref('0')
const speedUnit = ref('B/s')
const progress = reactive({ current: 0, total: 0 })

const logLines = ref([])
const logArea = ref(null)
let unlisteners = []

// ---------------- 账户 ID 下拉框（WinUI 3 Flyout 风格） ----------------
const comboRef = ref(null)
const comboOpen = ref(false)
const activeIndex = ref(-1)

function openCombo() {
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

// ---------------- 速度格式化 ----------------
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

// ---------------- 日志系统 ----------------
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

// 原生弹窗
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

// ---------------- 进度计算 ----------------
const progressPercent = computed(() => {
  if (!progress.total) return 0
  return Math.min((progress.current / progress.total) * 100, 100)
})
const progressText = computed(() => {
  if (!progress.total) return '0%'
  const pct = Math.floor(progressPercent.value)
  return `${pct}% (${progress.current}/${progress.total})`
})

// ---------------- 生命周期与事件监听 ----------------
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

  printLog('欢迎使用 X 媒体下载器 (WinUI 3 体验版)！', 'info')
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

// ---------------- 操作交互 ----------------
function browseFolder() {
  invoke('browse_folder', { defaultPath: save_path.value }).then((folder) => {
    if (folder) save_path.value = folder
  })
}

async function openFolder() {
  const base = save_path.value.replace(/[\\/]+$/, '')
  const uid = user_id.value.replace(/^@+/, '').trim()
  const folder = uid ? `${base}/${uid}` : base
  try {
    await invoke('open_folder', { path: folder })
  } catch (e) {
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
  statusColor.value = 'var(--win-accent)'
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
    statusColor.value = 'var(--win-text-secondary)'
  }
}

async function cancelDownload() {
  if (!running.value || cancelling.value) return
  cancelling.value = true
  statusText.value = '正在取消…'
  statusColor.value = 'var(--win-warning)'
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
    statusColor.value = 'var(--win-danger)'
  } else {
    statusText.value = '已完成'
    statusColor.value = 'var(--win-success)'
  }
}
</script>

<style scoped>
.app-root {
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-height: 0;
  flex: 1;
  overflow: hidden;
}

/* WinUI 3 应用头部品牌 */
.win-app-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1px 4px 5px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
}

.win-app-brand {
  display: flex;
  align-items: center;
  gap: 8px;
}

.win-app-title {
  font-size: 13.5px;
  font-weight: 600;
  color: var(--win-text-primary);
  letter-spacing: 0.2px;
}

.win-badge {
  font-size: 10px;
  font-weight: 600;
  padding: 1px 6px;
  background: var(--win-accent-subtle);
  color: var(--win-accent);
  border-radius: var(--win-radius-pill);
  border: 1px solid rgba(96, 205, 255, 0.25);
}

/* WinUI 3 现代质感卡片 */
.win-card {
  background: var(--win-card-bg);
  border: 1px solid var(--win-card-stroke);
  border-radius: var(--win-radius-card);
  box-shadow: var(--win-shadow-card);
  backdrop-filter: blur(20px);
  position: relative;
  transition: border-color 0.2s, box-shadow 0.2s;
}

.win-card:hover {
  border-color: var(--win-card-stroke-elevated);
}

.win-card:not(.win-log-card) {
  flex-shrink: 0;
}

/* 类似图吧工具箱的标题与说明布局 */
.win-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px 8px;
}

.win-header-left {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.win-card-title-row {
  display: flex;
  align-items: center;
  gap: 7px;
}

.win-section-icon {
  color: var(--win-accent);
  opacity: 0.9;
}

.win-card-title {
  font-size: 13.5px;
  font-weight: 600;
  color: var(--win-text-primary);
  letter-spacing: 0.1px;
}

.win-card-subtitle {
  font-size: 11px;
  color: var(--win-text-secondary);
}

.win-card-body {
  padding: 2px 10px 8px;
}

/* WinUI 3 表单行 - 融入类似图吧工具箱详细信息的斑马条纹质感 */
.win-form-row {
  display: flex;
  align-items: center;
  padding: 5px 10px;
  border-radius: var(--win-radius-control);
  gap: 12px;
  transition: background 0.15s ease;
}

.win-form-row.zebra {
  background: var(--win-row-zebra);
}

.win-form-row:hover {
  background: var(--win-row-hover);
}

.win-label {
  width: 82px;
  flex-shrink: 0;
  font-size: 12.5px;
  font-weight: 500;
  color: var(--win-text-secondary);
}

.win-control-wrapper {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 8px;
}

.win-path-group {
  display: flex;
  gap: 8px;
}

.win-path-group input {
  flex: 1;
}

/* WinUI 3 TextBox 输入控件 */
.win-textbox {
  width: 100%;
  height: 31px;
  background: var(--win-control-bg);
  color: var(--win-text-primary);
  border: 1px solid var(--win-control-stroke);
  border-bottom: 1px solid var(--win-control-elevation);
  border-radius: var(--win-radius-control);
  padding: 0 10px;
  font-family: Consolas, 'Cascadia Code', monospace;
  font-size: 12.5px;
  outline: none;
  transition: all 0.15s ease;
  box-shadow: var(--win-shadow-control);
}

.win-textbox:hover:not(:disabled) {
  background: var(--win-control-bg-hover);
  border-color: rgba(255, 255, 255, 0.12);
}

.win-textbox:focus {
  background: var(--win-control-bg-focus);
  border-color: var(--win-control-stroke);
  border-bottom: 2px solid var(--win-accent);
  padding-bottom: 1px;
}

.win-textbox::placeholder {
  color: var(--win-text-tertiary);
  font-family: 'Segoe UI', 'Microsoft YaHei', sans-serif;
  font-size: 11.5px;
}

/* WinUI 3 组合框 (ComboBox / Dropdown) */
.win-combo {
  position: relative;
  flex: 1;
  min-width: 0;
}

.win-combo input {
  padding-right: 32px;
}

.win-combo-toggle {
  position: absolute;
  top: 1px;
  right: 1px;
  bottom: 1px;
  width: 30px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  color: var(--win-text-secondary);
  border: none;
  cursor: pointer;
  border-radius: 0 var(--win-radius-control) var(--win-radius-control) 0;
  transition: all 0.15s ease;
}

.win-combo-toggle:hover {
  background: rgba(255, 255, 255, 0.08);
  color: var(--win-text-primary);
}

.win-combo-toggle.open svg {
  transform: rotate(180deg);
}

/* WinUI 3 Flyout 弹出层 */
.win-flyout {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  right: 0;
  z-index: 100;
  background: var(--win-flyout-bg);
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: var(--win-radius-tile);
  box-shadow: var(--win-shadow-flyout);
  padding: 4px;
  max-height: 180px;
  overflow-y: auto;
  backdrop-filter: blur(20px);
}

.win-flyout-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: var(--win-radius-control);
  font-family: Consolas, 'Cascadia Code', monospace;
  font-size: 12.5px;
  color: var(--win-text-primary);
  cursor: pointer;
  transition: background 0.1s;
}

.win-flyout-indicator {
  width: 3px;
  height: 12px;
  background: transparent;
  border-radius: 2px;
  transition: background 0.15s;
}

.win-flyout-item:hover,
.win-flyout-item.active {
  background: rgba(255, 255, 255, 0.08);
}

.win-flyout-item.active .win-flyout-indicator {
  background: var(--win-accent);
}

.win-flyout-text {
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.win-flyout-empty {
  padding: 8px 10px;
  font-size: 12px;
  color: var(--win-text-tertiary);
  text-align: center;
}

/* WinUI 3 日期选择器组 */
.win-date-container {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}

.win-date-picker-group {
  display: flex;
  align-items: center;
  gap: 2px;
  background: var(--win-control-bg);
  border: 1px solid var(--win-control-stroke);
  border-bottom: 1px solid var(--win-control-elevation);
  border-radius: var(--win-radius-control);
  padding: 1px 4px;
}

.win-date-tag {
  font-size: 11px;
  font-weight: 600;
  color: var(--win-text-secondary);
  padding: 0 4px;
}

.win-date-separator {
  font-size: 12px;
  color: var(--win-text-secondary);
}

/* WinUI 3 下拉 Select */
.win-select {
  height: 28px;
  background: transparent;
  color: var(--win-text-primary);
  border: none;
  outline: none;
  font-size: 12px;
  font-family: inherit;
  padding: 0 4px;
  cursor: pointer;
}

.win-select option {
  background: #2a2a2a;
  color: #fff;
}

.win-select:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.06);
  border-radius: 3px;
}

.win-select:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.win-concurrency-select {
  height: 31px;
  width: 58px;
  background: var(--win-control-bg);
  border: 1px solid var(--win-control-stroke);
  border-bottom: 1px solid var(--win-control-elevation);
  border-radius: var(--win-radius-control);
  text-align: center;
}

/* WinUI 3 Checkbox */
.win-checkbox {
  display: flex;
  align-items: center;
  gap: 7px;
  cursor: pointer;
  margin-left: 8px;
  user-select: none;
}

.win-checkbox input {
  display: none;
}

.win-checkbox-box {
  width: 18px;
  height: 18px;
  background: var(--win-control-bg);
  border: 1px solid rgba(255, 255, 255, 0.45);
  border-radius: var(--win-radius-control);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
}

.win-checkbox:hover .win-checkbox-box {
  background: rgba(255, 255, 255, 0.1);
  border-color: rgba(255, 255, 255, 0.6);
}

.win-checkbox input:checked + .win-checkbox-box {
  background: var(--win-accent);
  border-color: var(--win-accent);
}

.win-checkbox-label {
  font-size: 12px;
  color: var(--win-text-primary);
}

/* WinUI 3 按钮规范 (带微光与圆角质感) */
.win-btn {
  height: 31px;
  padding: 0 13px;
  font-size: 12px;
  font-weight: 500;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  border-radius: var(--win-radius-control);
  cursor: pointer;
  transition: all 0.1s cubic-bezier(0, 0, 0, 1);
  user-select: none;
  white-space: nowrap;
}

.win-btn:active:not(:disabled) {
  transform: scale(0.98);
}

.win-btn:disabled {
  opacity: 0.38;
  cursor: not-allowed;
}

/* Standard Button */
.win-btn-standard {
  background: var(--win-control-bg);
  border: 1px solid var(--win-control-stroke);
  border-bottom: 1px solid var(--win-control-elevation);
  color: var(--win-text-primary);
}

.win-btn-standard:hover:not(:disabled) {
  background: var(--win-control-bg-hover);
  border-color: rgba(255, 255, 255, 0.15);
}

.win-btn-sm {
  height: 26px;
  padding: 0 10px;
  font-size: 11.5px;
}

/* Accent Button (Windows 11 经典亮色主按键) */
.win-btn-accent {
  background: var(--win-accent);
  color: var(--win-accent-text);
  font-weight: 600;
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-bottom: 1px solid rgba(0, 0, 0, 0.4);
  box-shadow: 0 2px 6px rgba(96, 205, 255, 0.25);
}

.win-btn-accent:hover:not(:disabled) {
  background: var(--win-accent-hover);
}

.win-btn-accent:active:not(:disabled) {
  background: var(--win-accent-active);
}

/* Destructive / Danger Button */
.win-btn-danger {
  background: var(--win-danger-bg);
  border: 1px solid var(--win-danger-stroke);
  color: var(--win-danger);
  font-weight: 600;
}

.win-btn-danger:hover:not(:disabled) {
  background: rgba(255, 107, 107, 0.22);
  border-color: var(--win-danger);
}

/* 操作栏容器 */
.win-action-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 14px 10px;
  flex-wrap: wrap;
}

.win-divider {
  width: 1px;
  height: 18px;
  background: rgba(255, 255, 255, 0.1);
  margin: 0 3px;
}

.win-section-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--win-text-secondary);
}

/* WinUI 3 SegmentedControl (分段切换器) */
.win-segmented {
  display: flex;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: var(--win-radius-control);
  padding: 2px;
}

.win-segmented-item {
  background: transparent;
  color: var(--win-text-secondary);
  border: 1px solid transparent;
  border-radius: 4px;
  padding: 4px 10px;
  font-size: 11.5px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.win-segmented-item:hover:not(.active) {
  color: var(--win-text-primary);
  background: rgba(255, 255, 255, 0.04);
}

.win-segmented-item.active {
  background: rgba(255, 255, 255, 0.1);
  color: #fff;
  font-weight: 600;
  border-color: rgba(255, 255, 255, 0.08);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
}

/* WinUI 3 细长进度条与状态显示 */
.win-progress-section {
  padding: 0 14px 10px;
}

.win-progress-track {
  width: 100%;
  height: 4px;
  background: rgba(255, 255, 255, 0.08);
  border-radius: 2px;
  overflow: hidden;
}

.win-progress-fill {
  height: 100%;
  background: var(--win-accent);
  border-radius: 2px;
  transition: width 0.25s ease;
}

.win-progress-info {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 5px;
}

.win-progress-text {
  font-size: 11.5px;
  font-weight: 600;
  color: var(--win-text-secondary);
}

/* WinUI 3 InfoBadge 状态标识 */
.win-status-badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 2px 8px;
  border-radius: var(--win-radius-pill);
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.06);
}

.win-status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
}

.win-status-text {
  font-size: 11.5px;
  font-weight: 600;
}

/* 统计卡片网格 - 深度还原图吧工具箱指标磁贴质感 (Icon盒 + 类别 + 大数值) */
.win-stats-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 8px;
  padding: 0 14px 12px;
}

.win-stat-tile {
  background: rgba(255, 255, 255, 0.035);
  border: 1px solid rgba(255, 255, 255, 0.065);
  border-radius: var(--win-radius-tile);
  padding: 8px 10px;
  display: flex;
  align-items: center;
  gap: 10px;
  transition: all 0.15s ease;
}

.win-stat-tile:hover {
  background: rgba(255, 255, 255, 0.05);
  border-color: rgba(255, 255, 255, 0.1);
  transform: translateY(-1px);
}

.win-stat-icon-box {
  width: 36px;
  height: 36px;
  flex-shrink: 0;
  border-radius: var(--win-radius-control);
  display: flex;
  align-items: center;
  justify-content: center;
}

.win-stat-content {
  display: flex;
  flex-direction: column;
  min-width: 0;
  flex: 1;
}

.win-stat-category {
  font-size: 11px;
  color: var(--win-text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.win-stat-val-row {
  display: flex;
  align-items: baseline;
  gap: 4px;
}

.win-stat-value {
  font-size: 18px;
  font-weight: 600;
  font-family: 'Segoe UI Variable Display', 'Segoe UI', sans-serif;
  line-height: 1.2;
}

.win-stat-unit {
  font-size: 10px;
  color: var(--win-text-secondary);
}

/* 运行日志卡片 */
.win-log-card {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 160px;
}

.win-log-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px 8px;
}

.win-log-actions {
  display: flex;
  gap: 6px;
}

.win-log-terminal {
  flex: 1;
  margin: 0 14px 10px;
  background: var(--win-log-bg);
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: var(--win-radius-control);
  padding: 8px 12px;
  overflow-y: auto;
  font-family: Consolas, 'Cascadia Code', monospace;
  font-size: 12px;
  white-space: pre-wrap;
  word-break: break-all;
}

.win-log-line {
  line-height: 1.6;
}

.win-log-ts {
  color: var(--win-text-tertiary);
  margin-right: 8px;
}

.win-log-success { color: var(--win-success); }
.win-log-skip { color: var(--win-warning); }
.win-log-error { color: var(--win-danger); }
.win-log-warn { color: var(--win-warning); }
.win-log-info { color: #8ab4f8; }
.win-log-normal { color: var(--win-text-primary); }
</style>
