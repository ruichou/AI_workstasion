import "./styles.css";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow, LogicalSize, LogicalPosition, PhysicalSize, PhysicalPosition, Window as TauriWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";

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
  ai_url: string;
  off_time: string;
  apps: AppItem[];
}

interface Todo {
  text: string;
  time: string;
  done: boolean;
}

const $ = (id: string) => document.getElementById(id)!;

const WEEK_CN = ["日", "一", "二", "三", "四", "五", "六"];

// ---------- 农历（solarlunar 精简版） ----------
const LUNAR_INFO = [
  0x04bd8, 0x04ae0, 0x0a570, 0x054d5, 0x0d260, 0x0d950, 0x16554, 0x056a0, 0x09ad0, 0x055d2,
  0x04ae0, 0x0a5b6, 0x0a4d0, 0x0d250, 0x1d255, 0x0b540, 0x0d6a0, 0x0ada2, 0x095b0, 0x14977,
  0x04970, 0x0a4b0, 0x0b4b5, 0x06a50, 0x06d40, 0x1ab54, 0x02b60, 0x09570, 0x052f2, 0x04970,
  0x06566, 0x0d4a0, 0x0ea50, 0x06e95, 0x05ad0, 0x02b60, 0x186e3, 0x092e0, 0x1c8d7, 0x0c950,
  0x0d4a0, 0x1d8a6, 0x0b550, 0x056a0, 0x1a5b4, 0x025d0, 0x092d0, 0x0d2b2, 0x0a950, 0x0b557,
  0x06ca0, 0x0b550, 0x15355, 0x04da0, 0x0a5b0, 0x14573, 0x052b0, 0x0a9a8, 0x0e950, 0x06aa0,
  0x0aea6, 0x0ab50, 0x04b60, 0x0aae4, 0x0a570, 0x05260, 0x0f263, 0x0d950, 0x05b57, 0x056a0,
  0x096d0, 0x04dd5, 0x04ad0, 0x0a4d0, 0x0d4d4, 0x0d250, 0x0d558, 0x0b540, 0x0b6a0, 0x195a6,
  0x095b0, 0x049b0, 0x0a974, 0x0a4b0, 0x0b27a, 0x06a50, 0x06d40, 0x0af46, 0x0ab60, 0x09570,
  0x04af5, 0x04970, 0x064b0, 0x074a3, 0x0ea50, 0x06b58, 0x05ac0, 0x0ab60, 0x096d5, 0x092e0,
  0x0c960, 0x0d954, 0x0d4a0, 0x0da50, 0x07552, 0x056a0, 0x0abb7, 0x025d0, 0x092d0, 0x0cab5,
  0x0a950, 0x0b4a0, 0x0baa4, 0x0ad50, 0x055d9, 0x04ba0, 0x0a5b0, 0x15176, 0x052b0, 0x0a930,
  0x07954, 0x06aa0, 0x0ad50, 0x05b52, 0x04b60, 0x0a6e6, 0x0a4e0, 0x0d260, 0x0ea65, 0x0d530,
  0x05aa0, 0x076a3, 0x096d0, 0x04afb, 0x04ad0, 0x0a4d0, 0x1d0b6, 0x0d250, 0x0d520, 0x0dd45,
  0x0b5a0, 0x056d0, 0x055b2, 0x049b0, 0x0a577, 0x0a4b0, 0x0aa50, 0x1b255, 0x06d20, 0x0ada0,
  0x14b63, 0x09370, 0x049f8, 0x04970, 0x064b0, 0x168a6, 0x0ea50, 0x06b20, 0x1a6c4, 0x0aae0,
  0x092e0, 0x0d2e3, 0x0c960, 0x0d557, 0x0d4a0, 0x0da50, 0x05d55, 0x056a0, 0x0a6d0, 0x055d4,
  0x052d0, 0x0a9b8, 0x0a950, 0x0b4a0, 0x0b6a6, 0x0ad50, 0x055a0, 0x0aba4, 0x0a5b0, 0x052b0,
  0x0b273, 0x06930, 0x07337, 0x06aa0, 0x0ad50, 0x14b55, 0x04b60, 0x0a570, 0x054e4, 0x0d160,
  0x0e968, 0x0d520, 0x0daa0, 0x16aa6, 0x056d0, 0x04ae0, 0x0a9d4, 0x0a4d0, 0x0d150, 0x0f252,
  0x0d520,
];

const N_STR1 = ["日", "一", "二", "三", "四", "五", "六", "七", "八", "九", "十"];
const N_STR2 = ["初", "十", "廿", "卅"];
const N_STR3 = ["正", "二", "三", "四", "五", "六", "七", "八", "九", "十", "冬", "腊"];

function lunarLeapMonth(y: number): number {
  return LUNAR_INFO[y - 1900] & 0xf;
}

function lunarLeapDays(y: number): number {
  if (lunarLeapMonth(y)) {
    return LUNAR_INFO[y - 1900] & 0x10000 ? 30 : 29;
  }
  return 0;
}

