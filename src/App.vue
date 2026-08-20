<script setup>
import { ref, reactive, onMounted, onUnmounted, nextTick } from 'vue';
import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/api/dialog';

const config = reactive({
  userId: '',
  savePath: '',
  authToken: '',
  ct0: '',
  recentUserIds: [],
  mediaFilter: 'all',
  concurrency: 8,
  unlimitedTime: true,
  startYear: 1990,
  startMonth: 1,
  startDay: 1,
  endYear: new Date().getFullYear(),
  endMonth: new Date().getMonth() + 1,
  endDay: new Date().getDate()
});

const isRunning = ref(false);
const statusMsg = ref('就绪');
const logs = ref([]);
const progress = reactive({ text: '0 / 0', percent: 0 });
const stats = reactive({ total: 0, downloaded: 0, skipped: 0, failed: 0 });
const logAreaRef = ref(null);

const years = ref(Array.from({ length: new Date().getFullYear() - 1990 + 1 }, (_, i) => 1990 + i));
const months = ref(Array.from({ length: 12 }, (_, i) => i + 1));
const startDays = ref(Array.from({ length: 31 }, (_, i) => i + 1));
const endDays = ref(Array.from({ length: 31 }, (_, i) => i + 1));

const daysInMonth = (y, m) => new Date(y, m, 0).getDate();

const updateStartDays = () => {
  const dim = daysInMonth(config.startYear, config.startMonth);
  startDays.value = Array.from({ length: dim }, (_, i) => i + 1);
  if (config.startDay > dim) config.startDay = dim;
};

const updateEndDays = () => {
  const dim = daysInMonth(config.endYear, config.endMonth);
  endDays.value = Array.from({ length: dim }, (_, i) => i + 1);
  if (config.endDay > dim) config.endDay = dim;
};

const formatTime = (date) => date.toLocaleTimeString("zh-CN", { hour12: false });

const appendLog = (level, msg) => {
  logs.value.push({ level, msg, time: formatTime(new Date()) });
  nextTick(() => {
    if (logAreaRef.value) {
      logAreaRef.value.scrollTop = logAreaRef.value.scrollHeight;
    }
  });
};

const handleBrowse = async () => {
  const selected = await open({
    directory: true,
    multiple: false,
    defaultPath: config.savePath || undefined,
  });
  if (selected) {
    config.savePath = selected;
  }
};

const handleOpenDir = async () => {
  try {
    await invoke("open_download_dir", { config: getBackendConfig() });
  } catch (err) {
    appendLog("warn", `打开目录失败: ${err}`);
  }
};

const clearLog = () => logs.value = [];
const copyLog = async () => {
  try {
    const text = logs.value.map(l => `[${l.time}] ${l.msg}`).join('\n');
    await navigator.clipboard.writeText(text);
    appendLog("info", "[提示] 日志已复制到剪贴板。");
  } catch (err) {
    appendLog("warn", `复制日志失败: ${err}`);
  }
};

const getBackendConfig = () => {
  return {
    auth_token: config.authToken,
    ct0: config.ct0,
    user_id: config.userId,
    save_path: config.savePath,
    concurrency: config.concurrency,
    recent_user_ids: config.recentUserIds,
    media_filter: config.mediaFilter,
    unlimited_time: config.unlimitedTime,
    start_year: config.startYear,
    start_month: config.startMonth,
    start_day: config.startDay,
    end_year: config.endYear,
    end_month: config.endMonth,
    end_day: config.endDay,
  };
};

const startDownload = async () => {
  if (!config.userId || !config.savePath || !config.authToken || !config.ct0) {
    alert("请将账户 ID、保存位置、auth_token 及 ct0 填写完整！");
    return;
  }
  
  const uid = config.userId.trim();
  const recent = [uid, ...config.recentUserIds].filter((v, i, a) => v && a.indexOf(v) === i).slice(0, 7);
  config.recentUserIds = recent;
  
  const backendConfig = getBackendConfig();
  try {
    await invoke("save_config", { config: backendConfig });
  } catch (err) {
    appendLog("warn", `保存配置失败: ${err}`);
  }
  
  isRunning.value = true;
  statusMsg.value = "正在下载…";
  stats.total = 0; stats.downloaded = 0; stats.skipped = 0; stats.failed = 0;
  progress.text = '0 / 0'; progress.percent = 0;
  clearLog();
  
  let filterText = { all: '全部媒体', image: '仅图片', video: '仅视频' }[config.mediaFilter] || '全部';
  appendLog("info", `开始 | @${config.userId} | 类型 ${filterText} | 并发 ${config.concurrency}`);
  
  try {
    await invoke("start_download", { config: backendConfig });
  } catch (err) {
    appendLog("warn", `启动失败: ${err}`);
    isRunning.value = false;
    statusMsg.value = "就绪";
  }
};

