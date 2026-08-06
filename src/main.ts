import "./styles.css";
import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
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
  ai_keys: Record<string, string>;
  ai_models: Record<string, string>;
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

let lastCpuTemp: number | null = null;

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
    const cpuTemp = lastCpuTemp;
    const dims: { v: number; name: string }[] = [
      { v: s.cpu, name: "CPU" },
      { v: memPct, name: "内存" },
      { v: diskPct, name: "磁盘" },
      { v: cpuTemp ?? 0, name: "温度" },
    ];
    const thresholds = [
      [20, 40, 55, 45],
      [45, 60, 70, 55],
      [65, 75, 82, 65],
      [80, 85, 90, 75],
      [92, 93, 96, 85],
    ];
    let level = 0;
    for (let i = 0; i < dims.length; i++) {
      let lv = 0;
      for (let t = 0; t < thresholds.length; t++) {
        if (dims[i].v >= thresholds[t][i]) lv = t + 1;
      }
      if (lv > level) level = lv;
    }
    const LEVELS = [
      { label: "运行优秀", cls: "good", dot: "🟢" },
      { label: "运行良好", cls: "good", dot: "🟢" },
      { label: "负载适中", cls: "mid", dot: "🔵" },
      { label: "负载偏高", cls: "warn", dot: "🟡" },
      { label: "负载过高", cls: "bad", dot: "🔴" },
      { label: "状态异常", cls: "bad", dot: "🚨" },
    ];
    const worst = dims.reduce((a, b) => (b.v > a.v ? b : a));
    const lv = LEVELS[level];
    const detail = dims
      .map((d, i) => `${d.name} ${i === 3 ? `${d.v.toFixed(0)}°C` : `${Math.round(d.v)}%`}`)
      .join("  |  ");
    const badge = $("status-badge");
    badge.textContent = `${lv.dot} ${lv.label}${level >= 3 ? ` · ${worst.name} ${worst.v >= 90 && worst.name !== "温度" ? "告急" : "偏高"}` : ""}`;
    badge.className = `status-good ${lv.cls}`;
    badge.title = `系统状态：${detail}`;
  } catch {
    /* ignore */
  }
}

async function refreshTemps() {
  try {
    const t = await invoke<Temps>("get_temps");
    lastCpuTemp = t.cpu;
    $("val-cputemp").textContent = t.cpu != null ? `${t.cpu.toFixed(1)}°C` : "";
    $("val-disktemp").textContent = t.disk != null ? `${t.disk.toFixed(1)}°C` : "";
  } catch {
    /* ignore */
  }
}