function lunarMonthDays(y: number, m: number): number {
  return LUNAR_INFO[y - 1900] & (0x10000 >> m) ? 30 : 29;
}

function lunarYearDays(y: number): number {
  let sum = 348;
  for (let i = 0x8000; i > 0x8; i >>= 1) {
    sum += LUNAR_INFO[y - 1900] & i ? 1 : 0;
  }
  return sum + lunarLeapDays(y);
}

function toChinaDay(d: number): string {
  switch (d) {
    case 10:
      return "初十";
    case 20:
      return "二十";
    case 30:
      return "三十";
    default:
      return N_STR2[Math.floor(d / 10)] + N_STR1[d % 10];
  }
}

function solar2lunar(y: number, m: number, d: number): { month: number; day: number; isLeap: boolean; monthCn: string; dayCn: string } {
  const base = Date.UTC(1900, 0, 31);
  const cur = Date.UTC(y, m - 1, d);
  let offset = Math.floor((cur - base) / 86400000);
  let i = 1900;
  let temp = 0;
  for (i = 1900; i < 2101 && offset > 0; i++) {
    temp = lunarYearDays(i);
    offset -= temp;
  }
  if (offset < 0) {
    offset += temp;
    i--;
  }
  const year = i;
  const leap = lunarLeapMonth(year);
  let isLeap = false;
  let month = 0;
  let day = 0;
  for (i = 1; i < 13 && offset > 0; i++) {
    if (leap > 0 && i === leap + 1 && !isLeap) {
      --i;
      isLeap = true;
      temp = lunarLeapDays(year);
    } else {
      temp = lunarMonthDays(year, i);
    }
    if (isLeap && i === leap + 1) {
      isLeap = false;
    }
    offset -= temp;
  }
  if (offset === 0 && leap > 0 && i === leap + 1) {
    if (isLeap) {
      isLeap = false;
    } else {
      isLeap = true;
      --i;
    }
  }
  if (offset < 0) {
    offset += temp;
    --i;
  }
  month = i;
  day = offset + 1;
  return {
    month,
    day,
    isLeap,
    monthCn: (isLeap ? "闰" : "") + N_STR3[month - 1] + "月",
    dayCn: toChinaDay(day),
  };
}

// ---------- 时钟 + 农历 ----------
function fmtClock() {
  const now = new Date();
  $("clock-time").textContent = `${String(now.getHours()).padStart(2, "0")}:${String(now.getMinutes()).padStart(2, "0")}`;
  $("clock-sec").textContent = String(now.getSeconds()).padStart(2, "0");
  $("clock-date").textContent = `${now.getFullYear()}年${now.getMonth() + 1}月${now.getDate()}日 星期${WEEK_CN[now.getDay()]}`;
  const l = solar2lunar(now.getFullYear(), now.getMonth() + 1, now.getDate());
  $("clock-lunar").textContent = `农历${l.monthCn}${l.dayCn}`;
  $("mini-time").textContent = `${String(now.getHours()).padStart(2, "0")}:${String(now.getMinutes()).padStart(2, "0")}`;
  $("mini-date").textContent = `${now.getFullYear()}年${now.getMonth() + 1}月${now.getDate()}日 ${WEEK_CN[now.getDay()]}`;
  const mini = $("cal-mini");
  if (mini) {
    mini.textContent = `${now.getMonth() + 1}月${now.getDate()}日 周${WEEK_CN[now.getDay()]} · 农历${l.monthCn}${l.dayCn}`;
  }
  updateFestivals();
  updateOffWork();
}

// ---------- 天气 ----------
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
    $("w-desc").textContent = `${WMO_DESC[w.code] ?? "未知"}`;
    $("w-city").textContent = `📍 ${w.city}`;
    $("w-humid").textContent = `💧 湿度 ${Math.round(w.humidity)}%`;
    $("w-feels").textContent = `🌡 体感 ${Math.round(w.feels)}°`;
    $("w-wind").textContent = `🌬 风速 ${Math.round(w.wind)} km/h`;
  } catch {
    $("w-desc").textContent = "天气获取失败";
  }
}

// ---------- 系统状态（环形） ----------
const RING_C = 314.16;

function setRing(id: string, pct: number) {
  const el = $(id) as unknown as SVGCircleElement;
  el.style.strokeDashoffset = String(RING_C * (1 - Math.min(Math.max(pct, 0), 1)));
}

function gb(bytes: number): string {
  return (bytes / 1024 ** 3).toFixed(1);
}

async function refreshSys() {
  try {
    const s = await invoke<SysInfo>("get_sysinfo");
    const memPct = s.mem_total ? (s.mem_used / s.mem_total) * 100 : 0;
    const diskPct = s.disk_total ? (s.disk_used / s.disk_total) * 100 : 0;
    $("val-cpu").textContent = `${Math.round(s.cpu)}%`;
    $("val-mem-pct").textContent = `${Math.round(memPct)}%`;
    $("val-disk-pct").textContent = `${Math.round(diskPct)}%`;
    $("val-mem").textContent = `${gb(s.mem_used)}/${gb(s.mem_total)}G`;
    $("val-disk").textContent = `${gb(s.disk_used)}/${gb(s.disk_total)}G`;
    setRing("ring-cpu", s.cpu / 100);
    setRing("ring-mem", memPct / 100);
    setRing("ring-disk", diskPct / 100);
  } catch {
    /* ignore */
  }
}