const cancelDownload = async () => {
  await invoke("cancel_download");
};

const selectRecent = (e) => {
  if (e.target.value) {
    config.userId = e.target.value;
    e.target.selectedIndex = 0;
  }
};

onMounted(async () => {
  try {
    const loaded = await invoke("load_config");
    if (loaded) {
      if (loaded.auth_token) config.authToken = loaded.auth_token;
      if (loaded.ct0) config.ct0 = loaded.ct0;
      if (loaded.user_id) config.userId = loaded.user_id;
      if (loaded.save_path) config.savePath = loaded.save_path;
      if (loaded.concurrency) config.concurrency = loaded.concurrency;
      if (loaded.recent_user_ids) config.recentUserIds = loaded.recent_user_ids;
      if (loaded.media_filter) config.mediaFilter = loaded.media_filter;
      if (loaded.unlimited_time !== undefined) config.unlimitedTime = loaded.unlimited_time;
      if (loaded.start_year) config.startYear = loaded.start_year;
      if (loaded.start_month) config.startMonth = loaded.start_month;
      if (loaded.start_day) config.startDay = loaded.start_day;
      if (loaded.end_year) config.endYear = loaded.end_year;
      if (loaded.end_month) config.endMonth = loaded.end_month;
      if (loaded.end_day) config.endDay = loaded.end_day;
      
      updateStartDays();
      updateEndDays();
    }
  } catch (err) {
    appendLog("warn", `加载配置失败: ${err}`);
  }
  
  appendLog("info", "欢迎使用 (Vue 3 重构版)。");
  appendLog("info", "基于媒体唯一 ID 去重，重复运行不会重复下载。");
  
  listen("download-progress", (event) => {
    const p = event.payload;
    stats.total = p.total;
    stats.downloaded = p.downloaded;
    stats.skipped = p.skipped;
    stats.failed = p.failed;
    progress.text = `${p.downloaded + p.skipped} / ${p.total}`;
    progress.percent = p.total > 0 ? ((p.downloaded + p.skipped) / p.total) * 100 : 0;
  });
  
  listen("download-log", (event) => {
    const p = event.payload;
    logs.value.push({ level: p.level, msg: p.message, time: p.timestamp });
    nextTick(() => {
      if (logAreaRef.value) logAreaRef.value.scrollTop = logAreaRef.value.scrollHeight;
    });
  });
  
  listen("download-finish", (event) => {
    const p = event.payload;
    isRunning.value = false;
    statusMsg.value = p.success ? "成功完成" : "已取消/异常";
  });
});
</script>