// ---------- 一键清理 ----------
function initClean() {
  const btnMem = $("btn-clean-mem") as HTMLButtonElement;
  const btnJunk = $("btn-clean-junk") as HTMLButtonElement;
  btnMem.addEventListener("click", async () => {
    btnMem.disabled = true;
    btnMem.textContent = "清理中…";
    try {
      const r = await invoke<{ freed_mb: number; processes: number }>("clean_memory");
      alert(`内存清理完成：释放约 ${r.freed_mb} MB（刷新 ${r.processes} 个进程的工作集）\n未关闭任何程序。`);
    } catch (e) {
      alert(`清理失败：${e}`);
    }
    btnMem.disabled = false;
    btnMem.textContent = "🧹 清理内存";
  });
  btnJunk.addEventListener("click", async () => {
    if (
      !confirm(
        "将清理临时文件夹（%TEMP% / Windows\\Temp）中的垃圾文件。\n正在使用的文件会自动跳过，不会删除你的任何正常文件。\n确定继续？",
      )
    ) {
      return;
    }
    btnJunk.disabled = true;
    btnJunk.textContent = "清理中…";
    try {
      const r = await invoke<{ files: number; bytes: number }>("clean_junk");
      alert(`垃圾清理完成：删除 ${r.files} 个文件/文件夹，释放约 ${(r.bytes / 1024 / 1024).toFixed(1)} MB`);
    } catch (e) {
      alert(`清理失败：${e}`);
    }
    btnJunk.disabled = false;
    btnJunk.textContent = "🗑 清理垃圾";
  });
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
  const y = now.getFullYear();
  const list: { name: string; t: number; range?: string }[] = [];
  const midAutumn = lunar2solar(y, 8, 15);
  list.push({ name: "中秋节", t: midAutumn.getTime() });
  list.push({ name: "国庆节", t: new Date(y, 9, 1).getTime(), range: "10月1日-10月7日" });
  list.push({ name: "元旦", t: new Date(y, 0, 1).getTime() });
  const el = $("cal-fest");
  el.innerHTML = list
    .map((x, i) => {
      const d = new Date(x.t);
      const days = Math.round((x.t - today) / 86400000);
      const dot = x.name === "元旦" ? "fest-dot green" : "fest-dot";
      const when = days >= 0 ? `（还有 ${days} 天）` : "（已过）";
      const dateStr = x.range ?? `${d.getMonth() + 1}月${d.getDate()}日`;
      return `<div style="font-size:${Math.max(8, 10 - i)}px"><span class="${dot}"></span>${x.name} ${dateStr} ${when}</div>`;
    })
    .join("");
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
    const edit = document.createElement("span");
    edit.className = "todo-del todo-edit";
    edit.textContent = "✎";
    edit.title = "编辑";
    edit.addEventListener("click", () => {
      const all = loadTodos();
      const v = prompt("修改待办内容：", all[idx].text);
      if (v === null) return;
      const t = v.trim();
      if (!t) return;
      all[idx].text = t;
      saveTodos(all);
      renderTodos();
    });
    item.append(check, text, time, edit, del);
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
    return { city: "", lat: null, lon: null, ai_url: "", off_time: "18:00", apps: [], ai_keys: {}, ai_models: {} };
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

let suppressClick = false;

function clearDragOver() {
  document.querySelectorAll(".app-tile.drag-over").forEach((el) => el.classList.remove("drag-over"));
}

function makeDraggable(t: HTMLElement, idx: number) {
  t.dataset.idx = String(idx);
  t.addEventListener("pointerdown", (e) => {
    if (e.button !== 0) return;
    const downX = e.clientX;
    const downY = e.clientY;
    let dragging = false;
    const onMove = (ev: PointerEvent) => {
      if (!dragging) {
        if (Math.hypot(ev.clientX - downX, ev.clientY - downY) < 5) return;
        dragging = true;
        suppressClick = true;
        t.classList.add("drag-source");
      }
      const el = document.elementFromPoint(ev.clientX, ev.clientY);
      const target = el ? el.closest<HTMLElement>(".app-tile") : null;
      clearDragOver();
      if (target && target !== t) target.classList.add("drag-over");
    };
    const onUp = (ev: PointerEvent) => {
      document.removeEventListener("pointermove", onMove);
      document.removeEventListener("pointerup", onUp);
      document.removeEventListener("pointercancel", onUp);
      if (!dragging) return;
      const el = document.elementFromPoint(ev.clientX, ev.clientY);
      const target = el ? el.closest<HTMLElement>(".app-tile") : null;
      t.classList.remove("drag-source");
      clearDragOver();
      if (target && target !== t) {
        void (async () => {
          const c = await getConfig();
          if (idx < 0 || idx >= c.apps.length) return;
          const [moved] = c.apps.splice(idx, 1);
          if (target.classList.contains("add")) {
            c.apps.push(moved);
          } else {
            const to = Number(target.dataset.idx);
            if (Number.isNaN(to)) return;
            c.apps.splice(idx < to ? to - 1 : to, 0, moved);
          }
          await persistConfig(c);
        })();
      }
    };
    document.addEventListener("pointermove", onMove);
    document.addEventListener("pointerup", onUp);
    document.addEventListener("pointercancel", onUp);
  });
  t.addEventListener(
    "click",
    (e) => {
      if (suppressClick) {
        e.preventDefault();
        e.stopPropagation();
        suppressClick = false;
      }
    },
    true,
  );
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
  for (const [i, app] of apps.entries()) {
    const t = tile(
      app.name,
      app.emoji || "📦",
      () => {
        if (!app.path) {
          alert(`「${app.name}」未配置路径，已打开配置文件，请填写 path 后保存`);
          invoke("open_config").catch((e) => alert(e));
          return;
        }
        invoke("launch_or_activate", { path: app.path, args: app.args }).catch((e) => alert(e));
      },
      "",
      () => removeApp(app.name),
    );
    makeDraggable(t, i);
    appsGrid.appendChild(t);
    if (app.path) {
      invoke<string>("get_app_icon", { path: app.path, name: app.name })
        .then((p) => {
          const ico = t.querySelector(".app-ico") as HTMLElement;
          if (ico) {
            ico.innerHTML = `<img class="app-ico-img" src="${convertFileSrc(p)}" draggable="false" alt="" />`;
          }
        })
        .catch(() => {});
    }
  }
  const addTile = tile("添加应用", "+", () => openPicker(), "add");
  appsGrid.appendChild(addTile);

  if (!apps.length && !localStorage.getItem("picker-shown")) {
    localStorage.setItem("picker-shown", "1");
    openPicker();
  }
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
  $("btn-config").addEventListener("click", () => {
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
  $("btn-max").addEventListener("click", () => {
    if (!win) return;
    win.isMaximized()
      .then((m) => (m ? win!.unmaximize() : win!.maximize()))
      .catch(() => {});
  });
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

// ---------- AI API 直答 ----------
import { Channel } from "@tauri-apps/api/core";

const AI_GUIDES: Record<
  string,
  { url: string; steps: string; defaultModel: string; keyHint: string; models: { name: string; note: string }[] }
> = {
  千问: {
    url: "https://bailian.console.aliyun.com/#/api-key",
    steps:
      "1. 打开阿里云百炼控制台（用支付宝扫码即可登录阿里云账号）\n2. 控制台左侧「API-KEY 管理」→ 点击「创建 API-KEY」\n3. 复制生成的 Key（形如 sk-xxxxxxxx）粘贴到下方\n4. qwen3.7-flash 价格最低（新用户常送免费额度）；qwen3.8-max 是旗舰且支持图片",
    defaultModel: "qwen3.7-flash",
    keyHint: "sk-",
    models: [
      { name: "qwen3.7-flash", note: "最便宜 · 快" },
      { name: "qwen3.7-plus", note: "均衡 · 推荐" },
      { name: "qwen3.8-max", note: "最新旗舰 · 支持图片" },
    ],
  },
  DeepSeek: {
    url: "https://platform.deepseek.com/api_keys",
    steps:
      "1. 打开 DeepSeek 开放平台，手机号注册并登录\n2. 左侧菜单「API Keys」→ 点击「创建 API Key」\n3. 复制 Key（形如 sk-xxxxxxxx）粘贴到下方\n4. deepseek-v4-flash 免费额度多、速度快；deepseek-v4-pro 是 V4 旗舰",
    defaultModel: "deepseek-v4-flash",
    keyHint: "sk-",
    models: [
      { name: "deepseek-v4-flash", note: "V4 · 免费额度 · 推荐" },
      { name: "deepseek-v4-pro", note: "V4 旗舰 · 更强" },
    ],
  },
  Kimi: {
    url: "https://platform.moonshot.cn/console/api-keys",
    steps:
      "1. 打开 Moonshot AI 开放平台，注册并登录\n2. 控制台左侧「API Keys」→ 「新建 API Key」\n3. 复制 Key（形如 sk-xxxxxxxx）粘贴到下方\n4. kimi-k3 是新一代旗舰，原生视觉 + 100 万上下文",
    defaultModel: "kimi-k3",
    keyHint: "sk-",
    models: [
      { name: "kimi-k3", note: "K3 旗舰 · 视觉 · 推荐" },
      { name: "kimi-k2.6", note: "K2.6 · 视觉+文本" },
      { name: "kimi-k2.7-code", note: "编程特化" },
    ],
  },
  智谱: {
    url: "https://open.bigmodel.cn/usercenter/apikeys",
    steps:
      "1. 打开智谱 AI 开放平台，手机号注册并登录\n2. 控制台「API 密钥」→ 「添加 API Key」\n3. 复制 Key（形如 xxxxxxxxxx.xxxx）粘贴到下方\n4. glm-4.7-flash 免费；glm-5.2 是最新旗舰；glm-4.6v-flash 免费且支持图片",
    defaultModel: "glm-4.7-flash",
    keyHint: "",
    models: [
      { name: "glm-4.7-flash", note: "免费 · 推荐" },
      { name: "glm-4.6v-flash", note: "免费 · 视觉" },
      { name: "glm-5-turbo", note: "轻量高速" },
      { name: "glm-5.2", note: "最新旗舰 · 1M 上下文" },
    ],
  },
  豆包: {
    url: "https://console.volcengine.com/ark",
    steps:
      "1. 打开火山引擎方舟控制台，注册/登录火山引擎\n2. 开通「方舟」服务 → 「API Key 管理」创建 API Key\n3. 「在线推理」→ 创建推理接入点，获得模型名（形如 ep-2024xxxx-xxxxx）\n4. 把 API Key 和 ep-xxx 模型名都填上（豆包必须填模型名，不支持直接选）",
    defaultModel: "",
    keyHint: "",
    models: [],
  },
};

type AiContent =
  | string
  | { type: "text"; text: string }
  | { type: "image_url"; image_url: { url: string } };

type AiMsg = { role: "user" | "assistant"; content: string | AiContent[] };

interface AiDeltaPayload {
  delta?: string;
  done?: boolean;
  error?: string | null;
}

type AiAttach = { kind: "image"; name: string; dataUrl: string } | { kind: "text"; name: string; content: string };

let aiMsgs: AiMsg[] = [];
let aiAttachments: AiAttach[] = [];
let aiStreaming = false;
let aiPendingSend: (() => void) | null = null;

function aiCurrentModel(): string {
  return ($("ai-model") as HTMLSelectElement).value;
}

function aiKeyFor(model: string): Promise<string> {
  return getConfig().then((c) => c.ai_keys?.[model] ?? "");
}

function openAiPanel() {
  $("ai-panel").classList.remove("hidden");
  ($("ai-panel-model") as HTMLElement).textContent = aiCurrentModel();
  fillPanelModelSel();
  const ta = $("ai-chat-input") as HTMLTextAreaElement;
  if (!ta.value && aiMsgs.length === 0) ta.focus();
  renderAiMsgs();
}

function fillPanelModelSel() {
  const modelName = aiCurrentModel();
  const sel = $("ai-panel-model-select") as HTMLSelectElement;
  const guide = AI_GUIDES[modelName];
  void getConfig().then((c) => {
    const saved = c.ai_models?.[modelName] ?? guide?.defaultModel ?? "";
    const opts = (guide?.models ?? []).map((m) => `<option value="${m.name}">${m.name}</option>`);
    if (saved && !guide?.models.some((m) => m.name === saved)) {
      opts.unshift(`<option value="${saved}">${saved} · 自定义</option>`);
    }
    sel.innerHTML = opts.join("");
    sel.value = saved;
    ($("ai-panel-apimodel") as HTMLElement).textContent = saved || "默认";
  });
}

function closeAiPanel() {
  if (aiStreaming) return;
  $("ai-panel").classList.add("hidden");
}

function clearAiChat() {
  if (aiStreaming) return;
  aiMsgs = [];
  aiAttachments = [];
  renderAiAttachments();
  renderAiMsgs();
  toast("对话已清空");
}

function appendAiMsg(role: AiMsg["role"], content: AiMsg["content"]): HTMLElement {
  const wrap = document.createElement("div");
  wrap.className = `ai-msg ${role}`;
  const roleEl = document.createElement("div");
  roleEl.className = "ai-role";
  roleEl.textContent = role === "user" ? "你" : aiCurrentModel();
  const bubble = document.createElement("div");
  bubble.className = "ai-bubble";
  if (role === "user" && Array.isArray(content)) {
    for (const c of content) {
      if (typeof c === "string") continue;
      if (c.type === "image_url") {
        const img = document.createElement("img");
        img.className = "ai-attach-thumb";
        img.src = c.image_url.url;
        bubble.appendChild(img);
      } else if (c.type === "text") {
        bubble.appendChild(document.createTextNode(c.text));
      }
    }
  } else {
    bubble.textContent = Array.isArray(content)
      ? content.map((c) => (typeof c === "string" ? c : c.type === "text" ? c.text : "")).join("\n")
      : content;
  }
  wrap.append(roleEl, bubble);
  $("ai-msgs").appendChild(wrap);
  const list = $("ai-msgs");
  list.scrollTop = list.scrollHeight;
  return wrap;
}

function renderAiMsgs() {
  const list = $("ai-msgs");
  list.innerHTML = "";
  if (!aiMsgs.length) {
    list.innerHTML = `<div class="ai-msg ai-msg-empty">输入问题开始对话，可发图片/文件（图片需模型支持）</div>`;
    return;
  }
  for (const m of aiMsgs) {
    appendAiMsg(m.role, m.content);
  }
}

function renderAiAttachments() {
  const box = $("ai-attach-list");
  box.innerHTML = "";
  for (const [i, a] of aiAttachments.entries()) {
    const chip = document.createElement("span");
    chip.className = "ai-attach-chip";
    if (a.kind === "image") {
      const img = document.createElement("img");
      img.src = a.dataUrl;
      chip.appendChild(img);
    } else {
      chip.textContent = "📄 ";
    }
    const span = document.createElement("span");
    span.textContent = a.name;
    chip.appendChild(span);
    const x = document.createElement("span");
    x.className = "ai-attach-x";
    x.textContent = "✕";
    x.addEventListener("click", () => {
      aiAttachments.splice(i, 1);
      renderAiAttachments();
    });
    chip.appendChild(x);
    box.appendChild(chip);
  }
}

function handlePasteImage(e: ClipboardEvent) {
  const items = e.clipboardData?.items;
  if (!items) return;
  for (const it of items) {
    if (it.type.startsWith("image/")) {
      const file = it.getAsFile();
      if (!file) continue;
      e.preventDefault();
      if (aiAttachments.length >= 3) {
        toast("最多 3 个附件");
        return;
      }
      const fr = new FileReader();
      fr.onload = () => {
        aiAttachments.push({
          kind: "image",
          name: `粘贴图片 ${aiAttachments.length + 1}.png`,
          dataUrl: String(fr.result),
        });
        renderAiAttachments();
      };
      fr.readAsDataURL(file);
      return;
    }
  }
}

async function pickAttachments() {
  if (aiStreaming) return;
  const picked = await open({
    multiple: true,
    title: "选择图片或文本文件（图片需模型支持多模态）",
    filters: [
      { name: "图片与文本", extensions: ["png", "jpg", "jpeg", "gif", "webp", "txt", "md", "js", "ts", "py", "json", "css", "html", "log"] },
    ],
  });
  if (!picked) return;
  const files = Array.isArray(picked) ? picked : [picked];
  for (const f of files) {
    if (aiAttachments.length >= 3) {
      toast("最多 3 个附件");
      break;
    }
    const ext = f.split(".").pop()?.toLowerCase() ?? "";
    const isImage = ["png", "jpg", "jpeg", "gif", "webp"].includes(ext);
    const name = f.split(/[\\/]/).pop() ?? "file";
    try {
      const blob = await fetch(convertFileSrc(f)).then((r) => r.blob());
      if (blob.size > 6 * 1024 * 1024) {
        toast(`${name} 超过 6MB，跳过`);
        continue;
      }
      if (isImage) {
        const dataUrl = await new Promise<string>((res, rej) => {
          const fr = new FileReader();
          fr.onload = () => res(String(fr.result));
          fr.onerror = () => rej(new Error("读取失败"));
          fr.readAsDataURL(blob);
        });
        aiAttachments.push({ kind: "image", name, dataUrl });
      } else {
        const content = await blob.text();
        aiAttachments.push({ kind: "text", name, content: content.slice(0, 20000) });
      }
    } catch {
      toast(`${name} 读取失败`);
    }
  }
  renderAiAttachments();
}

async function sendAiChat() {
  if (aiStreaming) return;
  const ta = $("ai-chat-input") as HTMLTextAreaElement;
  const text = ta.value.trim();
  if (!text && !aiAttachments.length) return;

  const content: AiContent[] = [];
  for (const a of aiAttachments) {
    if (a.kind === "image") {
      content.push({ type: "image_url", image_url: { url: a.dataUrl } });
    } else {
      content.push({ type: "text", text: `（附件 ${a.name} 内容）\n${a.content}` });
    }
  }
  if (text) content.push({ type: "text", text });
  aiMsgs.push({ role: "user", content });
  appendAiMsg("user", content);
  ta.value = "";
  aiAttachments = [];
  renderAiAttachments();

  const modelName = aiCurrentModel();
  const wrap = appendAiMsg("assistant", "");
  const bubble = wrap.querySelector(".ai-bubble") as HTMLElement;
  const textNode = document.createTextNode("");
  bubble.appendChild(textNode);
  wrap.classList.add("typing");
  aiStreaming = true;
  ($("ai-chat-send") as HTMLButtonElement).disabled = true;

  const ch = new Channel<AiDeltaPayload>();
  const pending = [...aiMsgs.slice(-20)];
  let scrollScheduled = false;
  const scheduleScroll = () => {
    if (scrollScheduled) return;
    scrollScheduled = true;
    requestAnimationFrame(() => {
      scrollScheduled = false;
      const list = $("ai-msgs");
      list.scrollTop = list.scrollHeight;
    });
  };
  ch.onmessage = (m) => {
    if (m.delta) {
      textNode.appendData(m.delta);
      scheduleScroll();
    }
    if (m.error) {
      wrap.classList.remove("typing");
      textNode.data = `⚠ ${m.error}`;
      aiStreaming = false;
      ($("ai-chat-send") as HTMLButtonElement).disabled = false;
    }
    if (m.done) {
      wrap.classList.remove("typing");
      const full = textNode.data ?? "";
      aiMsgs = [...aiMsgs, { role: "assistant", content: full }];
      aiStreaming = false;
      ($("ai-chat-send") as HTMLButtonElement).disabled = false;
      scheduleScroll();
      if (aiPendingSend) {
        const next = aiPendingSend;
        aiPendingSend = null;
        next();
      }
    }
  };

  try {
    await invoke("ask_ai", { model: modelName, messages: pending, channel: ch });
  } catch (e) {
    wrap.remove();
    const err = String(e);
    if (err.includes("missing-key")) {
      toast("未配置 API Key");
      openAiKeyPanel();
    } else if (err.includes("missing-model")) {
      toast("该平台需要填写模型名（如豆包 ep-xxx）");
      openAiKeyPanel();
    } else {
      toast(`请求失败：${err}`);
    }
    aiStreaming = false;
    ($("ai-chat-send") as HTMLButtonElement).disabled = false;
  }
}

function openAiKeyPanel() {
  const modelName = aiCurrentModel();
  const guide = AI_GUIDES[modelName];
  ($("ai-key-model") as HTMLElement).textContent = modelName;
  const g = $("ai-key-guide") as HTMLElement;
  if (guide) {
    g.innerHTML = `<b>如何申请 ${modelName} API Key：</b>\n${guide.steps}\n\n🔗 申请地址：<a href="#" id="ai-guide-link">${guide.url}</a>`;
    $("ai-guide-link").addEventListener("click", (e) => {
      e.preventDefault();
      invoke("open_external", { url: guide.url }).catch(() => {});
    });
  } else {
    g.textContent = `平台 ${modelName} 暂无内置引导，请在对应官网申请 API Key。`;
  }
  ($("ai-key-input") as HTMLInputElement).value = "";

  const sel = $("ai-model-select") as HTMLSelectElement;
  const custom = $("ai-model-custom") as HTMLInputElement;
  const models = guide?.models ?? [];
  if (models.length) {
    sel.innerHTML =
      models.map((m) => `<option value="${m.name}">${m.name} · ${m.note}</option>`).join("") +
      `<option value="__custom__">✏️ 自定义模型名…</option>`;
    sel.classList.remove("hidden");
    sel.disabled = false;
    custom.classList.add("hidden");
  } else {
    sel.innerHTML = "";
    sel.classList.add("hidden");
    sel.disabled = true;
    custom.classList.remove("hidden");
    custom.placeholder = "输入模型名（如 ep-xxx）";
  }
  sel.onchange = () => {
    if (sel.value === "__custom__") {
      custom.classList.remove("hidden");
      custom.value = "";
      custom.focus();
    } else {
      custom.classList.add("hidden");
    }
  };

  void getConfig().then((c) => {
    if (c.ai_keys?.[modelName]) {
      const k = c.ai_keys[modelName];
      ($("ai-key-input") as HTMLInputElement).value = k.slice(0, 6) + "****" + k.slice(-4);
    }
    const saved = c.ai_models?.[modelName];
    if (saved) {
      if (models.some((m) => m.name === saved)) {
        sel.value = saved;
      } else {
        sel.value = "__custom__";
        custom.classList.remove("hidden");
        custom.value = saved;
      }
    } else if (guide?.defaultModel && models.some((m) => m.name === guide.defaultModel)) {
      sel.value = guide.defaultModel;
    }
  });
  $("ai-key-panel").classList.remove("hidden");
}

async function saveAiKey() {
  const modelName = aiCurrentModel();
  const keyRaw = ($("ai-key-input") as HTMLInputElement).value.trim();
  const sel = $("ai-model-select") as HTMLSelectElement;
  const custom = $("ai-model-custom") as HTMLInputElement;
  const modelRaw = sel.value === "__custom__" ? custom.value.trim() : sel.value;
  const cfg = await getConfig();
  const existing = cfg.ai_keys?.[modelName] ?? "";
  let key = keyRaw;
  if (keyRaw.includes("****")) key = existing;
  if (!key) {
    toast("请先填写 API Key");
    return;
  }
  if (!modelRaw) {
    toast("请选择或填写模型名");
    return;
  }
  cfg.ai_keys = { ...(cfg.ai_keys ?? {}), [modelName]: key };
  cfg.ai_models = { ...(cfg.ai_models ?? {}), [modelName]: modelRaw };
  await persistConfig(cfg);
  $("ai-key-panel").classList.add("hidden");
  ($("ai-panel-apimodel") as HTMLElement).textContent = modelRaw;
  toast(`${modelName} · ${modelRaw} 已配置`);
  openAiPanel();
  if (aiPendingSend) {
    const next = aiPendingSend;
    aiPendingSend = null;
    next();
  }
}

function initAi() {
  const input = $("ai-input") as HTMLInputElement;
  const model = $("ai-model") as HTMLSelectElement;
  const savedModel = localStorage.getItem("ai-model");
  if (savedModel && Array.from(model.options).some((o) => o.value === savedModel)) {
    model.value = savedModel;
  }
  model.addEventListener("change", () => {
    localStorage.setItem("ai-model", model.value);
    if (!$("ai-panel").classList.contains("hidden")) {
      ($("ai-panel-model") as HTMLElement).textContent = model.value;
      fillPanelModelSel();
    }
  });
  const send = async () => {
    const q = input.value.trim();
    if (!q) return;
    const modelName = model.value;
    const key = await aiKeyFor(modelName);
    if (!key) {
      aiPendingSend = () => {
        ($("ai-chat-input") as HTMLTextAreaElement).value = q;
        void sendAiChat();
      };
      openAiKeyPanel();
      return;
    }
    input.value = "";
    openAiPanel();
    ($("ai-chat-input") as HTMLTextAreaElement).value = q;
    void sendAiChat();
  };
  $("ai-send").addEventListener("click", () => void send());
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      void send();
    }
  });

  $("ai-panel-close").addEventListener("click", closeAiPanel);
  $("ai-panel-clear").addEventListener("click", clearAiChat);
  ($("ai-panel-model-select") as HTMLSelectElement).addEventListener("change", () => {
    const modelName = aiCurrentModel();
    const v = ($("ai-panel-model-select") as HTMLSelectElement).value;
    void (async () => {
      const c = await getConfig();
      c.ai_models = { ...(c.ai_models ?? {}), [modelName]: v };
      await persistConfig(c);
      ($("ai-panel-apimodel") as HTMLElement).textContent = v;
      toast(`已切换模型：${v}`);
    })();
  });
  $("ai-attach").addEventListener("click", () => void pickAttachments());
  $("ai-chat-send").addEventListener("click", () => void sendAiChat());
  ($("ai-chat-input") as HTMLTextAreaElement).addEventListener("keydown", (e) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      void sendAiChat();
    }
  });
  ($("ai-chat-input") as HTMLTextAreaElement).addEventListener("paste", handlePasteImage);
  $("ai-panel").addEventListener("paste", handlePasteImage);
  $("ai-panel").addEventListener("click", (e) => {
    if (e.target === e.currentTarget) closeAiPanel();
  });
  $("ai-key-close").addEventListener("click", () => $("ai-key-panel").classList.add("hidden"));
  $("ai-key-save").addEventListener("click", () => void saveAiKey());
  $("ai-key-panel").addEventListener("click", (e) => {
    if (e.target === e.currentTarget) $("ai-key-panel").classList.add("hidden");
  });
}