async function refreshTemps() {
  try {
    const t = await invoke<Temps>("get_temps");
    $("val-cputemp").textContent = t.cpu != null ? `${t.cpu.toFixed(1)}°C` : "";
    $("val-disktemp").textContent = t.disk != null ? `${t.disk.toFixed(1)}°C` : "";
  } catch {
    /* ignore */
  }
}

// ---------- 日历 ----------
let calYear = new Date().getFullYear();
let calMonth = new Date().getMonth();

const SOLAR_FEST: [number, number, string][] = [
  [1, 1, "元旦"],
  [2, 14, "情人节"],
  [3, 8, "妇女节"],
  [3, 12, "植树节"],
  [5, 1, "劳动节"],
  [5, 4, "青年节"],
  [6, 1, "儿童节"],
  [7, 1, "建党节"],
  [8, 1, "建军节"],
  [9, 10, "教师节"],
  [10, 1, "国庆节"],
  [12, 25, "圣诞节"],
];

const LUNAR_FEST: [number, number, string][] = [
  [1, 1, "春节"],
  [1, 15, "元宵节"],
  [2, 2, "龙抬头"],
  [5, 5, "端午节"],
  [7, 7, "七夕"],
  [8, 15, "中秋节"],
  [9, 9, "重阳节"],
  [12, 8, "腊八节"],
];

function lunar2solar(y: number, m: number, d: number): Date {
  let offset = 0;
  for (let i = 1900; i < y; i++) offset += lunarYearDays(i);
  const leap = lunarLeapMonth(y);
  for (let i = 1; i < m; i++) offset += lunarMonthDays(y, i);
  if (leap > 0 && m > leap) offset += lunarLeapDays(y);
  offset += d - 1;
  return new Date(Date.UTC(1900, 0, 31) + offset * 86400000);
}

function dayFestival(y: number, m: number, d: number, l: { month: number; day: number }): string {
  for (const [fm, fd, name] of SOLAR_FEST) {
    if (fm === m && fd === d) return name;
  }
  for (const [fm, fd, name] of LUNAR_FEST) {
    if (fm === l.month && fd === l.day) return name;
  }
  return "";
}

function fmtRemain(days: number): string {
  if (days === 0) return "今天";
  return `还有 ${days} 天`;
}

function updateFestivals() {
  const now = new Date();
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
  const list: { name: string; t: number }[] = [];
  for (const [m, dd, name] of SOLAR_FEST) {
    let t = new Date(now.getFullYear(), m - 1, dd).getTime();
    if (t < today) t = new Date(now.getFullYear() + 1, m - 1, dd).getTime();
    list.push({ name, t });
  }
  for (const [m, dd, name] of LUNAR_FEST) {
    let t = lunar2solar(now.getFullYear(), m, dd).getTime();
    if (t < today) t = lunar2solar(now.getFullYear() + 1, m, dd).getTime();
    list.push({ name, t });
  }
  list.sort((a, b) => a.t - b.t);
  const next3 = list.slice(0, 3);
  const el = $("cal-fest");
  if (next3.length) {
    el.innerHTML = next3
      .map((x) => {
        const days = Math.round((x.t - today) / 86400000);
        return days === 0 ? `<b>🎉 ${x.name} 今天</b>` : `${x.name} ${fmtRemain(days)}`;
      })
      .join("<br>");
  } else {
    el.textContent = "";
  }
}

function renderCalendar() {
  $("cal-title").textContent = `${calYear}年${calMonth + 1}月`;
  const grid = $("cal-grid");
  grid.innerHTML = "";
  ["一", "二", "三", "四", "五", "六", "日"].forEach((d, i) => {
    const el = document.createElement("div");
    el.className = "cal-dow";
    if (i >= 5) el.classList.add("weekend");
    el.textContent = d;
    grid.appendChild(el);
  });
  const first = new Date(calYear, calMonth, 1);
  const offset = (first.getDay() + 6) % 7;
  const days = new Date(calYear, calMonth + 1, 0).getDate();
  const prevDays = new Date(calYear, calMonth, 0).getDate();
  const today = new Date();
  const mkDay = (d: number, out: boolean) => {
    const el = document.createElement("div");
    el.className = `cal-day${out ? " out" : ""}`;
    const dow = new Date(calYear, calMonth, d).getDay();
    if (dow === 0 || dow === 6) el.classList.add("weekend");
    const l = solar2lunar(calYear, calMonth, d);
    const fest = dayFestival(calYear, calMonth + 1, d, l);
    const tag = fest || l.dayCn;
    const num = document.createElement("span");
    num.className = "cal-num";
    num.textContent = String(d);
    const tg = document.createElement("span");
    tg.className = `cal-tag${fest ? " fest" : ""}`;
    tg.textContent = tag;
    el.append(num, tg);
    if (d === today.getDate() && calMonth === today.getMonth() && calYear === today.getFullYear()) {
      el.classList.add("today");
    }
    grid.appendChild(el);
  };
  for (let i = offset - 1; i >= 0; i--) {
    mkDay(prevDays - i, true);
  }
  for (let d = 1; d <= days; d++) {
    mkDay(d, false);
  }
  const total = offset + days;
  for (let i = 1; total + i <= 42; i++) {
    mkDay(i, true);
  }
  updateFestivals();
}