<template>
  <div class="app-container">
    <section class="card config-card">
      <div class="card-title">账户设置</div>
      <div class="form-body">
        <div class="form-row">
          <label>账户 ID:</label>
          <div class="input-wrap">
            <input type="text" v-model="config.userId" placeholder="例如: elonmusk" autocomplete="off" />
            <select class="history-select" @change="selectRecent">
              <option value="" disabled selected>历史记录 ▼</option>
              <option v-for="id in config.recentUserIds" :key="id" :value="id">{{ id }}</option>
            </select>
          </div>
        </div>

        <div class="form-row">
          <label>保存位置:</label>
          <div class="input-with-button">
            <input type="text" v-model="config.savePath" placeholder="选择媒体文件保存目录..." />
            <button @click="handleBrowse" class="btn btn-ghost">选择...</button>
          </div>
        </div>

        <div class="form-row">
          <label>auth_token:</label>
          <div class="input-wrap">
            <input type="text" v-model="config.authToken" placeholder="填入浏览器 Cookie 中的 auth_token" autocomplete="off" spellcheck="false" />
          </div>
        </div>

        <div class="form-row">
          <label>ct0:</label>
          <div class="input-wrap">
            <input type="text" v-model="config.ct0" placeholder="填入浏览器 Cookie 中的 ct0" autocomplete="off" spellcheck="false" />
          </div>
        </div>

        <div class="form-row">
          <label>时间范围:</label>
          <div class="date-range" :style="{ opacity: config.unlimitedTime ? 0.45 : 1, pointerEvents: config.unlimitedTime ? 'none' : 'auto' }">
            <span class="date-label">起始</span>
            <select class="date-select year-select" v-model.number="config.startYear" @change="updateStartDays">
              <option v-for="y in years" :key="y" :value="y">{{ y }}</option>
            </select>
            <select class="date-select month-select" v-model.number="config.startMonth" @change="updateStartDays">
              <option v-for="m in months" :key="m" :value="m">{{ m }}</option>
            </select>
            <select class="date-select day-select" v-model.number="config.startDay">
              <option v-for="d in startDays" :key="d" :value="d">{{ d }}</option>
            </select>
            <span class="date-label">至</span>
            <select class="date-select year-select" v-model.number="config.endYear" @change="updateEndDays">
              <option v-for="y in years" :key="y" :value="y">{{ y }}</option>
            </select>
            <select class="date-select month-select" v-model.number="config.endMonth" @change="updateEndDays">
              <option v-for="m in months" :key="m" :value="m">{{ m }}</option>
            </select>
            <select class="date-select day-select" v-model.number="config.endDay">
              <option v-for="d in endDays" :key="d" :value="d">{{ d }}</option>
            </select>
          </div>
          <label class="checkbox-inline" style="margin-left:auto;">
            <input type="checkbox" v-model="config.unlimitedTime" />
            <span>不限时间</span>
          </label>
        </div>
      </div>
    </section>

    <section class="card action-card">
      <div class="action-row">
        <button @click="startDownload" class="btn btn-primary" :disabled="isRunning">开始下载</button>
        <button @click="cancelDownload" class="btn btn-danger" :disabled="!isRunning">取消下载</button>

        <span class="inline-label">类型</span>
        <div class="segmented">
          <button :class="{ active: config.mediaFilter === 'all' }" @click="config.mediaFilter = 'all'">全部媒体</button>
          <button :class="{ active: config.mediaFilter === 'image' }" @click="config.mediaFilter = 'image'">仅图片</button>
          <button :class="{ active: config.mediaFilter === 'video' }" @click="config.mediaFilter = 'video'">仅视频</button>
        </div>

        <span class="inline-label">线程数</span>
        <select v-model.number="config.concurrency" class="concurrency-select">
          <option v-for="c in [1,2,4,6,8,12,16,24,32]" :key="c" :value="c">{{ c }}</option>
        </select>

        <button @click="handleOpenDir" class="btn btn-ghost">打开目录</button>
        <span class="status-text" :class="{ running: isRunning, success: statusMsg === '成功完成' }">状态: {{ statusMsg }}</span>
      </div>

      <div class="progress-row">
        <div class="progress-bar-container">
          <div class="progress-bar" :style="{ width: progress.percent + '%' }"></div>
        </div>
        <div class="progress-text">{{ progress.text }}</div>
      </div>

      <div class="stats-grid">
        <div class="stat-card">
          <div class="stat-strip" style="background: var(--accent);"></div>
          <div class="stat-value-wrap">
            <span class="stat-value" style="color: var(--accent);">{{ stats.total }}</span>
            <span class="stat-unit">项</span>
          </div>
          <div class="stat-label">总计找到</div>
        </div>
        <div class="stat-card">
          <div class="stat-strip" style="background: var(--success);"></div>
          <div class="stat-value-wrap">
            <span class="stat-value" style="color: var(--success);">{{ stats.downloaded }}</span>
            <span class="stat-unit">项</span>
          </div>
          <div class="stat-label">本次下载</div>
        </div>
        <div class="stat-card">
          <div class="stat-strip" style="background: var(--warn);"></div>
          <div class="stat-value-wrap">
            <span class="stat-value" style="color: var(--warn);">{{ stats.skipped }}</span>
            <span class="stat-unit">项</span>
          </div>
          <div class="stat-label">已跳过(重复)</div>
        </div>
        <div class="stat-card">
          <div class="stat-strip" style="background: var(--danger);"></div>
          <div class="stat-value-wrap">
            <span class="stat-value" style="color: var(--danger);">{{ stats.failed }}</span>
            <span class="stat-unit">项</span>
          </div>
          <div class="stat-label">下载失败</div>
        </div>
      </div>
    </section>

    <section class="card log-card">
      <div class="log-header">
        <span class="card-title" style="margin-bottom:0;">运行日志</span>
        <div class="log-actions">
          <button @click="copyLog" class="btn btn-ghost btn-sm">复制日志</button>
          <button @click="clearLog" class="btn btn-ghost btn-sm">清空</button>
        </div>
      </div>
      <div class="log-area" ref="logAreaRef">
        <div v-for="(log, idx) in logs" :key="idx" class="log-line" :class="log.level">
          [{{ log.time }}] {{ log.msg }}
        </div>
      </div>
    </section>
  </div>
</template>