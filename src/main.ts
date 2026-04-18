import { invoke } from "@tauri-apps/api/core";

interface SmtcMediaProperties {
  title: string;
  artist: string;
  album_title: string;
  album_artist: string;
  subtitle: string;
  track_number: number;
  album_track_count: number;
  genres: string[];
}

interface SmtcPlaybackInfo {
  status: string;
  is_play_enabled: boolean;
  is_pause_enabled: boolean;
  is_stop_enabled: boolean;
  is_next_enabled: boolean;
  is_previous_enabled: boolean;
  is_fast_forward_enabled: boolean;
  is_rewind_enabled: boolean;
  auto_repeat_mode: string | null;
  shuffle_active: boolean | null;
  playback_rate: number | null;
  playback_type: string | null;
}

interface SmtcTimelineProperties {
  start_time_secs: number;
  end_time_secs: number;
  position_secs: number;
  min_seek_time_secs: number;
  max_seek_time_secs: number;
}

interface SmtcSession {
  source_app_user_model_id: string;
  is_current: boolean;
  media_properties: SmtcMediaProperties | null;
  playback_info: SmtcPlaybackInfo | null;
  timeline_properties: SmtcTimelineProperties | null;
}

const POLL_INTERVAL = 1000;

function formatSecs(secs: number): string {
  if (!isFinite(secs) || secs < 0) return "0:00";
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = Math.floor(secs % 60);
  if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  return `${m}:${String(s).padStart(2, "0")}`;
}

function statusClass(status: string): string {
  switch (status) {
    case "Playing": return "status-playing";
    case "Paused":  return "status-paused";
    case "Stopped": return "status-stopped";
    default:        return "status-other";
  }
}

function row(label: string, value: string | number | boolean | null | undefined): string {
  if (value === null || value === undefined || value === "") return "";
  const display = typeof value === "boolean" ? (value ? "true" : "false") : String(value);
  return `<tr><td class="info-key">${label}</td><td class="info-val">${display}</td></tr>`;
}

function renderControls(p: SmtcPlaybackInfo): string {
  const controls = [
    ["⏮ Prev",    p.is_previous_enabled],
    ["▶ Play",    p.is_play_enabled],
    ["⏸ Pause",   p.is_pause_enabled],
    ["⏹ Stop",    p.is_stop_enabled],
    ["⏭ Next",    p.is_next_enabled],
    ["⏩ FF",      p.is_fast_forward_enabled],
    ["⏪ Rewind",  p.is_rewind_enabled],
  ] as [string, boolean][];

  return controls
    .map(([label, enabled]) =>
      `<span class="ctrl-btn ${enabled ? "ctrl-on" : "ctrl-off"}">${label}</span>`
    )
    .join("");
}

function renderSession(s: SmtcSession): string {
  const mp = s.media_properties;
  const pi = s.playback_info;
  const tl = s.timeline_properties;

  const statusStr = pi?.status ?? "Unknown";
  const progressPct =
    tl && tl.end_time_secs > 0
      ? Math.min(100, (tl.position_secs / tl.end_time_secs) * 100)
      : 0;

  let html = `
    <div class="session-card ${s.is_current ? "session-current" : ""}">
      <div class="session-header">
        <div class="session-title-row">
          ${s.is_current ? '<span class="current-badge">● 当前</span>' : ""}
          <span class="session-appid">${s.source_app_user_model_id || "(未知应用)"}</span>
        </div>
        <span class="session-status ${statusClass(statusStr)}">${statusStr}</span>
      </div>`;

  if (mp) {
    html += `
      <section class="info-section">
        <div class="section-title">🎵 媒体信息</div>
        <table class="info-table">
          ${row("标题", mp.title)}
          ${row("艺术家", mp.artist)}
          ${row("专辑", mp.album_title)}
          ${row("专辑艺术家", mp.album_artist)}
          ${row("副标题", mp.subtitle)}
          ${row("曲目编号", mp.track_number > 0 ? `${mp.track_number}${mp.album_track_count > 0 ? " / " + mp.album_track_count : ""}` : null)}
          ${row("曲风", mp.genres.length > 0 ? mp.genres.join(", ") : null)}
        </table>
      </section>`;
  }

  if (pi) {
    html += `
      <section class="info-section">
        <div class="section-title">▶ 播放信息</div>
        <table class="info-table">
          ${row("播放类型", pi.playback_type)}
          ${row("播放速率", pi.playback_rate !== null ? `${pi.playback_rate}x` : null)}
          ${row("循环模式", pi.auto_repeat_mode)}
          ${row("随机播放", pi.shuffle_active !== null ? (pi.shuffle_active ? "开启" : "关闭") : null)}
        </table>
        <div class="controls-row">${renderControls(pi)}</div>
      </section>`;
  }

  if (tl) {
    const pos = formatSecs(tl.position_secs);
    const end = formatSecs(tl.end_time_secs);
    html += `
      <section class="info-section">
        <div class="section-title">⏱ 时间轴</div>
        <div class="progress-bar-wrap">
          <div class="progress-bar" style="width:${progressPct.toFixed(1)}%"></div>
        </div>
        <div class="progress-labels">
          <span>${pos}</span><span>${end}</span>
        </div>
        <table class="info-table">
          ${row("当前位置", pos)}
          ${row("总时长", end)}
          ${row("起始时间", formatSecs(tl.start_time_secs))}
          ${row("最小可跳", formatSecs(tl.min_seek_time_secs))}
          ${row("最大可跳", formatSecs(tl.max_seek_time_secs))}
        </table>
      </section>`;
  }

  html += `</div>`;
  return html;
}

async function poll() {
  const container = document.getElementById("sessions-container")!;
  const statusDot  = document.getElementById("status-dot")!;
  const statusText = document.getElementById("status-text")!;
  const lastUpdate = document.getElementById("last-update")!;

  try {
    const sessions: SmtcSession[] = await invoke("get_smtc_sessions");

    statusDot.className = "status-dot dot-ok";
    const now = new Date().toLocaleTimeString("zh-CN");
    lastUpdate.textContent = `更新于 ${now}`;

    if (sessions.length === 0) {
      statusText.textContent = "无活动会话";
      container.innerHTML = `
        <div class="empty-state">
          <div class="empty-icon">🔇</div>
          <p>当前没有 SMTC 媒体会话</p>
          <small>请播放音乐、视频等媒体后刷新</small>
        </div>`;
      return;
    }

    statusText.textContent = `${sessions.length} 个会话`;
    container.innerHTML = sessions.map(renderSession).join("");
  } catch (e) {
    statusDot.className = "status-dot dot-err";
    statusText.textContent = "获取失败";
    console.error("SMTC error:", e);
  }
}

window.addEventListener("DOMContentLoaded", () => {
  document.getElementById("btn-refresh")?.addEventListener("click", poll);
  poll();
  setInterval(poll, POLL_INTERVAL);
});