// ---------- 待办 ----------
function loadTodos(): Todo[] {
  try {
    const raw = localStorage.getItem("todos");
    if (raw) return JSON.parse(raw);
  } catch {
    /* ignore */
  }
  const defaults: Todo[] = [
    { text: "回帆首页 UI 设计", time: "10:00", done: true },
    { text: "测试微信 MCP 功能", time: "14:00", done: true },
    { text: "检查 ECS 服务器状态", time: "16:00", done: false },
    { text: "整理项目文档", time: "18:00", done: false },
    { text: "与团队同步进度", time: "20:00", done: false },
  ];
  saveTodos(defaults);
  return defaults;
}

function saveTodos(todos: Todo[]) {
  localStorage.setItem("todos", JSON.stringify(todos));
}

function renderTodos() {
  const todos = loadTodos();
  const list = $("todo-list");
  list.innerHTML = "";
  let done = 0;
  todos.forEach((t, idx) => {
    if (t.done) done++;
    const item = document.createElement("div");
    item.className = `todo-item${t.done ? " done" : ""}`;
    const check = document.createElement("span");
    check.className = "todo-check";
    check.textContent = "✓";
    check.title = t.done ? "标记未完成" : "标记完成";
    check.addEventListener("click", () => {
      const all = loadTodos();
      all[idx].done = !all[idx].done;
      saveTodos(all);
      renderTodos();
    });
    const text = document.createElement("span");
    text.className = "todo-text";
    text.textContent = t.text;
    const time = document.createElement("span");
    time.className = `todo-time${idx === 0 ? " hot" : ""}`;
    time.textContent = t.time;
    const del = document.createElement("span");
    del.className = "todo-del";
    del.textContent = "✕";
    del.title = "删除";
    del.addEventListener("click", () => {
      const all = loadTodos();
      all.splice(idx, 1);
      saveTodos(all);
      renderTodos();
    });
    item.append(check, text, time, del);
    list.appendChild(item);
  });
  $("todo-count").textContent = `已完成 ${done}/${todos.length}`;
  $("todo-fill").style.width = todos.length ? `${(done / todos.length) * 100}%` : "0%";
}

function addTodoInput() {
  const list = $("todo-list");
  if (list.querySelector(".todo-input-row")) return;
  const row = document.createElement("div");
  row.className = "todo-input-row";
  const input = document.createElement("input");
  input.placeholder = "输入待办内容，回车确认";
  input.autofocus = true;
  const btn = document.createElement("button");
  btn.className = "text-btn";
  btn.textContent = "确定";
  const commit = () => {
    const v = input.value.trim();
    if (v) {
      const all = loadTodos();
      const now = new Date();
      all.push({ text: v, time: `${String(now.getHours()).padStart(2, "0")}:${String(now.getMinutes()).padStart(2, "0")}`, done: false });
      saveTodos(all);
      renderTodos();
    } else {
      row.remove();
    }
  };
  btn.addEventListener("click", commit);
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter") commit();
    if (e.key === "Escape") row.remove();
  });
  row.append(input, btn);
  list.prepend(row);
  input.focus();
}

// ---------- 快捷启动 ----------
const FOLDERS: { name: string; shell: string; emoji: string }[] = [
  { name: "桌面", shell: "shell:Desktop", emoji: "🖥" },
  { name: "文档", shell: "shell:Personal", emoji: "📄" },
  { name: "下载", shell: "shell:Downloads", emoji: "⬇️" },
  { name: "图片", shell: "shell:My Pictures", emoji: "🖼" },
  { name: "音乐", shell: "shell:My Music", emoji: "🎵" },
  { name: "视频", shell: "shell:My Video", emoji: "🎬" },
];

const TOOLS: { id: string; name: string; emoji: string }[] = [
  { id: "explorer", name: "文件管理器", emoji: "🗂" },
  { id: "notepad", name: "记事本", emoji: "📝" },
  { id: "cmd", name: "命令提示符", emoji: "⌨" },
  { id: "snipping", name: "截图工具", emoji: "✂" },
  { id: "calc", name: "计算器", emoji: "🧮" },
  { id: "recycle", name: "回收站", emoji: "🗑" },
  { id: "control", name: "控制面板", emoji: "🖥" },
  { id: "taskmgr", name: "任务管理器", emoji: "📊" },
];

