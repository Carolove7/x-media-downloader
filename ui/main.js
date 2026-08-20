const { invoke } = window.__TAURI__.tauri;
const { listen } = window.__TAURI__.event;
const { open } = window.__TAURI__.dialog;

// DOM 元素
const userIdInput = document.getElementById("userId");
const recentUserIdsList = document.getElementById("recentUserIds");
const savePathInput = document.getElementById("savePath");
const authTokenInput = document.getElementById("authToken");
const ct0Input = document.getElementById("ct0");
const startYear = document.getElementById("startYear");
const startMonth = document.getElementById("startMonth");
const startDay = document.getElementById("startDay");
const endYear = document.getElementById("endYear");
const endMonth = document.getElementById("endMonth");
const endDay = document.getElementById("endDay");
const chkUnlimitedTime = document.getElementById("chkUnlimitedTime");
const mediaFilterBtns = document.querySelectorAll("#mediaFilter button");
const concurrencyInput = document.getElementById("concurrency");
const btnBrowse = document.getElementById("btnBrowse");
const btnStart = document.getElementById("btnStart");
const btnCancel = document.getElementById("btnCancel");
const btnOpenDir = document.getElementById("btnOpenDir");
const btnClearLog = document.getElementById("btnClearLog");
const btnCopyLog = document.getElementById("btnCopyLog");
const logArea = document.getElementById("logArea");
const progressBar = document.getElementById("progressBar");
const progressText = document.getElementById("progressText");
const statDownloaded = document.getElementById("statDownloaded");
const statSkipped = document.getElementById("statSkipped");
const statFailed = document.getElementById("statFailed");
const statSpeed = document.getElementById("statSpeed");
const statSpeedUnit = document.getElementById("statSpeedUnit");
const statusBadge = document.getElementById("statusBadge");

let currentConfig = null;
let mediaFilter = "all";

// 初始化
async function init() {
  initDatePickers();
  initMediaFilter();

  try {
    currentConfig = await invoke("load_config");
    if (currentConfig) {
      userIdInput.value = currentConfig.user_id || "";
      savePathInput.value = currentConfig.save_path || "";
      authTokenInput.value = currentConfig.auth_token || "";
      ct0Input.value = currentConfig.ct0 || "";
      concurrencyInput.value = String(currentConfig.concurrency || 8);

      // 最近账户
      populateRecentUserIds(currentConfig.recent_user_ids || []);

      // 时间范围
      chkUnlimitedTime.checked = currentConfig.unlimited_time !== false;
      if (currentConfig.start_year) startYear.value = String(currentConfig.start_year);
      if (currentConfig.start_month) startMonth.value = String(currentConfig.start_month);
      if (currentConfig.start_day) startDay.value = String(currentConfig.start_day);
      if (currentConfig.end_year) endYear.value = String(currentConfig.end_year);
      if (currentConfig.end_month) endMonth.value = String(currentConfig.end_month);
      if (currentConfig.end_day) endDay.value = String(currentConfig.end_day);
      refreshDays(startYear, startMonth, startDay);
      refreshDays(endYear, endMonth, endDay);
      toggleDateInputs();

      // 媒体类型
      mediaFilter = currentConfig.media_filter || "all";
      updateMediaFilterUI();
    }
  } catch (err) {
    appendLog("warn", `加载配置失败: ${err}`);
  }

  appendLog("info", "欢迎使用。");
  appendLog("info", "基于媒体唯一 ID 去重，重复运行不会重复下载；回车开始 / Esc 取消。");
}

function populateRecentUserIds(list) {
  recentUserIdsList.innerHTML = '<option value="" disabled selected>历史记录 ▼</option>';
  const seen = new Set();
  for (const id of list) {
    const v = (id || "").trim();
    if (v && !seen.has(v)) {
      seen.add(v);
      const opt = document.createElement("option");
      opt.value = v;
      opt.textContent = v;
      recentUserIdsList.appendChild(opt);
    }
  }
}

recentUserIdsList.addEventListener('change', (e) => {
  if (e.target.value) {
    userIdInput.value = e.target.value;
    e.target.selectedIndex = 0;
  }
});

