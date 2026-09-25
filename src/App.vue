<template>
  <div class="window-shell" :class="{ 'is-maximized': isMaximized }">
    <!-- Fluent 2 自定义无边框窗口标题栏 -->
    <header class="fluent-titlebar" data-tauri-drag-region @dblclick="toggleMaximize">
      <div class="fluent-titlebar-left" data-tauri-drag-region>
        <div class="fluent-titlebar-app-icon" data-tauri-drag-region>
          <svg viewBox="0 0 24 24" width="15" height="15" fill="currentColor">
            <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z"/>
          </svg>
        </div>
        <span class="fluent-titlebar-badge" data-tauri-drag-region>v1.1.0-pre</span>
      </div>

      <div class="fluent-titlebar-controls">
        <button
          type="button"
          class="fluent-caption-btn"
          title="最小化"
          aria-label="最小化"
          @click="minimizeWindow"
        >
          <svg width="10" height="10" viewBox="0 0 10 10">
            <path d="M0 5h10" stroke="currentColor" stroke-width="1"/>
          </svg>
        </button>
        <button
          type="button"
          class="fluent-caption-btn"
          :title="isMaximized ? '向下还原' : '最大化'"
          :aria-label="isMaximized ? '向下还原' : '最大化'"
          @click="toggleMaximize"
        >
          <svg v-if="isMaximized" width="10" height="10" viewBox="0 0 10 10">
            <path d="M2.5 0.5h7v7h-7z" fill="none" stroke="currentColor" stroke-width="1"/>
            <path d="M0.5 2.5v7h7v-7h-7z" fill="none" stroke="currentColor" stroke-width="1"/>
          </svg>
          <svg v-else width="10" height="10" viewBox="0 0 10 10">
            <rect x="0.5" y="0.5" width="9" height="9" fill="none" stroke="currentColor" stroke-width="1"/>
          </svg>
        </button>
        <button
          type="button"
          class="fluent-caption-btn fluent-close-btn"
          title="关闭"
          aria-label="关闭"
          @click="closeWindow"
        >
          <svg width="10" height="10" viewBox="0 0 10 10">
            <path d="M1 1l8 8m0-8L1 9" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"/>
          </svg>
        </button>
      </div>
    </header>

    <div class="app-root">
      <!-- 卡片 1：账户设置 (Fluent 2 SettingsCard 风格) -->
      <section class="fluent-card">
      <div class="fluent-card-header">
        <div class="fluent-header-content">
          <div class="fluent-title-wrap">
            <svg class="fluent-icon-primary" viewBox="0 0 20 20" width="16" height="16" fill="currentColor">
              <path d="M10 2a4 4 0 100 8 4 4 0 000-8zm-2 4a2 2 0 114 0 2 2 0 01-4 0zm-4 9a4 4 0 014-4h4a4 4 0 014 4v1H4v-1zm2 0a2 2 0 012-2h4a2 2 0 012 2v0H6v0z"/>
            </svg>
            <span class="fluent-card-title">账户设置</span>
          </div>
          <span class="fluent-card-subtitle">配置用于鉴权拉取媒体的凭据与目标规则</span>
        </div>
      </div>

      <div class="fluent-card-body">
        <!-- 账户 ID -->
        <div class="fluent-row">
          <div class="fluent-row-label">
            <svg class="fluent-row-icon" viewBox="0 0 16 16" width="14" height="14" fill="currentColor">
              <path d="M8 8a3 3 0 100-6 3 3 0 000 6zm2-3a2 2 0 11-4 0 2 2 0 014 0zm4 8c0 1-1 1-1 1H3s-1 0-1-1 1-4 6-4 6 3 6 4zm-1-.004c-.001-.246-.154-.986-.832-1.664C11.516 10.68 10.289 10 8 10c-2.29 0-3.516.68-4.168 1.332-.678.678-.83 1.418-.832 1.664h10z"/>
            </svg>
            <span>账户 ID</span>
          </div>
          <div class="fluent-control-container">
            <div class="fluent-combo" ref="comboRef">
              <input
                v-model.trim="user_id"
                class="fluent-textbox"
                type="text"
                placeholder="不含 @ 的账户 handle"
                @keydown.down.prevent="moveActive(1)"
                @keydown.up.prevent="moveActive(-1)"
                @keydown.enter.prevent="applyActive"
                @keydown.esc="closeCombo"
              />
              <button
                type="button"
                class="fluent-combo-button"
                :class="{ open: comboOpen }"
                tabindex="-1"
                aria-label="展开历史账户"
                @click="toggleCombo"
              >
                <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
                  <path d="M2.5 4.5L6 8L9.5 4.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
              </button>

              <!-- Fluent 2 MenuFlyout 下拉层 -->
              <div v-if="comboOpen" class="fluent-flyout">
                <div
                  v-for="(id, i) in recent_ids"
                  :key="id"
                  class="fluent-flyout-item"
                  :class="{ active: i === activeIndex }"
                  @mouseenter="activeIndex = i"
                  @mousedown.prevent="selectId(id)"
                >
                  <span class="fluent-flyout-pill"></span>
                  <span class="fluent-flyout-text">{{ id }}</span>
                </div>
                <div v-if="!recent_ids.length" class="fluent-flyout-empty">暂无历史记录</div>
              </div>
            </div>
          </div>
        </div>

        <!-- 保存位置 -->
        <div class="fluent-row zebra">
          <div class="fluent-row-label">
            <svg class="fluent-row-icon" viewBox="0 0 16 16" width="14" height="14" fill="currentColor">
              <path d="M1.5 2.5A1.5 1.5 0 013 1h3.086a1.5 1.5 0 011.06.44l1.414 1.414A1.5 1.5 0 009.62 3.25H13A1.5 1.5 0 0114.5 4.75v8.5A1.5 1.5 0 0113 14.75H3A1.5 1.5 0 011.5 13.25v-10.75zm1.5-.5a.5.5 0 00-.5.5v10.75a.5.5 0 00.5.5h10a.5.5 0 00.5-.5v-8.5a.5.5 0 00-.5-.5H9.621a2.5 2.5 0 01-1.768-.732L6.44 2.104A.5.5 0 006.086 2H3z"/>
            </svg>
            <span>保存位置</span>
          </div>
          <div class="fluent-control-container fluent-path-group">
            <input v-model.trim="save_path" class="fluent-textbox" type="text" placeholder="选择媒体文件保存目录..." />
            <button class="fluent-btn fluent-btn-standard" @click="browseFolder">
              <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
                <path d="M1.5 2.5A1.5 1.5 0 013 1h3.086a1.5 1.5 0 011.06.44l1.414 1.414A1.5 1.5 0 009.62 3.25H13A1.5 1.5 0 0114.5 4.75v8.5A1.5 1.5 0 0113 14.75H3A1.5 1.5 0 011.5 13.25v-10.75zm1.5-.5a.5.5 0 00-.5.5v10.75a.5.5 0 00.5.5h10a.5.5 0 00.5-.5v-8.5a.5.5 0 00-.5-.5H9.621a2.5 2.5 0 01-1.768-.732L6.44 2.104A.5.5 0 006.086 2H3z"/>
              </svg>
              <span>选择…</span>
            </button>
          </div>
        </div>

        <!-- auth_token -->
        <div class="fluent-row">
          <div class="fluent-row-label">
            <svg class="fluent-row-icon" viewBox="0 0 16 16" width="14" height="14" fill="currentColor">
              <path d="M0 8a4 4 0 017.465-2H14a.5.5 0 01.354.146l1.5 1.5a.5.5 0 010 .708l-1.5 1.5a.5.5 0 01-.708 0L13 9.207l-.646.647a.5.5 0 01-.708 0L11 9.207l-.646.647a.5.5 0 01-.708 0L9 9.207l-.646.647a.5.5 0 01-.708 0L7.465 10A4 4 0 010 8zm4-3a3 3 0 100 6 3 3 0 000-6z"/>
            </svg>
            <span>auth_token</span>
          </div>
          <div class="fluent-control-container">
            <input v-model.trim="auth_token" class="fluent-textbox" type="text" placeholder="Cookie 中的 auth_token" />
          </div>
        </div>

        <!-- ct0 -->
        <div class="fluent-row zebra">
          <div class="fluent-row-label">
            <svg class="fluent-row-icon" viewBox="0 0 16 16" width="14" height="14" fill="currentColor">
              <path d="M5.072.56C6.157.265 7.31 0 8 0s1.843.265 2.928.56c1.11.3 2.229.655 2.887.87a1.54 1.54 0 011.044 1.262c.596 4.477-.787 7.795-2.465 9.99a11.775 11.775 0 01-2.517 2.453 7.159 7.159 0 01-1.048.625c-.28.132-.581.24-.829.24s-.548-.108-.829-.24a7.158 7.158 0 01-1.048-.625 11.777 11.777 0 01-2.517-2.453C1.928 10.487.545 7.169 1.141 2.692A1.54 1.54 0 012.185 1.43 62.456 62.456 0 015.072.56z"/>
            </svg>
            <span>ct0</span>
          </div>
          <div class="fluent-control-container">
            <input v-model.trim="ct0" class="fluent-textbox" type="text" placeholder="Cookie 中的 ct0" />
          </div>
        </div>

        <!-- 时间范围 -->
        <div class="fluent-row">
          <div class="fluent-row-label">
            <svg class="fluent-row-icon" viewBox="0 0 16 16" width="14" height="14" fill="currentColor">
              <path d="M3.5 0a.5.5 0 01.5.5V1h8V.5a.5.5 0 011 0V1h1a2 2 0 012 2v11a2 2 0 01-2 2H2a2 2 0 01-2-2V3a2 2 0 012-2h1V.5a.5.5 0 01.5-.5zM1 4v10a1 1 0 001 1h12a1 1 0 001-1V4H1z"/>
            </svg>
            <span>时间范围</span>
          </div>
          <div class="fluent-control-container fluent-date-container">
            <div class="fluent-date-group">
              <span class="fluent-date-tag">起始</span>
              <select v-model="sy" class="fluent-select" :disabled="noLimit" @change="refreshDays('s')">
                <option v-for="y in years" :key="y" :value="y">{{ y }}</option>
              </select>
              <select v-model="sm" class="fluent-select" :disabled="noLimit" @change="refreshDays('s')">
                <option v-for="m in months" :key="m" :value="m">{{ m }}</option>
              </select>
              <select v-model="sd" class="fluent-select" :disabled="noLimit">
                <option v-for="d in startDays" :key="d" :value="d">{{ d }}</option>
              </select>
            </div>

            <span class="fluent-date-separator">至</span>

            <div class="fluent-date-group">
              <span class="fluent-date-tag">结束</span>
              <select v-model="ey" class="fluent-select" :disabled="noLimit" @change="refreshDays('e')">
                <option v-for="y in years" :key="y" :value="y">{{ y }}</option>
              </select>
              <select v-model="em" class="fluent-select" :disabled="noLimit" @change="refreshDays('e')">
                <option v-for="m in months" :key="m" :value="m">{{ m }}</option>
              </select>
              <select v-model="ed" class="fluent-select" :disabled="noLimit">
                <option v-for="d in endDays" :key="d" :value="d">{{ d }}</option>
              </select>
            </div>

            <!-- Fluent 2 Checkbox -->
            <label class="fluent-checkbox">
              <input type="checkbox" v-model="noLimit" />
              <span class="fluent-checkbox-box">
                <svg v-if="noLimit" width="10" height="8" viewBox="0 0 10 8" fill="none">
                  <path d="M1 3.5L3.5 6L9 1" stroke="#000000" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
              </span>
              <span class="fluent-checkbox-label">不限时间</span>
            </label>
          </div>
        </div>
      </div>
    </section>

    <!-- 卡片 2：操作与指标监控 (Fluent 2 规范) -->
    <section class="fluent-card">
      <div class="fluent-action-bar">
        <!-- 开始下载 (Fluent Accent Button) -->
        <button class="fluent-btn fluent-btn-accent" :disabled="running" @click="startDownload">
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
            <path d="M8 12l4-4h-2.5V2h-3v6H4l4 4z"/>
            <path d="M2 13.5v1h12v-1H2z"/>
          </svg>
          <span>开始下载</span>
        </button>

        <!-- 取消下载 (Fluent Destructive Button) -->
        <button class="fluent-btn fluent-btn-danger" :disabled="!running || cancelling" @click="cancelDownload">
          <svg width="13" height="13" viewBox="0 0 16 16" fill="currentColor">
            <rect x="3" y="3" width="10" height="10" rx="1.5"/>
          </svg>
          <span>取消下载</span>
        </button>

        <span class="fluent-divider"></span>

        <!-- 媒体类型分段切换器 (Fluent SegmentedControl) -->
        <span class="fluent-inline-label">类型</span>
        <div class="fluent-segmented">
          <button
            v-for="opt in ['全部媒体', '仅图片', '仅视频']"
            :key="opt"
            class="fluent-segmented-item"
            :class="{ active: media_filter_label === opt }"
            @click="media_filter_label = opt"
          >
            {{ opt }}
          </button>
        </div>

        <span class="fluent-divider"></span>

        <!-- 并发线程数选择 -->
        <span class="fluent-inline-label">线程数</span>
        <select v-model.number="concurrency" class="fluent-select fluent-concurrency-select">
          <option v-for="n in 32" :key="n" :value="n">{{ n }}</option>
        </select>

        <!-- 打开下载目录 -->
        <button class="fluent-btn fluent-btn-standard" @click="openFolder">
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
            <path d="M1.5 2.5A1.5 1.5 0 013 1h3.086a1.5 1.5 0 011.06.44l1.414 1.414A1.5 1.5 0 009.62 3.25H13A1.5 1.5 0 0114.5 4.75v8.5A1.5 1.5 0 0113 14.75H3A1.5 1.5 0 011.5 13.25v-10.75zm1.5-.5a.5.5 0 00-.5.5v10.75a.5.5 0 00.5.5h10a.5.5 0 00.5-.5v-8.5a.5.5 0 00-.5-.5H9.621a2.5 2.5 0 01-1.768-.732L6.44 2.104A.5.5 0 006.086 2H3z"/>
          </svg>
          <span>打开目录</span>
        </button>
      </div>

      <!-- Fluent 2 细条平滑进度条与状态条 -->
      <div class="fluent-progress-section">
        <div class="fluent-progress-track">
          <div class="fluent-progress-fill" :style="{ width: progressPercent + '%' }"></div>
        </div>
        <div class="fluent-progress-info">
          <span class="fluent-progress-text">{{ progressText }}</span>
          <!-- Fluent InfoBadge 状态指示 -->
          <div class="fluent-status-badge">
            <span class="fluent-status-dot" :style="{ backgroundColor: statusColor }"></span>
            <span class="fluent-status-text" :style="{ color: statusColor }">{{ statusText }}</span>
          </div>
        </div>
      </div>

      <!-- 4 组核心统计磁贴 (Fluent InfoCards 深度质感) -->
      <div class="fluent-stats-grid">
        <!-- 已下载 -->
        <div class="fluent-stat-tile">
          <div class="fluent-stat-icon-box" style="background: var(--fluent-success-subtle); color: var(--fluent-success);">
            <svg width="18" height="18" viewBox="0 0 16 16" fill="currentColor">
              <path d="M8 12l4-4h-2.5V2h-3v6H4l4 4z"/>
              <path d="M2 13.5v1h12v-1H2z"/>
            </svg>
          </div>
          <div class="fluent-stat-content">
            <span class="fluent-stat-category">已下载媒体</span>
            <div class="fluent-stat-val-row">
              <span class="fluent-stat-value" style="color: var(--fluent-success);">{{ stat.down }}</span>
              <span class="fluent-stat-unit">项</span>
            </div>
          </div>
        </div>

        <!-- 已跳过 -->
        <div class="fluent-stat-tile">
          <div class="fluent-stat-icon-box" style="background: var(--fluent-warning-subtle); color: var(--fluent-warning);">
            <svg width="18" height="18" viewBox="0 0 16 16" fill="currentColor">
              <path d="M4.5 3a.5.5 0 00-.5.5v9a.5.5 0 00.757.429l6-4.5a.5.5 0 000-.858l-6-4.5A.5.5 0 004.5 3zm7 0a.5.5 0 00-.5.5v9a.5.5 0 001 0v-9a.5.5 0 00-.5-.5z"/>
            </svg>
          </div>
          <div class="fluent-stat-content">
            <span class="fluent-stat-category">已跳过 (重复)</span>
            <div class="fluent-stat-val-row">
              <span class="fluent-stat-value" style="color: var(--fluent-warning);">{{ stat.skip }}</span>
              <span class="fluent-stat-unit">项</span>
            </div>
          </div>
        </div>

        <!-- 失败 -->
        <div class="fluent-stat-tile">
          <div class="fluent-stat-icon-box" style="background: var(--fluent-danger-bg); color: var(--fluent-danger);">
            <svg width="18" height="18" viewBox="0 0 16 16" fill="currentColor">
              <path d="M8 1a7 7 0 100 14A7 7 0 008 1zm0 10.5a.75.75 0 110-1.5.75.75 0 010 1.5zm.75-7.75v5h-1.5v-5h1.5z"/>
            </svg>
          </div>
          <div class="fluent-stat-content">
            <span class="fluent-stat-category">下载失败</span>
            <div class="fluent-stat-val-row">
              <span class="fluent-stat-value" style="color: var(--fluent-danger);">{{ stat.fail }}</span>
              <span class="fluent-stat-unit">项</span>
            </div>
          </div>
        </div>

        <!-- 实时速度 -->
        <div class="fluent-stat-tile">
          <div class="fluent-stat-icon-box" style="background: var(--fluent-accent-subtle); color: var(--fluent-accent);">
            <svg width="18" height="18" viewBox="0 0 16 16" fill="currentColor">
              <path d="M8 2a6 6 0 00-6 6c0 1.887.87 3.57 2.235 4.675.244.198.59.186.82-.045.247-.247.23-.65-.035-.87A4.75 4.75 0 013.25 8a4.75 4.75 0 119.5 0c0 1.488-.678 2.818-1.74 3.702-.262.219-.281.62-.036.868.228.23.575.242.82.045A6.002 6.002 0 008 2zm1.28 4.72a.75.75 0 00-1.06 0L6.47 8.47a.75.75 0 101.06 1.06l1.75-1.75a.75.75 0 000-1.06z"/>
            </svg>
          </div>
          <div class="fluent-stat-content">
            <span class="fluent-stat-category">实时下载速度</span>
            <div class="fluent-stat-val-row">
              <span class="fluent-stat-value" style="color: var(--fluent-accent);">{{ speedValue }}</span>
              <span class="fluent-stat-unit">{{ speedUnit }}</span>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- 卡片 3：运行日志控制台 (Fluent Terminal 风格) -->
    <section class="fluent-card fluent-log-card">
      <div class="fluent-log-header">
        <div class="fluent-header-content">
          <div class="fluent-title-wrap">
            <svg class="fluent-icon-primary" viewBox="0 0 16 16" width="15" height="15" fill="currentColor">
              <path d="M2.5 1.5A1.5 1.5 0 001 3v10a1.5 1.5 0 001.5 1.5h11A1.5 1.5 0 0015 13V3a1.5 1.5 0 00-1.5-1.5h-11zm2.146 4.146a.5.5 0 01.708 0L7.5 7.793l2.146-2.147a.5.5 0 01.708.708l-2.5 2.5a.5.5 0 01-.708 0l-2.5-2.5a.5.5 0 010-.708zM4 11h8a.5.5 0 010 1H4a.5.5 0 010-1z"/>
            </svg>
            <span class="fluent-card-title">运行日志</span>
          </div>
          <span class="fluent-card-subtitle">实时输出 GraphQL 请求与并发流式文件写入状态</span>
        </div>
        <div class="fluent-log-actions">
          <button class="fluent-btn fluent-btn-standard fluent-btn-sm" @click="copyLog">
            <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
              <path d="M4 1.5A1.5 1.5 0 015.5 0h6A1.5 1.5 0 0113 1.5v1H5.5A2.5 2.5 0 003 5v7H2.5A1.5 1.5 0 011 10.5v-9z"/>
              <path d="M4.5 4A1.5 1.5 0 016 2.5h6A1.5 1.5 0 0113.5 4v10a1.5 1.5 0 01-1.5 1.5H6A1.5 1.5 0 014.5 14V4zm1.5-.5a.5.5 0 00-.5.5v10a.5.5 0 00.5.5h6a.5.5 0 00.5-.5V4a.5.5 0 00-.5-.5H6z"/>
            </svg>
            <span>复制</span>
          </button>
          <button class="fluent-btn fluent-btn-standard fluent-btn-sm" @click="clearLog">
            <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
              <path d="M5.5 5.5A.5.5 0 016 6v6a.5.5 0 01-1 0V6a.5.5 0 01.5-.5zm2.5 0a.5.5 0 01.5.5v6a.5.5 0 01-1 0V6a.5.5 0 01.5-.5zm3 .5a.5.5 0 00-1 0v6a.5.5 0 001 0V6z"/>
              <path fill-rule="evenodd" d="M14.5 3a1 1 0 01-1 1H13v9a2 2 0 01-2 2H5a2 2 0 01-2-2V4h-.5a1 1 0 01-1-1V2a1 1 0 011-1H6a1 1 0 011-1h2a1 1 0 011 1h3.5a1 1 0 011 1v1zM4.118 4L4 4.059V13a1 1 0 001 1h6a1 1 0 001-1V4.059L11.882 4H4.118zM2.5 3V2h11v1h-11z"/>
            </svg>
            <span>清空</span>
          </button>
        </div>
      </div>

      <div class="fluent-log-terminal" ref="logArea">
        <div v-for="(line, i) in logLines" :key="i" class="fluent-log-line" :class="'fluent-log-' + line.tag">
          <span class="fluent-log-ts">{{ line.ts }}</span>
          <span class="fluent-log-msg">{{ line.msg }}</span>
        </div>
      </div>
    </section>
  </div>