function tile(name: string, emoji: string, onClick: () => void, extra = "", onDelete?: () => void) {
  const t = document.createElement("button");
  t.className = `app-tile${extra ? ` ${extra}` : ""}`;
  t.title = name;
  t.innerHTML = `<span class="app-ico${extra ? ` ${extra}` : ""}">${emoji}</span><span class="app-name">${name}</span>`;
  t.addEventListener("click", onClick);
  if (onDelete) {
    const del = document.createElement("span");
    del.className = "app-del";
    del.textContent = "✕";
    del.title = "删除";
    del.addEventListener("click", (e) => {
      e.stopPropagation();
      onDelete();
    });
    t.appendChild(del);
  }
  return t;
}

async function getConfig(): Promise<Config> {
  try {
    return await invoke<Config>("get_config");
  } catch {
    return { city: "", lat: null, lon: null, ai_url: "", off_time: "18:00", apps: [] };
  }
}

async function persistConfig(cfg: Config) {
  await invoke("save_config", { cfg }).catch((e) => alert(e));
  refreshApps();
}

async function addApp() {
  const picked = await open({
    multiple: false,
    title: "选择要添加的应用程序",
    filters: [{ name: "程序", extensions: ["exe", "lnk", "bat", "cmd"] }],
  });
  if (!picked) return;
  const p = Array.isArray(picked) ? picked[0] : picked;
  const name = p.split(/[\\/]/).pop()!.replace(/\.(exe|lnk|bat|cmd)$/i, "");
  const cfg = await getConfig();
  cfg.apps.push({ name, path: p, emoji: "📦", args: null });
  await persistConfig(cfg);
}

async function removeApp(name: string) {
  const cfg = await getConfig();
  const before = cfg.apps.length;
  cfg.apps = cfg.apps.filter((a) => a.name !== name);
  if (cfg.apps.length === before) return;
  await persistConfig(cfg);
}

// ---------- 应用选择面板 ----------
let pickerApps: AppItem[] = [];
let pickerLoaded = false;
let pickerAdded = new Set<string>();
let pickerFilter = "";

async function openPicker() {
  $("app-picker").classList.remove("hidden");
  ($("picker-search") as HTMLInputElement).value = "";
  pickerFilter = "";
  const cfg = await getConfig();
  pickerAdded = new Set(cfg.apps.map((a) => a.name));
  if (!pickerLoaded) {
    renderPicker("", true);
    try {
      pickerApps = await invoke<AppItem[]>("scan_apps");
    } catch {
      pickerApps = [];
    }
    pickerLoaded = true;
  }
  renderPicker("", false);
}

function renderPicker(filter: string, loading: boolean) {
  const list = $("picker-list");
  if (loading) {
    list.innerHTML = `<div class="picker-empty">正在扫描全盘应用…</div>`;
    return;
  }
  if (!pickerApps.length) {
    list.innerHTML = `<div class="picker-empty">没有扫描到应用</div>`;
    return;
  }
  const kw = filter.trim().toLowerCase();
  const items = pickerApps.filter((a) => !kw || a.name.toLowerCase().includes(kw));
  if (!items.length) {
    list.innerHTML = `<div class="picker-empty">没有找到匹配的应用</div>`;
    return;
  }
  list.innerHTML = "";
  for (const app of items) {
    const added = pickerAdded.has(app.name);
    const el = document.createElement("div");
    el.className = `picker-item${added ? " added" : ""}`;
    el.title = app.path;
    el.innerHTML = `<span class="picker-ico">${app.emoji || "📦"}</span><span class="picker-name">${app.name}</span><span class="picker-state">${added ? "已添加 ✓" : "+ 添加"}</span>`;
    el.addEventListener("click", () => {
      void (async () => {
        const cfg = await getConfig();
        if (pickerAdded.has(app.name)) {
          cfg.apps = cfg.apps.filter((x) => x.name !== app.name);
          pickerAdded.delete(app.name);
        } else {
          if (!cfg.apps.some((x) => x.name === app.name)) {
            cfg.apps.push({ name: app.name, path: app.path, emoji: app.emoji || "📦", args: null });
          }
          pickerAdded.add(app.name);
        }
        await persistConfig(cfg);
        renderPicker(pickerFilter, false);
      })();
    });
    list.appendChild(el);
  }
}

function initPicker() {
  $("picker-close").addEventListener("click", () => $("app-picker").classList.add("hidden"));
  ($("picker-search") as HTMLInputElement).addEventListener("input", (e) => {
    pickerFilter = (e.target as HTMLInputElement).value;
    renderPicker(pickerFilter, false);
  });
  $("picker-browse").addEventListener("click", () => {
    $("app-picker").classList.add("hidden");
    addApp();
  });
  $("app-picker").addEventListener("click", (e) => {
    if (e.target === e.currentTarget) {
      $("app-picker").classList.add("hidden");
    }
  });
}

