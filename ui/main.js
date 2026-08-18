const { invoke } = window.__TAURI__.tauri;
const { listen } = window.__TAURI__.event;
const { open } = window.__TAURI__.dialog;

const userIdInput = document.getElementById("userId");
const concurrencyInput = document.getElementById("concurrency");
const savePathInput = document.getElementById("savePath");
const authTokenInput = document.getElementById("authToken");
const ct0Input = document.getElementById("ct0");
const btnBrowse = document.getElementById("btnBrowse");
const btnStart = document.getElementById("btnStart");
const btnCancel = document.getElementById("btnCancel");
const btnClearLog = document.getElementById("btnClearLog");
const logArea = document.getElementById("logArea");
const progressBar = document.getElementById("progressBar");
const metricProgress = document.getElementById("metricProgress");
const metricDownloaded = document.getElementById("metricDownloaded");
const metricSkipped = document.getElementById("metricSkipped");
const statusBadge = document.getElementById("statusBadge");

async function init() {
  try {
    const config = await invoke("load_config");
    if (config) {
      userIdInput.value = config.user_id || "";
      savePathInput.value = config.save_path || "";
      authTokenInput.value = config.auth_token || "";
      ct0Input.value = config.ct0 || "";
      concurrencyInput.value = config.concurrency || 8;
    }
  } catch (err) {
    appendLog("warn", `加载配置失败: ${err}`);
  }
}

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

function appendLog(level, msg, timestamp = new Date().toLocaleTimeString()) {
  const line = document.createElement("div");
  line.style.color = level === "warn" ? "#f59e0b" : level === "success" ? "#10b981" : "#d1d5db";
  line.textContent = `[${timestamp}] ${msg}`;
  logArea.appendChild(line);
  logArea.scrollTop = logArea.scrollHeight;
}

btnClearLog.addEventListener("click", () => {
  logArea.innerHTML = "";
});

listen("download-progress", (event) => {
  const p = event.payload;
  progressBar.style.width = `${p.percent}%`;
  metricProgress.textContent = `${p.percent.toFixed(1)}% (${p.current}/${p.total})`;
  metricDownloaded.textContent = p.downloaded;
  metricSkipped.textContent = p.skipped;
});

listen("download-log", (event) => {
  const { level, message, timestamp } = event.payload;
  appendLog(level, message, timestamp);
  if (level === "success" || level === "warn") {
    setRunningState(false);
  }
});

function setRunningState(running) {
  btnStart.disabled = running;
  btnCancel.disabled = !running;
  statusBadge.textContent = running ? "正在下载" : "就绪";
  statusBadge.className = `status-badge ${running ? "running" : "ready"}`;
}

btnStart.addEventListener("click", async () => {
  const config = {
    user_id: userIdInput.value.trim(),
    save_path: savePathInput.value.trim(),
    auth_token: authTokenInput.value.trim(),
    ct0: ct0Input.value.trim(),
    concurrency: parseInt(concurrencyInput.value, 10) || 8,
  };

  if (!config.user_id || !config.save_path || !config.auth_token || !config.ct0) {
    alert("请将用户 ID、存储位置及认证 Cookie 填写完整！");
    return;
  }

  await invoke("save_config", { config });
  setRunningState(true);

  try {
    await invoke("start_download", { config });
  } catch (err) {
    appendLog("warn", `启动失败: ${err}`);
    setRunningState(false);
  }
});

btnCancel.addEventListener("click", async () => {
  await invoke("cancel_download");
});

init();