</div>
</template>

<script setup>
import { reactive, ref, computed, onMounted, onUnmounted, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

// ---------------- 自定义无边框窗口控制 ----------------
const isMaximized = ref(false)

async function checkMaximized() {
  try {
    isMaximized.value = await invoke('win_is_maximized')
  } catch (e) {
    console.error('Failed to check window maximized state:', e)
  }
}

async function minimizeWindow() {
  try {
    await invoke('win_minimize')
  } catch (e) {
    console.error('Failed to minimize window:', e)
  }
}

async function toggleMaximize() {
  try {
    isMaximized.value = await invoke('win_toggle_maximize')
  } catch (e) {
    console.error('Failed to toggle maximize window:', e)
  }
}

async function closeWindow() {
  try {
    await invoke('win_close')
  } catch (e) {
    console.error('Failed to close window:', e)
  }
}

// ---------------- 核心响应式状态 ----------------
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
const statusColor = ref('var(--fluent-text-secondary)')

const stat = reactive({ down: 0, skip: 0, fail: 0 })
const speedValue = ref('0')
const speedUnit = ref('B/s')
const progress = reactive({ current: 0, total: 0 })

const logLines = ref([])
const logArea = ref(null)
let unlisteners = []

// ---------------- 账户 ID 下拉框 (Fluent Flyout) ----------------
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

// ---------------- 时间范围计算 ----------------
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

  printLog('欢迎使用 X 媒体下载器！', 'info')
  printLog('基于媒体唯一 ID 去重，重复运行不会重复下载；回车开始 / Esc 取消。', 'info')

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
  checkMaximized()
  window.addEventListener('resize', checkMaximized)
})