async function refreshApps() {
  const appsGrid = $("apps-grid");
  appsGrid.innerHTML = "";
  let cfg: Config | null = null;
  try {
    cfg = await invoke<Config>("get_config");
  } catch {
    /* ignore */
  }
  let apps = cfg?.apps ?? [];
  const allEmpty = apps.length > 0 && apps.every((a) => !a.path);
  if (allEmpty) {
    const c = await getConfig();
    c.apps = [];
    await invoke("save_config", { cfg: c }).catch(() => {});
    apps = [];
  }
  for (const app of apps) {
    appsGrid.appendChild(
      tile(
        app.name,
        app.emoji || "📦",
        () => {
          if (!app.path) {
            alert(`「${app.name}」未配置路径，已打开配置文件，请填写 path 后保存`);
            invoke("open_config").catch((e) => alert(e));
            return;
          }
          invoke("launch_app", { path: app.path, args: app.args }).catch((e) => alert(e));
        },
        "",
        () => removeApp(app.name),
      ),
    );
  }
  appsGrid.appendChild(
    tile("添加应用", "+", () => openPicker(), "add"),
  );

  if (!apps.length && !localStorage.getItem("picker-shown")) {
    localStorage.setItem("picker-shown", "1");
    openPicker();
  }

  const foldersGrid = $("folders-grid");
  foldersGrid.innerHTML = "";
  for (const f of FOLDERS) {
    foldersGrid.appendChild(
      tile(f.name, f.emoji, () => {
        invoke("launch_app", { path: "explorer.exe", args: f.shell }).catch((e) => alert(e));
      }),
    );
  }
  foldersGrid.appendChild(
    tile("添加", "+", () => invoke("open_config").catch((e) => alert(e)), "add"),
  );
}

function renderTools() {
  const grid = $("tools-grid");
  grid.innerHTML = "";
  for (const t of TOOLS) {
    grid.appendChild(
      tile(t.name, t.emoji, () => {
        invoke("launch_tool", { id: t.id }).catch((e) => alert(e));
      }),
    );
  }
}

// ---------- 设置 ----------
function initSettings() {
  let win: TauriWindow | null = null;
  try {
    win = getCurrentWindow();
  } catch {
    /* 非 Tauri 环境 */
  }
  const sliderOp = $("slider-opacity") as HTMLInputElement;
  const sliderVol = $("slider-volume") as HTMLInputElement;
  const sliderEye = $("slider-eyecare") as HTMLInputElement;
  const toggleEye = $("toggle-eyecare") as HTMLInputElement;

  const savedOp = Number(localStorage.getItem("opacity") ?? "100");
  sliderOp.value = String(savedOp);
  $("val-opacity").textContent = `${savedOp}%`;
  document.documentElement.style.setProperty("--bg-a", String(savedOp / 100));

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

  sliderOp.addEventListener("input", () => {
    const pct = Number(sliderOp.value);
    $("val-opacity").textContent = `${pct}%`;
    localStorage.setItem("opacity", String(pct));
    document.documentElement.style.setProperty("--bg-a", String(pct / 100));
  });

  sliderVol.addEventListener("input", () => {
    const pct = Number(sliderVol.value);
    $("val-volume").textContent = `${pct}%`;
    invoke("set_volume", { level: pct / 100 }).catch((e) =>
      toast(`音量调节失败：${e}`),
    );
  });

  toggleEye.addEventListener("change", () => {
    const on = toggleEye.checked;
    sliderEye.disabled = !on;
    localStorage.setItem("eyecare", on ? "1" : "0");
    invoke("set_eye_care", { enabled: on, intensity: Number(sliderEye.value) / 100 }).catch(() => {});
  });

  sliderEye.addEventListener("input", () => {
    const pct = Number(sliderEye.value);
    $("val-eyecare").textContent = `${pct}%`;
    localStorage.setItem("eyecare-int", String(pct));
    if (toggleEye.checked) {
      invoke("set_eye_care", { enabled: true, intensity: pct / 100 }).catch(() => {});
    }
  });

  let configOpened = false;
  $("btn-settings").addEventListener("click", () => {
    invoke("open_config").catch((e) => alert(e));
    configOpened = true;
  });

  if (win) {
    win.onFocusChanged(({ payload }) => {
      if (payload && configOpened) {
        configOpened = false;
        invoke("reload_config")
          .then(() => refreshApps())
          .catch(() => {});
      }
    });
  }

  $("btn-pin").addEventListener("click", () => {
    const pinned = localStorage.getItem("pinned") !== "0";
    const next = !pinned;
    localStorage.setItem("pinned", next ? "1" : "0");
    win?.setAlwaysOnTop(next);
    $("btn-pin").style.opacity = next ? "1" : "0.4";
  });
  if (localStorage.getItem("pinned") === "0") {
    win?.setAlwaysOnTop(false);
    $("btn-pin").style.opacity = "0.4";
  }
  $("btn-min").addEventListener("click", () => win?.minimize());
  $("btn-close").addEventListener("click", () => {
    invoke("restore_eye_care")
      .catch(() => {})
      .finally(() => win?.hide());
  });
}

// ---------- AI 对话 ----------
let toastTimer: number | undefined;

function toast(msg: string) {
  const t = $("toast");
  t.textContent = msg;
  t.classList.remove("hidden");
  clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => t.classList.add("hidden"), 4500);
}