// ---------- 习惯打卡（喝水/抽烟，按天静默统计） ----------
const HABIT_KEY = "habits";
const HABIT_KEEP_DAYS = 400;

type HabitDay = { water: number; smoke: number };

function habitKey(d: Date): string {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

function loadHabitDays(): Record<string, HabitDay> {
  try {
    const raw = localStorage.getItem(HABIT_KEY);
    if (raw) {
      const obj = JSON.parse(raw);
      if (obj && typeof obj === "object") {
        if (!("date" in obj)) return obj as Record<string, HabitDay>;
        const legacy: Record<string, HabitDay> = {};
        if ((obj.water || 0) > 0 || (obj.smoke || 0) > 0) {
          legacy[obj.date] = { water: obj.water || 0, smoke: obj.smoke || 0 };
        }
        return legacy;
      }
    }
  } catch {
    /* ignore */
  }
  return {};
}

function saveHabitDays(days: Record<string, HabitDay>) {
  const cutoff = new Date();
  cutoff.setDate(cutoff.getDate() - HABIT_KEEP_DAYS);
  const cutKey = habitKey(cutoff);
  for (const k of Object.keys(days)) {
    if (k < cutKey) delete days[k];
  }
  localStorage.setItem(HABIT_KEY, JSON.stringify(days));
}

function todayHabit(days: Record<string, HabitDay>): HabitDay {
  const k = habitKey(new Date());
  if (!days[k]) days[k] = { water: 0, smoke: 0 };
  return days[k];
}

function renderHabits() {
  const h = todayHabit(loadHabitDays());
  $("cnt-water").textContent = String(h.water);
  $("cnt-smoke").textContent = String(h.smoke);
}

function habitBump(key: "water" | "smoke", label: string, emoji: string) {
  const days = loadHabitDays();
  todayHabit(days)[key] += 1;
  saveHabitDays(days);
  renderHabits();
  toast(`${label} +1 ${emoji}`);
}

function habitReset() {
  if (!confirm("确定清零今天的记录？")) return;
  const days = loadHabitDays();
  delete days[habitKey(new Date())];
  saveHabitDays(days);
  renderHabits();
  renderHabitStats();
  toast("今日记录已清零");
}

// ---------- 习惯统计面板 ----------
function statRange(from: Date, to: Date): { days: string[]; total: Record<string, number> } {
  const total: Record<string, number> = { water: 0, smoke: 0 };
  const days: string[] = [];
  const store = loadHabitDays();
  const cur = new Date(from);
  while (cur <= to) {
    const k = habitKey(cur);
    days.push(k);
    const r = store[k];
    if (r) {
      total.water += r.water;
      total.smoke += r.smoke;
    }
    cur.setDate(cur.getDate() + 1);
  }
  return { days, total };
}

function renderHabitStats() {
  const now = new Date();
  const todayK = habitKey(now);
  const store = loadHabitDays();

  const dow = (now.getDay() + 6) % 7;
  const weekStart = new Date(now);
  weekStart.setDate(now.getDate() - dow);
  const week = statRange(weekStart, now);

  const monthStart = new Date(now.getFullYear(), now.getMonth(), 1);
  const month = statRange(monthStart, now);

  const t = store[todayK] || { water: 0, smoke: 0 };
  $("hs-today-water").textContent = String(t.water);
  $("hs-today-smoke").textContent = String(t.smoke);

  const weekDays = week.days.length || 1;
  const monthDays = month.days.length || 1;
  $("hs-week-water").textContent = String(week.total.water);
  $("hs-week-smoke").textContent = String(week.total.smoke);
  $("hs-week-avg").textContent = `${(week.total.water / weekDays).toFixed(1)} / ${(week.total.smoke / weekDays).toFixed(1)}`;
  $("hs-month-water").textContent = String(month.total.water);
  $("hs-month-smoke").textContent = String(month.total.smoke);
  $("hs-month-avg").textContent = `${(month.total.water / monthDays).toFixed(1)} / ${(month.total.smoke / monthDays).toFixed(1)}`;

  const chart = $("hs-chart");
  chart.innerHTML = "";
  const last7: { k: string; water: number; smoke: number; label: string }[] = [];
  for (let i = 6; i >= 0; i--) {
    const d = new Date(now);
    d.setDate(now.getDate() - i);
    const k = habitKey(d);
    const r = store[k] || { water: 0, smoke: 0 };
    last7.push({ k, water: r.water, smoke: r.smoke, label: k === todayK ? "今天" : `${d.getMonth() + 1}/${d.getDate()}` });
  }
  const max = Math.max(1, ...last7.map((x) => Math.max(x.water, x.smoke)));
  for (const x of last7) {
    const col = document.createElement("div");
    col.className = `hs-col${x.k === todayK ? " today" : ""}`;
    const bars = document.createElement("div");
    bars.className = "hs-bars";
    const bw = document.createElement("div");
    bw.className = "hs-bar water";
    bw.style.height = `${Math.round((x.water / max) * 100)}%`;
    bw.title = `${x.label} 喝水 ${x.water} 杯`;
    const bs = document.createElement("div");
    bs.className = "hs-bar smoke";
    bs.style.height = `${Math.round((x.smoke / max) * 100)}%`;
    bs.title = `${x.label} 抽烟 ${x.smoke} 根`;
    bars.append(bw, bs);
    const lbl = document.createElement("div");
    lbl.className = "hs-label";
    lbl.textContent = x.label;
    const val = document.createElement("div");
    val.className = "hs-val";
    val.textContent = `${x.water}/${x.smoke}`;
    col.append(bars, val, lbl);
    chart.appendChild(col);
  }

  const list = $("hs-history");
  list.innerHTML = "";
  const keys = Object.keys(store)
    .filter((k) => k <= todayK)
    .sort()
    .reverse()
    .slice(0, 14);
  if (!keys.length) {
    list.innerHTML = `<div class="hs-empty">暂无历史记录，打卡后自动统计</div>`;
  } else {
    for (const k of keys) {
      const r = store[k];
      if (!r.water && !r.smoke) continue;
      const row = document.createElement("div");
      row.className = "hs-row";
      row.innerHTML = `<span class="hs-row-date">${k}</span><span>💧 ${r.water}杯</span><span>🚬 ${r.smoke}根</span>`;
      list.appendChild(row);
    }
  }
}

function initHabits() {
  renderHabits();
  $("btn-water").addEventListener("click", () => habitBump("water", "喝水", "🥤"));
  $("btn-smoke").addEventListener("click", () => habitBump("smoke", "抽烟", "🚬"));
  $("btn-habit-stats").addEventListener("click", () => {
    renderHabitStats();
    $("habit-stats").classList.remove("hidden");
  });
  $("habit-stats-close").addEventListener("click", () => $("habit-stats").classList.add("hidden"));
  $("habit-stats").addEventListener("click", (e) => {
    if (e.target === e.currentTarget) $("habit-stats").classList.add("hidden");
  });
  $("habit-reset").addEventListener("click", () => habitReset());
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
  const parts = offTime.split(":");
  const h = Number(parts[0]);
  const m = parts.length > 1 ? Number(parts[1]) : 0;
  const off = new Date(
    now.getFullYear(),
    now.getMonth(),
    now.getDate(),
    Number.isFinite(h) ? h : 18,
    Number.isFinite(m) ? m : 0,
    0,
  );
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

// ---------- 便签 ----------
interface Note {
  title: string;
  content: string;
  date: string;
}

const NOTES_KEY = "notes";

function loadNotes(): Note[] {
  const defaults: Note[] = [
    { title: "今天的灵感", content: "优化工作台布局的细节，提升效率与美观并存。", date: "今天 09:45" },
    { title: "项目需求", content: "回帆 MVP 版本功能梳理", date: "8月5日" },
    { title: "服务器备份", content: "每周五自动备份数据", date: "8月1日" },
  ];
  try {
    const raw = localStorage.getItem(NOTES_KEY);
    if (raw !== null) {
      const arr = JSON.parse(raw);
      if (Array.isArray(arr)) return arr;
    }
  } catch {
    /* ignore */
  }
  localStorage.setItem(NOTES_KEY, JSON.stringify(defaults));
  return defaults;
}

function saveNotes(notes: Note[]) {
  localStorage.setItem(NOTES_KEY, JSON.stringify(notes));
}

function renderNotes(showAll = false) {
  const notes = loadNotes();
  const list = $("notes-list");
  list.innerHTML = "";
  const items = showAll ? notes : notes.slice(0, 3);
  for (const [i, n] of items.entries()) {
    const el = document.createElement("div");
    el.className = "note-item";
    const head = document.createElement("div");
    head.className = "note-head";
    const title = document.createElement("span");
    title.className = "note-title";
    title.textContent = n.title;
    const date = document.createElement("span");
    date.className = "note-date";
    date.textContent = n.date;
    const actions = document.createElement("span");
    actions.className = "note-actions";
    const btnEdit = document.createElement("button");
    btnEdit.className = "note-btn";
    btnEdit.textContent = "✎";
    btnEdit.title = "编辑";
    btnEdit.addEventListener("click", () => editNote(i));
    const btnDel = document.createElement("button");
    btnDel.className = "note-btn";
    btnDel.textContent = "✕";
    btnDel.title = "删除";
    btnDel.addEventListener("click", () => removeNote(i));
    actions.append(btnEdit, btnDel);
    head.append(title, date, actions);
    const body = document.createElement("div");
    body.className = "note-body";
    body.textContent = n.content;
    el.append(head, body);
    list.appendChild(el);
  }
  if (!notes.length) {
    list.innerHTML = `<div class="note-empty">暂无便签，点击「+ 新建」添加</div>`;
  }
  const allBtn = $("notes-all");
  const hasMore = notes.length > 3;
  allBtn.style.display = showAll || hasMore ? "" : "none";
  allBtn.textContent = showAll ? "收起 ↑" : "查看全部便签 →";
}

function addNote() {
  const title = prompt("便签标题：");
  if (!title) return;
  const content = prompt("便签内容：") ?? "";
  const now = new Date();
  const date = `${now.getMonth() + 1}月${now.getDate()}日 ${String(now.getHours()).padStart(2, "0")}:${String(now.getMinutes()).padStart(2, "0")}`;
  const notes = loadNotes();
  notes.unshift({ title, content, date });
  saveNotes(notes);
  renderNotes();
}

function editNote(idx: number) {
  const notes = loadNotes();
  const n = notes[idx];
  if (!n) return;
  const title = prompt("便签标题：", n.title);
  if (title === null) return;
  const content = prompt("便签内容：", n.content) ?? "";
  const now = new Date();
  n.title = title;
  n.content = content;
  n.date = `${now.getMonth() + 1}月${now.getDate()}日 ${String(now.getHours()).padStart(2, "0")}:${String(now.getMinutes()).padStart(2, "0")}`;
  saveNotes(notes);
  renderNotes();
}

function removeNote(idx: number) {
  const notes = loadNotes();
  if (idx < 0 || idx >= notes.length) return;
  if (!confirm("确定删除这条便签？")) return;
  notes.splice(idx, 1);
  saveNotes(notes);
  renderNotes();
}

function initNotes() {
  renderNotes();
  $("btn-note-add").addEventListener("click", addNote);
  $("notes-all").addEventListener("click", () => {
    const notes = loadNotes();
    const expanded = $("notes-all").textContent.includes("收起");
    if (expanded) {
      renderNotes();
    } else if (notes.length > 3) {
      renderNotes(true);
    }
  });
}

// ---------- 主题切换 ----------
function applyTheme(theme: string) {
  document.body.classList.remove("theme-light", "theme-dark", "theme-system");
  document.body.classList.add(`theme-${theme}`);
  localStorage.setItem("theme", theme);
  const btns = document.querySelectorAll<HTMLButtonElement>(".theme-btn");
  btns.forEach((b) => b.classList.toggle("active", b.dataset.theme === theme));
}

function initTheme() {
  const saved = localStorage.getItem("theme") || "dark";
  applyTheme(saved);
  document.querySelectorAll<HTMLButtonElement>(".theme-btn").forEach((b) => {
    b.addEventListener("click", () => applyTheme(b.dataset.theme || "dark"));
  });
}

// ---------- 运行时长 ----------
const APP_START = Date.now();

function updateUptime() {
  const el = $("uptime");
  if (!el) return;
  const diff = Math.floor((Date.now() - APP_START) / 1000);
  const hh = String(Math.floor(diff / 3600)).padStart(2, "0");
  const mm = String(Math.floor((diff % 3600) / 60)).padStart(2, "0");
  const ss = String(diff % 60).padStart(2, "0");
  el.textContent = `⏱ 已运行 ${hh}:${mm}:${ss}`;
}

function initUptime() {
  updateUptime();
  setInterval(updateUptime, 1000);
  const check = $("btn-check-update") as HTMLButtonElement;
  if (check) {
    check.addEventListener("click", async () => {
      check.textContent = "检查中…";
      check.disabled = true;
      try {
        const r = await fetch("https://api.github.com/repos/ruichou/AI_workstasion/releases/latest", {
          headers: { Accept: "application/vnd.github+json" },
        });
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
        const j = await r.json();
        const tag = String(j.tag_name ?? "unknown").replace(/^v/, "");
        if (tag && tag !== "1.0.0") {
          toast(`发现新版本 v${tag}，请到仓库 Release 下载更新`);
          invoke("open_external", { url: "https://github.com/ruichou/AI_workstasion/releases/latest" }).catch(() => {});
        } else {
          toast("已是最新版本 v1.0.0");
        }
      } catch {
        toast("检查更新失败（网络或仓库无 Release）");
      }
      check.textContent = "↑ 检查更新";
      check.disabled = false;
    });
  }
}

// ---------- 城市设置（全国省市区县镇级联） ----------
import pcasData from "./data/pcas.json";

type DivTree = Record<string, Record<string, Record<string, string[]>>>;

const DIVS = pcasData as DivTree;

function selProvs(): string[] {
  return Object.keys(DIVS);
}

function selCities(prov: string): string[] {
  const c = DIVS[prov];
  return c ? Object.keys(c) : [];
}

function selDists(prov: string, city: string): string[] {
  const d = DIVS[prov]?.[city];
  return d ? Object.keys(d) : [];
}

function selTowns(prov: string, city: string, dist: string): string[] {
  return DIVS[prov]?.[city]?.[dist] ?? [];
}

function fillSelect(sel: HTMLSelectElement, items: string[], placeholder: string) {
  sel.innerHTML = `<option value="">${placeholder}</option>` + items.map((x) => `<option value="${x}">${x}</option>`).join("");
  sel.disabled = items.length === 0;
}

function findBest(list: string[], token: string): string | null {
  if (!token) return null;
  const core = token.replace(/省|市|区|县|镇|自治|特别行政区/g, "");
  for (const n of list) {
    if (n === token || n.includes(core) || (core.length >= 2 && core.includes(n.replace(/省|市|区|县|镇/g, "")))) return n;
  }
  for (const n of list) {
    if (core.length >= 2 && n.includes(core.slice(0, 2))) return n;
  }
  return null;
}

async function geocodePlace(name: string): Promise<{ lat: number; lon: number } | null> {
  try {
    const r = await fetch(
      `https://geocoding-api.open-meteo.com/v1/search?name=${encodeURIComponent(name)}&count=5&country=CN&language=zh&format=json`,
    );
    const j = await r.json();
    const hit = (j.results ?? []).find((x: { country?: string; latitude: number; longitude: number }) => {
      const cn = (x.country ?? "").toLowerCase();
      return cn.includes("china") || cn.includes("cn") || cn === "CN";
    });
    if (hit) return { lat: hit.latitude, lon: hit.longitude };
  } catch {
    /* ignore */
  }
  return null;
}

function renderCityCasc() {
  const prov = $("city-prov") as HTMLSelectElement;
  const city = $("city-cit") as HTMLSelectElement;
  const dist = $("city-dist") as HTMLSelectElement;
  const town = $("city-town") as HTMLSelectElement;

  const fill = () => {
    const p = prov.value;
    const c = city.value;
    const d = dist.value;
    fillSelect(city, selCities(p), "选择市");
    fillSelect(dist, selDists(p, c), "选择区/县");
    fillSelect(town, selTowns(p, c, d), "选择镇/街道（可省）");
    if (c) city.value = c;
    if (d) dist.value = d;
  };

  fillSelect(prov, selProvs(), "选择省");
  prov.addEventListener("change", () => {
    city.innerHTML = "";
    dist.innerHTML = "";
    town.innerHTML = "";
    fill();
  });
  city.addEventListener("change", () => {
    dist.innerHTML = "";
    town.innerHTML = "";
    fill();
  });
  dist.addEventListener("change", () => fill());

  void (async () => {
    const cfg = await getConfig();
    const tokens = (cfg.city ?? "").split(/[·\s]+/).filter(Boolean);
    if (tokens.length) {
      const p = findBest(selProvs(), tokens[tokens.length - 3] ?? tokens[0]) || findBest(selProvs(), tokens[0]);
      if (p) {
        prov.value = p;
        const cities = selCities(p);
        const c = findBest(cities, tokens[tokens.length - 2] ?? "") || findBest(cities, tokens[1] ?? "");
        fill();
        if (c) {
          city.value = c;
          const dists = selDists(p, c);
          const d = findBest(dists, tokens[tokens.length - 1] ?? "");
          fill();
          if (d) {
            dist.value = d;
            const towns = selTowns(p, c, d);
            const t = findBest(towns, tokens[tokens.length - 1] ?? "");
            fill();
            if (t) town.value = t;
          }
        }
      }
    }
  })();
}

async function saveCityFromCasc() {
  const prov = $("city-prov") as HTMLSelectElement;
  const city = $("city-cit") as HTMLSelectElement;
  const dist = $("city-dist") as HTMLSelectElement;
  const town = $("city-town") as HTMLSelectElement;
  if (!prov.value || !city.value || !dist.value) {
    toast("请选择 省 + 市 + 区/县");
    return;
  }
  const name = [prov.value, city.value, dist.value, town.value].filter(Boolean).join(" ");
  const cfg = await getConfig();
  let geo = town.value ? await geocodePlace(town.value) : null;
  if (!geo) geo = await geocodePlace(dist.value);
  if (!geo) geo = await geocodePlace(city.value);
  cfg.city = name;
  cfg.lat = geo ? geo.lat : null;
  cfg.lon = geo ? geo.lon : null;
  await invoke("save_config", { cfg }).catch((e) => alert(e));
  refreshWeather();
  $("city-picker").classList.add("hidden");
  toast(geo ? `已切换到 ${name}` : `已保存 ${name}（未获取到坐标）`);
}

function initCityPicker() {
  $("btn-city").addEventListener("click", () => {
    $("city-picker").classList.remove("hidden");
    renderCityCasc();
  });
  $("city-close").addEventListener("click", () => $("city-picker").classList.add("hidden"));
  $("city-picker").addEventListener("click", (e) => {
    if (e.target === e.currentTarget) $("city-picker").classList.add("hidden");
  });
  const btnSave = $("city-casc-save");
  btnSave.addEventListener("click", () => {
    void saveCityFromCasc();
  });
  $("city-name").addEventListener("input", () => {
    void (async () => {
      const name = ($("city-name") as HTMLInputElement).value.trim();
      if (!name) return;
      const geo = await geocodePlace(name);
      if (geo) {
        ($("city-lat") as HTMLInputElement).value = String(geo.lat);
        ($("city-lon") as HTMLInputElement).value = String(geo.lon);
      }
    })();
  });
  $("city-custom-save").addEventListener("click", () => {
    void (async () => {
      const name = ($("city-name") as HTMLInputElement).value.trim();
      const lat = Number(($("city-lat") as HTMLInputElement).value);
      const lon = Number(($("city-lon") as HTMLInputElement).value);
      if (!name || !Number.isFinite(lat) || !Number.isFinite(lon)) {
        toast("请填写完整：名称 + 纬度 + 经度");
        return;
      }
      const cfg = await getConfig();
      cfg.city = name;
      cfg.lat = lat;
      cfg.lon = lon;
      await invoke("save_config", { cfg }).catch((e) => alert(e));
      refreshWeather();
      toast(`已切换到 ${name}`);
      $("city-picker").classList.add("hidden");
    })();
  });
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
  for (const fn of [
    initSettings,
    initClean,
    initCollapse,
    initAi,
    initWindow,
    initPicker,
    initHabits,
    initNotes,
    initTheme,
    initUptime,
    initCityPicker,
  ]) {
    try {
      fn();
    } catch (e) {
      console.error("init failed:", fn.name, e);
    }
  }
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
}

init();