onUnmounted(() => {
  unlisteners.forEach((fn) => fn())
  window.removeEventListener('keydown', onKeydown)
  document.removeEventListener('mousedown', onDocumentMouseDown)
  window.removeEventListener('resize', checkMaximized)
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
  statusColor.value = 'var(--fluent-accent)'
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
    statusColor.value = 'var(--fluent-text-secondary)'
  }
}

async function cancelDownload() {
  if (!running.value || cancelling.value) return
  cancelling.value = true
  statusText.value = '正在取消…'
  statusColor.value = 'var(--fluent-warning)'
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
    statusColor.value = 'var(--fluent-danger)'
  } else {
    statusText.value = '已完成'
    statusColor.value = 'var(--fluent-success)'
  }
}
</script>

<style scoped>
.app-root {
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-height: 0;
  flex: 1;
  overflow: hidden;
  padding: 8px 12px 12px;
}

/* Fluent 2 亚克力 / 云母微光卡片 */
.fluent-card {
  background: var(--fluent-card-bg);
  border: 1px solid var(--fluent-card-stroke);
  border-radius: var(--fluent-radius-card);
  box-shadow: var(--fluent-shadow-card);
  backdrop-filter: blur(25px);
  position: relative;
  transition: border-color 0.2s cubic-bezier(0.1, 0.9, 0.2, 1);
}

