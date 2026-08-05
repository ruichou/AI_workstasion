import "./styles.css";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

interface SysInfo {
  cpu: number;
  mem_used: number;
  mem_total: number;
  disk_used: number;
  disk_total: number;
}

interface Temps {
  cpu: number | null;
  disk: number | null;
}

interface Weather {
  city: string;
  temp: number;
  feels: number;
  humidity: number;
  wind: number;
  code: number;
  is_day: boolean;
}

interface AppItem {
  name: string;
  path: string;
  emoji: string;
  args: string | null;
}

interface Config {
  city: string;
  lat: number | null;
  lon: number | null;
  apps: AppItem[];
}

const $ = (id: string) => document.getElementById(id)!;

const WEEK_CN = ["日", "一", "二", "三", "四", "五", "六"];

function gb(bytes: number): string {
  return (bytes / 1024 ** 3).toFixed(1);
}

function fmtClock() {
  const now = new Date();
  const hh = String(now.getHours()).padStart(2, "0");
  const mm = String(now.getMinutes()).padStart(2, "0");
  const ss = String(now.getSeconds()).padStart(2, "0");
  $("clock-time").textContent = `${hh}:${mm}`;
  $("clock-sec").textContent = ss;
  $("clock-date").textContent = `${now.getFullYear()}年${now.getMonth() + 1}月${now.getDate()}日 星期${WEEK_CN[now.getDay()]}`;
}

let calYear = new Date().getFullYear();
let calMonth = new Date().getMonth();

function renderCalendar() {
  $("cal-title").textContent = `${calYear}年${calMonth + 1}月`;
  const grid = $("cal-grid");
  grid.innerHTML = "";
  const head = ["一", "二", "三", "四", "五", "六", "日"];
  head.forEach((d) => {
    const el = document.createElement("div");
    el.className = "cal-dow";
    el.textContent = d;
    grid.appendChild(el);
  });
  const first = new Date(calYear, calMonth, 1);
  const offset = (first.getDay() + 6) % 7;
  const days = new Date(calYear, calMonth + 1, 0).getDate();
  const today = new Date();
  for (let i = 0; i < offset; i++) {
    grid.appendChild(document.createElement("div"));
  }
  for (let d = 1; d <= days; d++) {
    const el = document.createElement("div");
    el.className = "cal-day";
    el.textContent = String(d);
    if (
      d === today.getDate() &&
      calMonth === today.getMonth() &&
      calYear === today.getFullYear()
    ) {
      el.classList.add("cal-today");
    }
    grid.appendChild(el);
  }
}

async function refreshSys() {
  try {
    const s = await invoke<SysInfo>("get_sysinfo");
    const cpu = Math.round(s.cpu);
    const memPct = s.mem_total ? (s.mem_used / s.mem_total) * 100 : 0;
    const diskPct = s.disk_total ? (s.disk_used / s.disk_total) * 100 : 0;
    $("val-cpu").textContent = `${cpu}%`;
    $("val-mem").textContent = `${gb(s.mem_used)}/${gb(s.mem_total)}G`;
    $("val-disk").textContent = `${gb(s.disk_used)}/${gb(s.disk_total)}G`;
    $("bar-cpu").style.width = `${Math.min(cpu, 100)}%`;
    $("bar-mem").style.width = `${Math.min(memPct, 100)}%`;
    $("bar-disk").style.width = `${Math.min(diskPct, 100)}%`;
  } catch {
    /* ignore */
  }
}

const WMO_EMOJI: Record<number, string> = {
  0: "☀️", 1: "🌤", 2: "⛅", 3: "☁️", 45: "🌫", 48: "🌫", 51: "🌦", 53: "🌦",
  55: "🌦", 56: "🌧", 57: "🌧", 61: "🌧", 63: "🌧", 65: "🌧", 66: "🌧", 67: "🌧",
  71: "🌨", 73: "🌨", 75: "🌨", 77: "❄️", 80: "🌦", 81: "🌦", 82: "🌦", 85: "🌨",
  86: "🌨", 95: "⛈", 96: "⛈", 99: "⛈",
};

const WMO_DESC: Record<number, string> = {
  0: "晴", 1: "晴间多云", 2: "多云", 3: "阴", 45: "雾", 48: "雾", 51: "毛毛雨",
  53: "毛毛雨", 55: "毛毛雨", 56: "冻毛毛雨", 57: "冻毛毛雨", 61: "小雨", 63: "中雨",
  65: "大雨", 66: "冻雨", 67: "冻雨", 71: "小雪", 73: "中雪", 75: "大雪", 77: "雪粒",
  80: "阵雨", 81: "阵雨", 82: "强阵雨", 85: "阵雪", 86: "强阵雪", 95: "雷暴",
  96: "雷暴伴冰雹", 99: "雷暴伴冰雹",
};

async function refreshWeather() {
  try {
    const w = await invoke<Weather>("get_weather");
    $("w-emoji").textContent = WMO_EMOJI[w.code] ?? "🌡";
    $("w-temp").textContent = `${Math.round(w.temp)}°`;
    $("w-desc").textContent = `${WMO_DESC[w.code] ?? "未知"} · 体感 ${Math.round(w.feels)}°`;
    $("w-city").textContent = `📍 ${w.city}`;
    $("w-humid").textContent = `💧 湿度 ${Math.round(w.humidity)}%`;
    $("w-wind").textContent = `🌬 风速 ${Math.round(w.wind)} km/h`;
  } catch (e) {
    $("w-desc").textContent = String(e);
  }
}