function initDatePickers() {
  const today = new Date();
  const years = Array.from({ length: today.getFullYear() - 1990 + 1 }, (_, i) => 1990 + i);
  const months = Array.from({ length: 12 }, (_, i) => i + 1);
  const days = Array.from({ length: 31 }, (_, i) => i + 1);

  fillSelect(startYear, years, 1990);
  fillSelect(endYear, years, today.getFullYear());
  fillSelect(startMonth, months, 1);
  fillSelect(endMonth, months, today.getMonth() + 1);
  fillSelect(startDay, days, 1);
  fillSelect(endDay, days, today.getDate());

  [startYear, startMonth].forEach((el) =>
    el.addEventListener("change", () => refreshDays(startYear, startMonth, startDay))
  );
  [endYear, endMonth].forEach((el) =>
    el.addEventListener("change", () => refreshDays(endYear, endMonth, endDay))
  );
}

function fillSelect(select, values, defaultValue) {
  select.innerHTML = "";
  for (const v of values) {
    const opt = document.createElement("option");
    opt.value = String(v);
    opt.textContent = String(v);
    select.appendChild(opt);
  }
  select.value = String(defaultValue);
}

function refreshDays(yearEl, monthEl, dayEl) {
  const y = parseInt(yearEl.value, 10);
  const m = parseInt(monthEl.value, 10);
  const dim = daysInMonth(y, m);
  const current = parseInt(dayEl.value, 10);
  dayEl.innerHTML = "";
  for (let d = 1; d <= dim; d++) {
    const opt = document.createElement("option");
    opt.value = String(d);
    opt.textContent = String(d);
    dayEl.appendChild(opt);
  }
  dayEl.value = String(Math.min(current, dim));
}

function daysInMonth(year, month) {
  return new Date(year, month, 0).getDate();
}

function initMediaFilter() {
  mediaFilterBtns.forEach((btn) => {
    btn.addEventListener("click", () => {
      mediaFilter = btn.dataset.value;
      updateMediaFilterUI();
    });
  });
}

function updateMediaFilterUI() {
  mediaFilterBtns.forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.value === mediaFilter);
  });
}

chkUnlimitedTime.addEventListener("change", toggleDateInputs);

function toggleDateInputs() {
  const disabled = chkUnlimitedTime.checked;
  [startYear, startMonth, startDay, endYear, endMonth, endDay].forEach((el) => {
    el.disabled = disabled;
    el.style.opacity = disabled ? "0.45" : "1";
  });
}

// 浏览目录
btnBrowse.addEventListener("click", async () => {
  const selected = await open({
    directory: true,
    multiple: false,
    defaultPath: savePathInput.value || undefined,
  });
  if (selected) {
    savePathInput.value = selected;
  }
});

// 打开目录
btnOpenDir.addEventListener("click", async () => {
  try {
    await invoke("open_download_dir", { config: buildConfig(false) });
  } catch (err) {
    appendLog("warn", `打开目录失败: ${err}`);
  }
});

// 清空/复制日志
btnClearLog.addEventListener("click", () => {
  logArea.innerHTML = "";
});

btnCopyLog.addEventListener("click", async () => {
  try {
    await navigator.clipboard.writeText(logArea.innerText);
    appendLog("info", "[提示] 日志已复制到剪贴板。");
  } catch (err) {
    appendLog("warn", `复制日志失败: ${err}`);
  }
});

// 日志追加
function appendLog(level, msg, timestamp = formatTime(new Date())) {
  const line = document.createElement("div");
  line.className = `log-line ${level}`;
  line.textContent = `[${timestamp}] ${msg}`;
  logArea.appendChild(line);
  logArea.scrollTop = logArea.scrollHeight;
}

function formatTime(date) {
  return date.toLocaleTimeString("zh-CN", { hour12: false });
}

// 构建配置对象
function buildConfig(includeRecent = true) {
  const cfg = {
    auth_token: authTokenInput.value.trim(),
    ct0: ct0Input.value.trim(),
    user_id: userIdInput.value.trim(),
    save_path: savePathInput.value.trim(),
    concurrency: parseInt(concurrencyInput.value, 10) || 8,
    media_filter: mediaFilter,
    unlimited_time: chkUnlimitedTime.checked,
    start_year: parseInt(startYear.value, 10),
    start_month: parseInt(startMonth.value, 10),
    start_day: parseInt(startDay.value, 10),
    end_year: parseInt(endYear.value, 10),
    end_month: parseInt(endMonth.value, 10),
    end_day: parseInt(endDay.value, 10),
  };
  if (includeRecent && currentConfig) {
    cfg.recent_user_ids = currentConfig.recent_user_ids || [];
  }
  return cfg;
}