.fluent-card:hover {
  border-color: var(--fluent-card-stroke-highlight);
}

.fluent-card:not(.fluent-log-card) {
  flex-shrink: 0;
}

/* 卡片标题区 (Fluent 2 规范) */
.fluent-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 14px 4px;
}

.fluent-header-content {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.fluent-title-wrap {
  display: flex;
  align-items: center;
  gap: 7px;
}

.fluent-icon-primary {
  color: var(--fluent-accent);
  opacity: 0.95;
}

.fluent-card-title {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--fluent-text-primary);
  letter-spacing: 0.1px;
}

.fluent-card-subtitle {
  font-size: 11px;
  color: var(--fluent-text-secondary);
}

.fluent-card-body {
  padding: 2px 8px 6px;
}

/* Fluent 2 设置项行 (SettingsCard 风格) */
.fluent-row {
  display: flex;
  align-items: center;
  padding: 4px 8px;
  border-radius: var(--fluent-radius-control);
  gap: 10px;
  transition: background 0.12s ease;
}

.fluent-row.zebra {
  background: var(--fluent-row-bg);
}

.fluent-row:hover {
  background: var(--fluent-row-hover);
}

.fluent-row-label {
  width: 92px;
  flex-shrink: 0;
  font-size: 12px;
  font-weight: 500;
  color: var(--fluent-text-secondary);
  display: flex;
  align-items: center;
  gap: 6px;
}