function initAi() {
  const input = $("ai-input") as HTMLTextAreaElement;
  const model = $("ai-model") as HTMLSelectElement;
  const savedModel = localStorage.getItem("ai-model");
  if (savedModel && Array.from(model.options).some((o) => o.value === savedModel)) {
    model.value = savedModel;
  }
  model.addEventListener("change", () => localStorage.setItem("ai-model", model.value));
  const send = () => {
    const q = input.value.trim();
    if (!q) return;
    writeText(q).catch(() => {});
    invoke("open_ai_chat", { model: model.value, question: q }).catch((e) => alert(e));
    toast(`已打开${model.value}并复制问题，如页面未自动填入请按 Ctrl+V`);
  };
  $("ai-send").addEventListener("click", send);
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  });
}

// ---------- 习惯打卡（喝水/抽烟，每日自动重置） ----------
const HABIT_KEY = "habits";

function loadHabits(): { date: string; water: number; smoke: number } {
  const today = new Date();
  const todayStr = `${today.getFullYear()}-${today.getMonth() + 1}-${today.getDate()}`;
  try {
    const raw = localStorage.getItem(HABIT_KEY);
    if (raw) {
      const h = JSON.parse(raw);
      if (h.date === todayStr) return h;
    }
  } catch {
    /* ignore */
  }
  return { date: todayStr, water: 0, smoke: 0 };
}

function saveHabits(h: { date: string; water: number; smoke: number }) {
  localStorage.setItem(HABIT_KEY, JSON.stringify(h));
}

function renderHabits() {
  const h = loadHabits();
  $("cnt-water").textContent = String(h.water);
  $("cnt-smoke").textContent = String(h.smoke);
}

function initHabits() {
  renderHabits();
  const bump = (key: "water" | "smoke") => {
    const h = loadHabits();
    h[key] += 1;
    saveHabits(h);
    renderHabits();
  };
  const reset = (key: "water" | "smoke") => {
    const h = loadHabits();
    h[key] = 0;
    saveHabits(h);
    renderHabits();
  };
  $("btn-water").addEventListener("click", () => bump("water"));
  $("btn-smoke").addEventListener("click", () => bump("smoke"));
  $("btn-water").addEventListener("dblclick", () => reset("water"));
  $("btn-smoke").addEventListener("dblclick", () => reset("smoke"));
}

// ---------- 下班倒计时 ----------
let offTime = "18:00";

async function loadOffTime() {
  try {
    const cfg = await invoke<Config>("get_config");
    if (cfg.off_time) offTime = cfg.off_time;
  } catch {
    /* ignore */
  }
}

function updateOffWork() {
  const el = $("off-work");
  if (!el) return;
  const now = new Date();
  const [h, m] = offTime.split(":").map((x) => Number(x) || 0);
  const off = new Date(now.getFullYear(), now.getMonth(), now.getDate(), h, m, 0);
  if (now >= off) {
    el.textContent = "🎉 已下班";
    return;
  }
  const diff = Math.floor((off.getTime() - now.getTime()) / 1000);
  const hh = String(Math.floor(diff / 3600)).padStart(2, "0");
  const mm = String(Math.floor((diff % 3600) / 60)).padStart(2, "0");
  const ss = String(diff % 60).padStart(2, "0");
  el.textContent = `⏰ 距下班 ${hh}:${mm}:${ss}`;
}

// ---------- 缩服 ----------
let collapsed = false;

const MIN_W = 490;
const MIN_H = 225;

function initCollapse() {
  let win: TauriWindow | null = null;
  try {
    win = getCurrentWindow();
  } catch {
    /* 非 Tauri 环境 */
  }
  $("btn-collapse").addEventListener("click", () => {
    collapsed = !collapsed;
    document.body.classList.toggle("collapsed", collapsed);
    $("btn-collapse").textContent = collapsed ? "↗" : "▬";
    $("mini-bar").classList.toggle("hidden", !collapsed);
    if (win) {
      if (collapsed) {
        win.setMinSize(new LogicalSize(360, 110)).catch(() => {});
      } else {
        win.setMinSize(new LogicalSize(MIN_W, MIN_H)).catch(() => {});
      }
    }
    if (collapsed) {
      invoke("set_window_size", { width: 360, height: 110 }).catch(() => {});
    } else {
      let w = DEFAULT_W;
      let h = DEFAULT_H;
      try {
        const s = JSON.parse(localStorage.getItem(WIN_STATE_KEY) || "{}");
        if (s.w && s.h) {
          w = s.w;
          h = s.h;
        }
      } catch {
        /* ignore */
      }
      invoke("set_window_size", { width: w, height: h }).catch(() => {});
    }
  });
}

// ---------- 窗口初始化 ----------
const DEFAULT_W = 1200;
const DEFAULT_H = 760;
const WIN_STATE_KEY = "win-state";