function renderApps(apps: AppItem[]) {
  const grid = $("apps-grid");
  grid.innerHTML = "";
  $("apps-empty").classList.toggle("hidden", apps.length > 0);
  for (const app of apps) {
    const tile = document.createElement("button");
    tile.className = "app-tile";
    tile.title = `${app.name}\n${app.path}`;
    tile.innerHTML = `<span class="app-ico">${app.emoji || "📦"}</span><span class="app-name">${app.name}</span>`;
    tile.addEventListener("click", () => {
      invoke("launch_app", { path: app.path, args: app.args }).catch((e) =>
        alert(e),
      );
    });
    grid.appendChild(tile);
  }
}

async function refreshApps() {
  try {
    const cfg = await invoke<Config>("get_config");
    renderApps(cfg.apps);
  } catch {
    /* ignore */
  }
}

let lastOpacity = 1;

function initSettings() {
  const win = getCurrentWindow();
  const panel = $("settings-panel");
  const sliderOp = $("slider-opacity") as HTMLInputElement;
  const sliderVol = $("slider-volume") as HTMLInputElement;
  const sliderEye = $("slider-eyecare") as HTMLInputElement;
  const toggleEye = $("toggle-eyecare") as HTMLInputElement;

  const savedOp = Number(localStorage.getItem("opacity") ?? "100");
  lastOpacity = savedOp / 100;
  sliderOp.value = String(savedOp);
  $("val-opacity").textContent = `${savedOp}%`;
  invoke("set_window_opacity", { level: lastOpacity }).catch(() => {});

  const savedEye = localStorage.getItem("eyecare") === "1";
  const savedEyeInt = Number(localStorage.getItem("eyecare-int") ?? "35");
  toggleEye.checked = savedEye;
  sliderEye.value = String(savedEyeInt);
  sliderEye.disabled = !savedEye;
  $("val-eyecare").textContent = `${savedEyeInt}%`;
  if (savedEye) {
    invoke("set_eye_care", { enabled: true, intensity: savedEyeInt / 100 }).catch(() => {});
  }

  invoke<number | null>("get_volume")
    .then((v) => {
      if (v != null) {
        const pct = Math.round(v * 100);
        sliderVol.value = String(pct);
        $("val-volume").textContent = `${pct}%`;
      }
    })
    .catch(() => {});

  $("btn-settings").addEventListener("click", () => {
    panel.classList.toggle("hidden");
  });

  sliderOp.addEventListener("input", () => {
    const pct = Number(sliderOp.value);
    lastOpacity = pct / 100;
    $("val-opacity").textContent = `${pct}%`;
    localStorage.setItem("opacity", String(pct));
    invoke("set_window_opacity", { level: lastOpacity }).catch(() => {});
  });

  sliderVol.addEventListener("input", () => {
    const pct = Number(sliderVol.value);
    $("val-volume").textContent = `${pct}%`;
    invoke("set_volume", { level: pct / 100 }).catch(() => {});
  });

  toggleEye.addEventListener("change", () => {
    const on = toggleEye.checked;
    sliderEye.disabled = !on;
    localStorage.setItem("eyecare", on ? "1" : "0");
    invoke("set_eye_care", {
      enabled: on,
      intensity: Number(sliderEye.value) / 100,
    }).catch(() => {});
  });

  sliderEye.addEventListener("input", () => {
    const pct = Number(sliderEye.value);
    $("val-eyecare").textContent = `${pct}%`;
    localStorage.setItem("eyecare-int", String(pct));
    if (toggleEye.checked) {
      invoke("set_eye_care", { enabled: true, intensity: pct / 100 }).catch(() => {});
    }
  });

  win.onFocusChanged(({ payload }) => {
    if (payload && lastOpacity < 0.05) {
      lastOpacity = 0.6;
      localStorage.setItem("opacity", "60");
      sliderOp.value = "60";
      $("val-opacity").textContent = "60%";
      invoke("set_window_opacity", { level: 0.6 }).catch(() => {});
    }
  });
}

async function refreshTemps() {
  try {
    const t = await invoke<Temps>("get_temps");
    $("val-cputemp").textContent =
      t.cpu != null ? `${t.cpu.toFixed(1)}°C` : "--";
    $("val-disktemp").textContent =
      t.disk != null ? `${t.disk.toFixed(1)}°C` : "--";
  } catch {
    /* ignore */
  }
}

async function refreshAll() {
  refreshSys();
  refreshTemps();
  refreshWeather();
  refreshApps();
}

function init() {
  fmtClock();
  renderCalendar();
  refreshAll();
  initSettings();
  setInterval(fmtClock, 1000);
  setInterval(refreshSys, 2000);
  setInterval(refreshTemps, 5000);
  setInterval(refreshWeather, 20 * 60 * 1000);

  const win = getCurrentWindow();
  $("btn-min").addEventListener("click", () => win.minimize());
  $("btn-close").addEventListener("click", () => win.close());
  let pinned = true;
  $("btn-pin").addEventListener("click", () => {
    pinned = !pinned;
    win.setAlwaysOnTop(pinned);
    $("btn-pin").style.opacity = pinned ? "1" : "0.4";
  });
  $("btn-config").addEventListener("click", () => {
    invoke("open_config").catch((e) => alert(e));
  });
  $("cal-prev").addEventListener("click", () => {
    calMonth--;
    if (calMonth < 0) {
      calMonth = 11;
      calYear--;
    }
    renderCalendar();
  });
  $("cal-next").addEventListener("click", () => {
    calMonth++;
    if (calMonth > 11) {
      calMonth = 0;
      calYear++;
    }
    renderCalendar();
  });
}

init();