.fluent-row-icon {
  color: var(--fluent-text-tertiary);
  flex-shrink: 0;
}

.fluent-control-container {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 8px;
}

.fluent-path-group {
  display: flex;
  gap: 8px;
}

.fluent-path-group input {
  flex: 1;
}

/* Fluent 2 TextBox 输入框 */
.fluent-textbox {
  width: 100%;
  height: 30px;
  background: var(--fluent-control-bg);
  color: var(--fluent-text-primary);
  border: 1px solid var(--fluent-control-stroke);
  border-bottom: 1px solid var(--fluent-control-elevation);
  border-radius: var(--fluent-radius-control);
  padding: 0 10px;
  font-family: 'Cascadia Code', Consolas, monospace;
  font-size: 12px;
  outline: none;
  transition: all 0.12s ease;
  box-shadow: var(--fluent-shadow-control);
}

.fluent-textbox:hover:not(:disabled) {
  background: var(--fluent-control-bg-hover);
  border-color: rgba(255, 255, 255, 0.14);
}

.fluent-textbox:focus {
  background: var(--fluent-control-bg-focus);
  border-color: rgba(255, 255, 255, 0.1);
  border-bottom: 2px solid var(--fluent-accent);
  box-shadow: 0 0 0 1px rgba(96, 205, 255, 0.15);
}