// 开始下载
btnStart.addEventListener("click", async () => {
  const config = buildConfig(true);

  if (!config.user_id || !config.save_path || !config.auth_token || !config.ct0) {
    alert("请将账户 ID、保存位置、auth_token 及 ct0 填写完整！");
    return;
  }

  // 本地同步更新最近账户，保证下拉立即刷新
  const uid = config.user_id;
  config.recent_user_ids = [uid, ...(config.recent_user_ids || [])]
    .filter((v, i, a) => v && a.indexOf(v) === i)
    .slice(0, 7);

  try {
    await invoke("save_config", { config });
    currentConfig = config;
    populateRecentUserIds(config.recent_user_ids);
  } catch (err) {
    appendLog("warn", `保存配置失败: ${err}`);
  }

  setRunningState(true);
  clearStats();
  logArea.innerHTML = "";
  appendLog("info", `开始 | @${config.user_id} | 类型 ${mediaFilterText()} | 并发 ${config.concurrency}`);

  try {
    await invoke("start_download", { config });
  } catch (err) {
    appendLog("warn", `启动失败: ${err}`);
    setRunningState(false);
  }
});

function mediaFilterText() {
  return { all: "全部媒体", image: "仅图片", video: "仅视频" }[mediaFilter] || "全部媒体";
}

// 取消下载
btnCancel.addEventListener("click", async () => {
  await invoke("cancel_download");
});

// 键盘快捷键
document.addEventListener("keydown", (e) => {
  if (e.key === "Enter" && !btnStart.disabled) {
    btnStart.click();
  } else if (e.key === "Escape" && !btnCancel.disabled) {
    btnCancel.click();
  }
});

// 状态控制
function setRunningState(running) {
  btnStart.disabled = running;
  btnCancel.disabled = !running;
  if (running) {
    statusBadge.textContent = "状态: 正在下载…";
    statusBadge.className = "status-text running";
  }
}

function setFinishedState(cancelled) {
  btnStart.disabled = false;
  btnCancel.disabled = true;
  statusBadge.className = cancelled ? "status-text cancelled" : "status-text success";
  statusBadge.textContent = cancelled ? "状态: 已取消" : "状态: 已完成";
}

function setErrorState() {
  btnStart.disabled = false;
  btnCancel.disabled = true;
  statusBadge.className = "status-text cancelled";
  statusBadge.textContent = "状态: 失败";
}

function clearStats() {
  statDownloaded.textContent = "0";
  statSkipped.textContent = "0";
  statFailed.textContent = "0";
  statSpeed.textContent = "0";
  statSpeedUnit.textContent = "B/s";
  progressBar.style.width = "0%";
  progressText.textContent = "0%";
}

// 事件监听
listen("download-progress", (event) => {
  const p = event.payload;
  progressBar.style.width = `${p.percent}%`;
  progressText.textContent = `${p.percent.toFixed(1)}%  (${p.current}/${p.total})`;
  statDownloaded.textContent = p.downloaded;
  statSkipped.textContent = p.skipped;
  statFailed.textContent = p.failed;

  const { value, unit } = formatSpeedParts(p.speed);
  statSpeed.textContent = value;
  statSpeedUnit.textContent = unit;
});

listen("download-log", (event) => {
  const { level, message, timestamp } = event.payload;
  appendLog(level, message, timestamp);
  // 只有任务级日志（以 >>> 开头）才改变运行状态；单个文件的
  // [成功]/[失败]/[跳过] 日志不改变按钮状态——否则下载中途成功
  // 第一个文件就会提前恢复“开始”按钮并禁用“取消”，造成取消无反应。
  if (!message.startsWith(">>>")) return;
  if (level === "success") {
    setFinishedState(false);
  } else if (level === "warn") {
    setFinishedState(message.includes("取消"));
  } else if (level === "error") {
    setErrorState();
  }
});

// 速度格式化
function formatSpeed(bps) {
  let speed = Math.max(0, bps);
  const units = ["B/s", "KB/s", "MB/s", "GB/s"];
  let idx = 0;
  while (speed >= 1024 && idx < units.length - 1) {
    speed /= 1024;
    idx++;
  }
  if (idx === 0) return `${Math.round(speed)} ${units[idx]}`;
  return `${speed.toFixed(1)} ${units[idx]}`;
}

function formatSpeedParts(bps) {
  const text = formatSpeed(bps);
  const [value, ...rest] = text.split(" ");
  return { value, unit: rest.join(" ") };
}

init();