function saveWinState(patch: Record<string, number>) {
  if (collapsed) return;
  let cur: Record<string, number> = {};
  try {
    cur = JSON.parse(localStorage.getItem(WIN_STATE_KEY) || "{}");
  } catch {
    /* ignore */
  }
  Object.assign(cur, patch);
  localStorage.setItem(WIN_STATE_KEY, JSON.stringify(cur));
}

function initWindow() {
  try {
    const win = getCurrentWindow();
    win.setMinSize(new LogicalSize(MIN_W, MIN_H)).catch(() => {});
    let restored = false;
    try {
      const s = JSON.parse(localStorage.getItem(WIN_STATE_KEY) || "null");
      if (s && Number.isFinite(s.w) && Number.isFinite(s.h)) {
        const w = Math.min(Math.max(s.w, 400), 4000);
        const h = Math.min(Math.max(s.h, 200), 2000);
        win.setSize(new PhysicalSize(w, h)).catch(() => {});
        if (
          Number.isFinite(s.x) &&
          Number.isFinite(s.y) &&
          s.x > -4000 &&
          s.x < 8000 &&
          s.y > -4000 &&
          s.y < 8000
        ) {
          win.setPosition(new PhysicalPosition(s.x, s.y)).catch(() => {});
        }
        restored = true;
      }
    } catch {
      /* ignore */
    }
    if (!restored) {
      win.setSize(new LogicalSize(DEFAULT_W, DEFAULT_H)).catch(() => {});
    }
    win.onResized(({ payload }) => {
      if (payload.width < 300 || payload.height < 150) return;
      saveWinState({ w: payload.width, h: payload.height });
    });
    win.onMoved(({ payload }) => {
      if (payload.x < -4000 || payload.x > 8000 || payload.y < -4000 || payload.y > 8000) return;
      saveWinState({ x: payload.x, y: payload.y });
    });
  } catch {
    /* 非 Tauri 环境 */
  }
}

// ---------- 粒子背景 ----------
function initParticles() {
  const canvas = $("particles") as HTMLCanvasElement;
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  const resize = () => {
    canvas.width = canvas.clientWidth * dpr;
    canvas.height = canvas.clientHeight * dpr;
  };
  resize();
  window.addEventListener("resize", resize);
  const COLORS = ["120,170,255", "190,120,255", "255,150,120", "80,230,210", "255,110,220", "255,220,120"];
  const N = Math.max(40, Math.min(100, Math.floor(canvas.clientWidth / 20)));
  const ps = Array.from({ length: N }, () => ({
    x: Math.random(),
    y: Math.random(),
    r: Math.random() * 4.5 + 1.6,
    vx: (Math.random() - 0.5) * 0.0003,
    vy: -(Math.random() * 0.0004 + 0.00008),
    a: Math.random() * 0.45 + 0.55,
    c: COLORS[Math.floor(Math.random() * COLORS.length)],
    tw: Math.random() * Math.PI * 2,
  }));
  let t = 0;
  const tick = () => {
    t += 0.016;
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    for (const p of ps) {
      p.x += p.vx;
      p.y += p.vy;
      if (p.y < -0.03) {
        p.y = 1.03;
        p.x = Math.random();
      }
      if (p.x < -0.03) p.x = 1.03;
      if (p.x > 1.03) p.x = -0.03;
      const alpha = p.a * (0.6 + 0.4 * Math.sin(t * 1.6 + p.tw));
      const grad = ctx.createRadialGradient(p.x * canvas.width, p.y * canvas.height, 0, p.x * canvas.width, p.y * canvas.height, p.r * 3 * dpr);
      grad.addColorStop(0, `rgba(${p.c},${alpha.toFixed(3)})`);
      grad.addColorStop(1, `rgba(${p.c},0)`);
      ctx.fillStyle = grad;
      ctx.beginPath();
      ctx.arc(p.x * canvas.width, p.y * canvas.height, p.r * 3 * dpr, 0, Math.PI * 2);
      ctx.fill();
    }
    requestAnimationFrame(tick);
  };
  tick();
}

// ---------- init ----------
function init() {
  fmtClock();
  renderCalendar();
  renderTodos();
  renderTools();
  refreshApps();
  refreshSys();
  refreshTemps();
  refreshWeather();
  initSettings();
  initCollapse();
  initAi();
  initWindow();
  initParticles();
  initPicker();
  initHabits();
  loadOffTime();
  setInterval(fmtClock, 1000);
  setInterval(refreshSys, 2000);
  setInterval(refreshTemps, 5000);
  setInterval(refreshWeather, 5 * 60 * 1000);
  updateOffWork();

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
  $("btn-todo-add").addEventListener("click", addTodoInput);
  $("tab-apps").addEventListener("click", () => {
    $("tab-apps").classList.add("active");
    $("tab-folders").classList.remove("active");
    $("apps-grid").classList.remove("hidden");
    $("folders-grid").classList.add("hidden");
  });
  $("tab-folders").addEventListener("click", () => {
    $("tab-folders").classList.add("active");
    $("tab-apps").classList.remove("active");
    $("folders-grid").classList.remove("hidden");
    $("apps-grid").classList.add("hidden");
  });
}

init();