.fluent-textbox::placeholder {
  color: var(--fluent-text-tertiary);
  font-family: 'Segoe UI Variable Text', -apple-system, BlinkMacSystemFont, sans-serif;
  font-size: 11.5px;
}

/* Fluent 2 组合输入框 (ComboBox) */
.fluent-combo {
  position: relative;
  flex: 1;
  min-width: 0;
}

.fluent-combo input {
  padding-right: 32px;
}

.fluent-combo-button {
  position: absolute;
  top: 1px;
  right: 1px;
  bottom: 1px;
  width: 30px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  color: var(--fluent-text-secondary);
  border: none;
  cursor: pointer;
  border-radius: 0 var(--fluent-radius-control) var(--fluent-radius-control) 0;
  transition: all 0.12s ease;
}

.fluent-combo-button:hover {
  background: rgba(255, 255, 255, 0.08);
  color: var(--fluent-text-primary);
}

.fluent-combo-button.open svg {
  transform: rotate(180deg);
}

/* Fluent 2 MenuFlyout 悬浮菜单 */
.fluent-flyout {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  right: 0;
  z-index: 100;
  background: var(--fluent-flyout-bg);
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: var(--fluent-radius-card);
  box-shadow: var(--fluent-shadow-flyout);
  padding: 4px;
  max-height: 180px;
  overflow-y: auto;
  backdrop-filter: blur(25px);
}

.fluent-flyout-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 5px 8px;
  border-radius: var(--fluent-radius-control);
  font-family: 'Cascadia Code', Consolas, monospace;
  font-size: 12px;
  color: var(--fluent-text-primary);
  cursor: pointer;
  transition: background 0.1s;
}

.fluent-flyout-pill {
  width: 3px;
  height: 12px;
  background: transparent;
  border-radius: 2px;
  transition: background 0.15s;
}

.fluent-flyout-item:hover,
.fluent-flyout-item.active {
  background: rgba(255, 255, 255, 0.08);
}

.fluent-flyout-item.active .fluent-flyout-pill {
  background: var(--fluent-accent);
}

.fluent-flyout-text {
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.fluent-flyout-empty {
  padding: 8px 10px;
  font-size: 11.5px;
  color: var(--fluent-text-tertiary);
  text-align: center;
}

/* Fluent 2 日期选择器组 */
.fluent-date-container {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}

.fluent-date-group {
  display: flex;
  align-items: center;
  gap: 2px;
  background: var(--fluent-control-bg);
  border: 1px solid var(--fluent-control-stroke);
  border-bottom: 1px solid var(--fluent-control-elevation);
  border-radius: var(--fluent-radius-control);
  padding: 1px 4px;
}

.fluent-date-tag {
  font-size: 11px;
  font-weight: 600;
  color: var(--fluent-text-secondary);
  padding: 0 4px;
}

.fluent-date-separator {
  font-size: 12px;
  color: var(--fluent-text-secondary);
}

.fluent-select {
  height: 27px;
  background: transparent;
  color: var(--fluent-text-primary);
  border: none;
  outline: none;
  font-size: 11.5px;
  font-family: inherit;
  padding: 0 4px;
  cursor: pointer;
}

.fluent-select option {
  background: #252525;
  color: #fff;
}

.fluent-select:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.06);
  border-radius: 3px;
}

.fluent-select:disabled {
  opacity: 0.38;
  cursor: not-allowed;
}

.fluent-concurrency-select {
  height: 30px;
  width: 58px;
  background: var(--fluent-control-bg);
  border: 1px solid var(--fluent-control-stroke);
  border-bottom: 1px solid var(--fluent-control-elevation);
  border-radius: var(--fluent-radius-control);
  text-align: center;
}

/* Fluent 2 Checkbox */
.fluent-checkbox {
  display: flex;
  align-items: center;
  gap: 7px;
  cursor: pointer;
  margin-left: 6px;
  user-select: none;
}

.fluent-checkbox input {
  display: none;
}

.fluent-checkbox-box {
  width: 17px;
  height: 17px;
  background: var(--fluent-control-bg);
  border: 1px solid rgba(255, 255, 255, 0.4);
  border-radius: var(--fluent-radius-control);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.12s ease;
}

.fluent-checkbox:hover .fluent-checkbox-box {
  background: rgba(255, 255, 255, 0.1);
  border-color: rgba(255, 255, 255, 0.6);
}

.fluent-checkbox input:checked + .fluent-checkbox-box {
  background: var(--fluent-accent);
  border-color: var(--fluent-accent);
}

.fluent-checkbox-label {
  font-size: 12px;
  color: var(--fluent-text-primary);
}

/* Fluent 2 按键系统 */
.fluent-btn {
  height: 30px;
  padding: 0 12px;
  font-size: 12px;
  font-weight: 500;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  border-radius: var(--fluent-radius-control);
  cursor: pointer;
  transition: all 0.1s cubic-bezier(0.1, 0.9, 0.2, 1);
  user-select: none;
  white-space: nowrap;
}

.fluent-btn:active:not(:disabled) {
  transform: scale(0.98);
}

.fluent-btn:disabled {
  opacity: 0.36;
  cursor: not-allowed;
}

/* Standard Button */
.fluent-btn-standard {
  background: var(--fluent-control-bg);
  border: 1px solid var(--fluent-control-stroke);
  border-bottom: 1px solid var(--fluent-control-elevation);
  color: var(--fluent-text-primary);
}

.fluent-btn-standard:hover:not(:disabled) {
  background: var(--fluent-control-bg-hover);
  border-color: rgba(255, 255, 255, 0.15);
}

.fluent-btn-sm {
  height: 25px;
  padding: 0 9px;
  font-size: 11px;
}

/* Accent Button (Windows 11 经典亮色主操作) */
.fluent-btn-accent {
  background: linear-gradient(180deg, #60cdff 0%, #3bb8fa 100%);
  color: var(--fluent-accent-text);
  font-weight: 600;
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-bottom: 1px solid rgba(0, 0, 0, 0.35);
  box-shadow: 0 2px 8px rgba(96, 205, 255, 0.3);
}

.fluent-btn-accent:hover:not(:disabled) {
  background: linear-gradient(180deg, #7ad5ff 0%, #4ec1fc 100%);
  box-shadow: 0 3px 12px rgba(96, 205, 255, 0.42);
}

.fluent-btn-accent:active:not(:disabled) {
  background: #38b4f7;
}

/* Destructive / Danger Button */
.fluent-btn-danger {
  background: var(--fluent-danger-bg);
  border: 1px solid var(--fluent-danger-stroke);
  color: var(--fluent-danger);
  font-weight: 600;
}

.fluent-btn-danger:hover:not(:disabled) {
  background: rgba(255, 107, 107, 0.22);
  border-color: var(--fluent-danger);
}

/* 操作栏 */
.fluent-action-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px 8px;
  flex-wrap: wrap;
}

.fluent-divider {
  width: 1px;
  height: 18px;
  background: rgba(255, 255, 255, 0.1);
  margin: 0 2px;
}

.fluent-inline-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--fluent-text-secondary);
}

/* Fluent 2 SegmentedControl (分段切换器) */
.fluent-segmented {
  display: flex;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: var(--fluent-radius-control);
  padding: 2px;
}

.fluent-segmented-item {
  background: transparent;
  color: var(--fluent-text-secondary);
  border: 1px solid transparent;
  border-radius: 3px;
  padding: 3px 9px;
  font-size: 11.5px;
  cursor: pointer;
  transition: all 0.12s ease;
}

.fluent-segmented-item:hover:not(.active) {
  color: var(--fluent-text-primary);
  background: rgba(255, 255, 255, 0.04);
}

.fluent-segmented-item.active {
  background: rgba(255, 255, 255, 0.11);
  color: #fff;
  font-weight: 600;
  border-color: rgba(255, 255, 255, 0.08);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25);
}

/* Fluent 2 细条平滑进度条 */
.fluent-progress-section {
  padding: 0 14px 8px;
}

.fluent-progress-track {
  width: 100%;
  height: 3px;
  background: rgba(255, 255, 255, 0.08);
  border-radius: 2px;
  overflow: hidden;
}

.fluent-progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #4cc2f7, #60cdff);
  border-radius: 2px;
  transition: width 0.25s ease;
  box-shadow: 0 0 6px rgba(96, 205, 255, 0.4);
}

.fluent-progress-info {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 5px;
}

.fluent-progress-text {
  font-size: 11.5px;
  font-weight: 600;
  color: var(--fluent-text-secondary);
}

/* Fluent InfoBadge 状态指示 */
.fluent-status-badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 2px 8px;
  border-radius: var(--fluent-radius-pill);
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.06);
}

.fluent-status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
}

.fluent-status-text {
  font-size: 11.5px;
  font-weight: 600;
}

/* Fluent 2 InfoCard 统计看板 (图吧工具箱指标磁贴质感) */
.fluent-stats-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 8px;
  padding: 0 12px 10px;
}

.fluent-stat-tile {
  background: rgba(255, 255, 255, 0.035);
  border: 1px solid rgba(255, 255, 255, 0.065);
  border-radius: var(--fluent-radius-tile);
  padding: 7px 10px;
  display: flex;
  align-items: center;
  gap: 10px;
  transition: all 0.15s ease;
}

.fluent-stat-tile:hover {
  background: rgba(255, 255, 255, 0.055);
  border-color: rgba(255, 255, 255, 0.12);
  transform: translateY(-1px);
}

.fluent-stat-icon-box {
  width: 34px;
  height: 34px;
  flex-shrink: 0;
  border-radius: var(--fluent-radius-control);
  display: flex;
  align-items: center;
  justify-content: center;
}

.fluent-stat-content {
  display: flex;
  flex-direction: column;
  min-width: 0;
  flex: 1;
}

.fluent-stat-category {
  font-size: 11px;
  color: var(--fluent-text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.fluent-stat-val-row {
  display: flex;
  align-items: baseline;
  gap: 3px;
}

.fluent-stat-value {
  font-size: 19px;
  font-weight: 700;
  font-family: 'Segoe UI Variable Display', 'Segoe UI', system-ui, sans-serif;
  line-height: 1.15;
  letter-spacing: -0.3px;
}

.fluent-stat-unit {
  font-size: 10.5px;
  font-weight: 500;
  color: var(--fluent-text-secondary);
}

/* 运行日志卡片 (Fluent 2 Terminal 风格 - 紧凑缩短) */
.fluent-log-card {
  flex: 0 0 auto;
  max-height: 180px;
  min-height: 130px;
  display: flex;
  flex-direction: column;
}

.fluent-log-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 7px 14px 4px;
}

.fluent-log-actions {
  display: flex;
  gap: 6px;
}

.fluent-log-terminal {
  height: 105px;
  margin: 0 12px 10px;
  background: var(--fluent-log-bg);
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: var(--fluent-radius-control);
  padding: 6px 10px;
  overflow-y: auto;
  font-family: 'Cascadia Code', Consolas, monospace;
  font-size: 11.5px;
  white-space: pre-wrap;
  word-break: break-all;
}

.fluent-log-line {
  line-height: 1.55;
}

.fluent-log-ts {
  color: var(--fluent-text-tertiary);
  margin-right: 8px;
  font-size: 11px;
}

.fluent-log-success { color: var(--fluent-success); }
.fluent-log-skip { color: var(--fluent-warning); }
.fluent-log-error { color: var(--fluent-danger); }
.fluent-log-warn { color: var(--fluent-warning); }
.fluent-log-info { color: #8ab4f8; }
.fluent-log-normal { color: var(--fluent-text-primary); }
</style>
