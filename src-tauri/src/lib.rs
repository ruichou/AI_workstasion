use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use sysinfo::{Disks, System};
use tauri::{AppHandle, Manager, State};

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
struct AppItem {
    name: String,
    path: String,
    emoji: String,
    args: Option<String>,
}

impl Default for AppItem {
    fn default() -> Self {
        Self {
            name: String::new(),
            path: String::new(),
            emoji: String::from("📦"),
            args: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct SiteCfg {
    base_url: String,
    cookie: String,
    username: String,
    password: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
struct Config {
    city: String,
    lat: Option<f64>,
    lon: Option<f64>,
    ai_url: String,
    off_time: String,
    apps: Vec<AppItem>,
    ai_keys: std::collections::HashMap<String, String>,
    ai_models: std::collections::HashMap<String, String>,
    sites: std::collections::HashMap<String, SiteCfg>,
    monitored_nicks: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        let blank = |name: &str, emoji: &str| AppItem {
            name: String::from(name),
            path: String::new(),
            emoji: String::from(emoji),
            args: None,
        };
        Self {
            city: String::from("常州·湖塘"),
            lat: Some(31.7332),
            lon: Some(119.9649),
            ai_url: String::from("https://www.qianwen.com/"),
            off_time: String::from("18:00"),
            apps: vec![
                blank("ChatGPT", "🤖"),
                blank("OpenCode", "◯"),
                blank("VS Code", "💻"),
                blank("微信开发者工具", "🛠"),
                blank("Chrome", "🌐"),
                blank("DeepSeek", "🧠"),
                blank("Mimo", "🎨"),
                blank("Xshell", "🖥"),
                blank("Typora", "📝"),
                blank("GitHub", "🐙"),
                blank("服务器管理", "☁️"),
            ],
            ai_keys: std::collections::HashMap::new(),
            ai_models: std::collections::HashMap::new(),
            sites: std::collections::HashMap::new(),
            monitored_nicks: Vec::new(),
        }
    }
}

#[derive(Serialize, Clone)]
struct SysInfo {
    cpu: f32,
    mem_used: u64,
    mem_total: u64,
    disk_used: u64,
    disk_total: u64,
}

#[derive(Serialize, Default)]
struct CleanResult {
    freed_mb: u64,
    processes: u32,
}

#[derive(Serialize, Default)]
struct JunkResult {
    files: u64,
    bytes: u64,
}

#[derive(Serialize, Clone)]
struct Temps {
    cpu: Option<f32>,
    disk: Option<f32>,
}

#[derive(Serialize, Clone)]
struct Weather {
    city: String,
    temp: f64,
    feels: f64,
    humidity: f64,
    wind: f64,
    code: u8,
    is_day: bool,
}

struct AppState {
    config: Mutex<Config>,
    geocode: Mutex<Option<(f64, f64)>>,
    http: reqwest::Client,
    eye_ramp: Mutex<Option<[u16; 768]>>,
}

impl AppState {
    fn new() -> Self {
        Self {
            config: Mutex::new(Config::default()),
            geocode: Mutex::new(None),
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
            eye_ramp: Mutex::new(None),
        }
    }
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("config.json"))
}

fn load_config(app: &AppHandle) -> Config {
    let path = match config_path(app) {
        Ok(p) => p,
        Err(_) => return Config::default(),
    };
    match fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_else(|_| {
            fs::write(&path, serde_json::to_string_pretty(&Config::default()).unwrap()).ok();
            Config::default()
        }),
        Err(_) => {
            // 首次启动：尝试从安装包资源读取预置配置（含预置 API Key，不含账号密码）
            let bundled = app
                .path()
                .resource_dir()
                .ok()
                .map(|d| d.join("config.json"))
                .and_then(|p| fs::read_to_string(p).ok())
                .and_then(|raw| serde_json::from_str::<Config>(&raw).ok());
            if let Some(cfg) = bundled {
                fs::write(&path, serde_json::to_string_pretty(&cfg).unwrap()).ok();
                return cfg;
            }
            fs::write(&path, serde_json::to_string_pretty(&Config::default()).unwrap()).ok();
            Config::default()
        }
    }
}

#[tauri::command]
fn get_config(state: State<'_, AppState>) -> Config {
    state.config.lock().unwrap().clone()
}

use std::sync::OnceLock;

fn sysinfo_sys() -> &'static std::sync::Mutex<System> {
    static SYS: OnceLock<std::sync::Mutex<System>> = OnceLock::new();
    SYS.get_or_init(|| std::sync::Mutex::new(System::new_all()))
}

fn sysinfo_disks() -> &'static std::sync::Mutex<Disks> {
    static DISKS: OnceLock<std::sync::Mutex<Disks>> = OnceLock::new();
    DISKS.get_or_init(|| std::sync::Mutex::new(Disks::new_with_refreshed_list()))
}

#[tauri::command]
async fn get_sysinfo() -> SysInfo {
    {
        let mut sys = sysinfo_sys().lock().unwrap();
        sys.refresh_memory();
        sys.refresh_cpu_usage();
    }
    tokio::time::sleep(Duration::from_millis(250)).await;
    let (cpu, mem_used, mem_total) = {
        let mut sys = sysinfo_sys().lock().unwrap();
        sys.refresh_cpu_usage();
        (
            sys.global_cpu_usage(),
            sys.used_memory(),
            sys.total_memory(),
        )
    };
    let (disk_used, disk_total) = {
        let mut disks = sysinfo_disks().lock().unwrap();
        disks.refresh(true);
        match disks
            .iter()
            .find(|d| d.mount_point().to_string_lossy().to_uppercase().starts_with("C:"))
            .or_else(|| disks.iter().max_by_key(|d| d.total_space()))
        {
            Some(d) => (
                d.total_space().saturating_sub(d.available_space()),
                d.total_space(),
            ),
            None => (0, 0),
        }
    };

    SysInfo {
        cpu,
        mem_used,
        mem_total,
        disk_used,
        disk_total,
    }
}

/// 一键清理内存：对除本进程和系统关键进程外的所有进程执行 EmptyWorkingSet，
/// 把物理内存页刷回页面文件。不会关闭任何程序、不会丢失任何数据。
#[tauri::command]
async fn clean_memory() -> Result<CleanResult, String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        GetCurrentProcessId, OpenProcess, SetProcessWorkingSetSize, PROCESS_ACCESS_RIGHTS,
        PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_QUOTA,
    };

    let mut sys = System::new_all();
    sys.refresh_memory();
    let before = sys.used_memory();

    let own_pid = unsafe { GetCurrentProcessId() };
    let access = PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SET_QUOTA;
    let mut cleaned = 0u32;

    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    for p in sys.processes().values() {
        let pid = p.pid().as_u32();
        if pid == 0 || pid == 4 || pid == own_pid {
            continue;
        }
        unsafe {
            let Ok(handle) = OpenProcess(access, false, pid) else { continue };
            if SetProcessWorkingSetSize(handle, usize::MAX, usize::MAX).is_ok() {
                cleaned += 1;
            }
            let _ = CloseHandle(handle);
        }
    }

    tokio::time::sleep(Duration::from_millis(600)).await;
    sys.refresh_memory();
    let after = sys.used_memory();

    Ok(CleanResult {
        freed_mb: before.saturating_sub(after) / (1024 * 1024),
        processes: cleaned,
    })
}

fn dir_size(path: &std::path::Path) -> u64 {
    let mut total = 0u64;
    let mut visited = 0u64;
    let mut stack: Vec<PathBuf> = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        visited += 1;
        if visited > 8000 {
            return total;
        }
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            let md = match e.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            if md.is_dir() {
                stack.push(p);
            } else {
                total += md.len();
            }
        }
    }
    total
}

/// 一键清理垃圾：删除用户临时目录与 Windows 临时目录中的内容。
/// 正在被占用/使用的文件删除会失败并被自动跳过，绝不触碰任何正常文件。
#[tauri::command]
async fn clean_junk() -> Result<JunkResult, String> {
    let mut dirs: Vec<PathBuf> = vec![std::env::temp_dir()];
    if let Some(windir) = std::env::var_os("WINDIR") {
        dirs.push(PathBuf::from(&windir).join("Temp"));
    }

    let mut files = 0u64;
    let mut bytes = 0u64;
    for dir in dirs {
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            let size = dir_size(&p);
            let ok = if p.is_dir() {
                fs::remove_dir_all(&p).is_ok()
            } else {
                fs::remove_file(&p).is_ok()
            };
            if ok {
                files += 1;
                bytes += size;
            }
        }
    }
    Ok(JunkResult { files, bytes })
}

fn city_coords(city: &str) -> Option<(f64, f64)> {
    let (lat, lon) = match city.trim() {
        "北京" => (39.9042, 116.4074),
        "上海" => (31.2304, 121.4737),
        "广州" => (23.1291, 113.2644),
        "深圳" => (22.5431, 114.0579),
        "成都" => (30.5728, 104.0668),
        "杭州" => (30.2741, 120.1551),
        "武汉" => (30.5928, 114.3055),
        "西安" => (34.3416, 108.9398),
        "重庆" => (29.5630, 106.5516),
        "南京" => (32.0603, 118.7969),
        "天津" => (39.3434, 117.3616),
        "苏州" => (31.2989, 120.5853),
        "常州" => (31.7332, 119.9649),
        "青岛" => (36.0671, 120.3826),
        "郑州" => (34.7466, 113.6254),
        "长沙" => (28.2282, 112.9388),
        "沈阳" => (41.8057, 123.4315),
        "厦门" => (24.4798, 118.0894),
        "香港" => (22.3193, 114.1694),
        "台北" => (25.0330, 121.5654),
        _ => (39.9042, 116.4074),
    };
    Some((lat, lon))
}

async fn resolve_coords(state: &AppState, cfg: &Config) -> Result<(f64, f64), String> {
    if let (Some(lat), Some(lon)) = (cfg.lat, cfg.lon) {
        return Ok((lat, lon));
    }
    if let Some(g) = *state.geocode.lock().unwrap() {
        return Ok(g);
    }
    let coords = city_coords(&cfg.city);
    *state.geocode.lock().unwrap() = coords;
    Ok(coords.unwrap_or((39.9042, 116.4074)))
}

fn wmo_desc(code: u8) -> (String, &'static str) {
    let (emoji, desc) = match code {
        0 => ("☀️", "晴"),
        1 => ("🌤", "晴间多云"),
        2 => ("⛅", "多云"),
        3 => ("☁️", "阴"),
        45 | 48 => ("🌫", "雾"),
        51 | 53 | 55 => ("🌦", "毛毛雨"),
        56 | 57 => ("🌧", "冻毛毛雨"),
        61 | 63 | 65 => ("🌧", "雨"),
        66 | 67 => ("🌧", "冻雨"),
        71 | 73 | 75 => ("🌨", "雪"),
        77 => ("❄️", "雪粒"),
        80..=82 => ("🌦", "阵雨"),
        85 | 86 => ("🌨", "阵雪"),
        95 => ("⛈", "雷暴"),
        96 | 99 => ("⛈", "雷暴伴冰雹"),
        _ => ("🌡", "未知"),
    };
    (emoji.to_string(), desc)
}

#[tauri::command]
async fn get_weather(state: State<'_, AppState>) -> Result<Weather, String> {
    let cfg = state.config.lock().unwrap().clone();
    let (lat, lon) = resolve_coords(&state, &cfg).await?;

    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}&current=temperature_2m,relative_humidity_2m,apparent_temperature,weather_code,wind_speed_10m,is_day&timezone=auto"
    );
    let resp = state
        .http
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("天气请求失败: {e}"))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("解析失败: {e}"))?;

    let cur = resp
        .get("current")
        .ok_or_else(|| String::from("响应缺少 current 字段"))?;
    let getf = |k: &str| cur.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0);
    let code = cur.get("weather_code").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
    let is_day = cur.get("is_day").and_then(|v| v.as_u64()).unwrap_or(1) == 1;
    let _ = wmo_desc(code);

    Ok(Weather {
        city: cfg.city.clone(),
        temp: getf("temperature_2m"),
        feels: getf("apparent_temperature"),
        humidity: getf("relative_humidity_2m"),
        wind: getf("wind_speed_10m"),
        code,
        is_day,
    })
}

fn split_args(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quote = false;
    let mut has = false;
    for c in input.chars() {
        match c {
            '"' => in_quote = !in_quote,
            ' ' | '\t' if !in_quote => {
                if has {
                    out.push(std::mem::take(&mut cur));
                    has = false;
                }
            }
            _ => {
                cur.push(c);
                has = true;
            }
        }
    }
    if has {
        out.push(cur);
    }
    out
}

#[tauri::command]
fn launch_app(path: String, args: Option<String>) -> Result<(), String> {
    if path.to_lowercase().ends_with(".lnk") {
        std::process::Command::new("cmd")
            .args(["/c", "start", "", &path])
            .spawn()
            .map_err(|e| format!("启动失败: {e}"))?;
        return Ok(());
    }
    let mut cmd = std::process::Command::new(&path);
    if let Some(a) = args {
        let a = a.trim();
        if !a.is_empty() {
            cmd.args(split_args(a));
        }
    }
    cmd.spawn()
        .map_err(|e| format!("启动失败: {e}"))?;
    Ok(())
}

/// 解析 .lnk 快捷方式指向的真实 exe 路径
fn lnk_target(lnk: &str) -> Option<String> {
    use windows::core::{Interface, PCWSTR};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT, IPersistFile,
        STGM_READ,
    };
    use windows::Win32::UI::Shell::IShellLinkW;
    const CLSID_SHELL_LINK: windows::core::GUID =
        windows::core::GUID::from_u128(0x00021401_0000_0000_C000_000000000046);
    unsafe {
        let _ = CoInitializeEx(None, COINIT(0));
        let link: IShellLinkW = CoCreateInstance(&CLSID_SHELL_LINK, None, CLSCTX_ALL).ok()?;
        let persist: IPersistFile = link.cast().ok()?;
        let wide: Vec<u16> = lnk.encode_utf16().chain(std::iter::once(0)).collect();
        persist.Load(PCWSTR(wide.as_ptr()), STGM_READ).ok()?;
        let mut buf = [0u16; 1024];
        let _ = link.GetPath(&mut buf, std::ptr::null_mut(), 0);
        CoUninitialize();
        let target = String::from_utf16_lossy(&buf).trim_end_matches('\0').to_string();
        if target.is_empty() { None } else { Some(target) }
    }
}

/// 查找运行中且 exe 名匹配的进程
fn find_process_by_exe(exe_path: &str) -> Option<u32> {
    let exe_name = std::path::Path::new(exe_path)
        .file_name()?
        .to_string_lossy()
        .to_lowercase();
    let mut sys = System::new_all();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, false);
    sys.processes().values().find_map(|p| {
        p.exe()
            .and_then(|e| e.file_name())
            .map(|f| f.to_string_lossy().to_lowercase() == exe_name)
            .unwrap_or(false)
            .then(|| p.pid().as_u32())
    })
}

/// 激活指定进程的窗口（最小化则还原并置前）
fn activate_process_window(pid: u32) -> bool {
    use windows::Win32::Foundation::LPARAM;
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, IsWindowVisible, SetForegroundWindow, ShowWindow,
        SW_RESTORE,
    };
    use windows::Win32::Foundation::HWND;
    static mut TARGET_PID: u32 = 0;
    static mut SLOT: Option<HWND> = None;
    unsafe {
        TARGET_PID = pid;
        SLOT = None;
        extern "system" fn enum_cb(hwnd: HWND, _lparam: LPARAM) -> windows::core::BOOL {
            let mut wpid: u32 = 0;
            unsafe {
                let _ = GetWindowThreadProcessId(hwnd, Some(&mut wpid));
                if wpid == TARGET_PID && IsWindowVisible(hwnd).as_bool() {
                    SLOT = Some(hwnd);
                    return windows::core::BOOL(0);
                }
            }
            windows::core::BOOL(1)
        }
        let _ = EnumWindows(Some(enum_cb), LPARAM(0));
        if let Some(hwnd) = SLOT {
            let _ = ShowWindow(hwnd, SW_RESTORE);
            let _ = SetForegroundWindow(hwnd);
            return true;
        }
    }
    false
}

/// 点击快速启动：已在运行则激活其窗口，否则正常启动
#[tauri::command]
fn launch_or_activate(path: String, args: Option<String>) -> Result<(), String> {
    let exe = if path.to_lowercase().ends_with(".lnk") {
        lnk_target(&path).unwrap_or_default()
    } else {
        path.clone()
    };
    if !exe.is_empty() {
        if let Some(pid) = find_process_by_exe(&exe) {
            if activate_process_window(pid) {
                return Ok(());
            }
        }
    }
    launch_app(path, args)
}

#[tauri::command]
fn open_config(app: AppHandle) -> Result<String, String> {
    let path = config_path(&app)?;
    std::process::Command::new("notepad")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("打开失败: {e}"))?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
fn reload_config(app: AppHandle, state: State<'_, AppState>) -> Config {
    let cfg = load_config(&app);
    *state.config.lock().unwrap() = cfg.clone();
    cfg
}

#[cfg(target_os = "windows")]
mod temps {
    use std::collections::HashMap;
    use wmi::{Variant, WMIConnection};

    pub fn cpu_temp_celsius() -> Option<f32> {
        let conn = WMIConnection::new().ok()?;
        let rows: Vec<HashMap<String, Variant>> = conn
            .raw_query("SELECT * FROM Win32_PerfFormattedData_Counters_ThermalZoneInformation")
            .ok()?;
        for row in rows {
            if let Some(Variant::UI4(n)) = row.get("Temperature") {
                let c = *n as f64 / 10.0;
                if c > 0.0 && c < 150.0 {
                    return Some(c as f32);
                }
            }
        }
        None
    }

    pub fn disk_temp_celsius() -> Option<f32> {
        let conn = WMIConnection::with_namespace_path(r"root\Microsoft\Windows\Storage").ok()?;
        let rows: Vec<HashMap<String, Variant>> = conn
            .raw_query("SELECT * FROM MSFT_PhysicalDisk")
            .ok()?;
        let mut best: Option<f32> = None;
        for row in rows {
            if let Some(Variant::UI2(n)) = row.get("Temperature") {
                if *n > 0 && *n < 150 {
                    best = Some(best.map_or(*n as f32, |b| b.max(*n as f32)));
                }
            }
        }
        if best.is_some() {
            return best;
        }
        let rows: Vec<HashMap<String, Variant>> = conn
            .raw_query("SELECT * FROM MSFT_StorageReliabilityCounter")
            .ok()?;
        for row in rows {
            if let Some(Variant::UI2(n)) = row.get("Temperature") {
                if *n > 0 && *n < 150 {
                    best = Some(best.map_or(*n as f32, |b| b.max(*n as f32)));
                }
            }
        }
        if best.is_some() {
            return best;
        }
        let rows: Vec<HashMap<String, Variant>> = conn
            .raw_query("SELECT * FROM MSFT_StorageHealth")
            .ok()?;
        for row in rows {
            if let Some(Variant::UI2(n)) = row.get("Temperature") {
                if *n > 0 && *n < 150 {
                    best = Some(best.map_or(*n as f32, |b| b.max(*n as f32)));
                }
            }
        }
        best
    }
}

static TEMP_CACHE: OnceLock<std::sync::Mutex<Option<(std::time::Instant, Temps)>>> = OnceLock::new();

#[tauri::command]
async fn get_temps() -> Result<Temps, String> {
    let cache = TEMP_CACHE.get_or_init(|| std::sync::Mutex::new(None));
    if let Some((at, t)) = cache.lock().unwrap().as_ref() {
        if at.elapsed() < Duration::from_secs(10) {
            return Ok(t.clone());
        }
    }
    let cpu = tauri::async_runtime::spawn_blocking(|| temps::cpu_temp_celsius())
        .await
        .ok()
        .flatten();
    let disk = tauri::async_runtime::spawn_blocking(|| temps::disk_temp_celsius())
        .await
        .ok()
        .flatten();
    let t = Temps { cpu, disk };
    *cache.lock().unwrap() = Some((std::time::Instant::now(), t.clone()));
    Ok(t)
}

#[cfg(target_os = "windows")]
mod audio {
    use windows::core::GUID;
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{eMultimedia, eRender, IMMDeviceEnumerator};
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX, COINIT};

    const CLSID_MM_DEVICE_ENUMERATOR: GUID = GUID::from_u128(0xBCDE0395_E52F_467C_8E3D_C4579291692E);

    fn with_volume<R>(f: impl FnOnce(&IAudioEndpointVolume) -> Result<R, String>) -> Result<R, String> {
        unsafe {
            let mut need_uninit = false;
            let hr = CoInitializeEx(None, COINIT(0));
            if hr.is_ok() {
                need_uninit = hr.0 == 0;
            } else if hr.0 as u32 != 0x80010106 {
                return Err(format!("COM 初始化失败: {hr}"));
            }
            let result = (|| {
                let enumerator: IMMDeviceEnumerator =
                    match CoCreateInstance(&CLSID_MM_DEVICE_ENUMERATOR, None, CLSCTX(1)) {
                        Ok(e) => e,
                        Err(e) => return Err(format!("创建设备枚举器失败: {e}")),
                    };
                let device = match enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia) {
                    Ok(d) => d,
                    Err(e) => return Err(format!("获取默认音频输出设备失败: {e}")),
                };
                let volume = match device.Activate::<IAudioEndpointVolume>(CLSCTX(1), None) {
                    Ok(v) => v,
                    Err(e) => return Err(format!("激活音量接口失败: {e}")),
                };
                f(&volume)
            })();
            if need_uninit {
                CoUninitialize();
            }
            result
        }
    }

    pub fn get_volume() -> Result<Option<f32>, String> {
        with_volume(|v| unsafe { v.GetMasterVolumeLevelScalar() }
            .map(Some)
            .map_err(|e| format!("读取音量失败: {e}")))
    }

    pub fn set_volume(level: f32) -> Result<(), String> {
        with_volume(|v| unsafe { v.SetMasterVolumeLevelScalar(level, &GUID::zeroed()) }
            .map(|_| ())
            .map_err(|e| format!("设置音量失败: {e}")))
    }
}

#[tauri::command]
fn get_volume() -> Result<Option<f32>, String> {
    audio::get_volume()
}

#[tauri::command]
fn set_volume(level: f32) -> Result<(), String> {
    audio::set_volume(level.clamp(0.0, 1.0))
}

#[tauri::command]
fn set_window_opacity(app: AppHandle, level: f64) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::COLORREF;
        use windows::Win32::UI::WindowsAndMessaging::{
            GetWindowLongPtrW, SetLayeredWindowAttributes, SetWindowLongPtrW, GWL_EXSTYLE,
            LWA_ALPHA, WS_EX_LAYERED,
        };
        unsafe {
            let hwnd = app.get_window("main").and_then(|w| w.hwnd().ok()).ok_or_else(|| String::from("获取窗口句柄失败"))?;
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as isize;
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, (ex | WS_EX_LAYERED.0 as isize) as _);
            let alpha = (level.clamp(0.0, 1.0) * 255.0).round() as u8;
            SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
mod gamma {
    use std::ffi::c_void;
    use windows::Win32::Graphics::Gdi::{GetDC, ReleaseDC};
    use windows::Win32::UI::ColorSystem::{GetDeviceGammaRamp, SetDeviceGammaRamp};

    pub fn get_original(ramp: &mut [u16; 768]) -> bool {
        unsafe {
            let hdc = GetDC(None);
            if hdc.is_invalid() {
                return false;
            }
            let ok = GetDeviceGammaRamp(hdc, ramp.as_mut_ptr() as *mut c_void).as_bool();
            let _ = ReleaseDC(None, hdc);
            ok
        }
    }

    pub fn set_ramp(ramp: &[u16; 768]) -> bool {
        unsafe {
            let hdc = GetDC(None);
            if hdc.is_invalid() {
                return false;
            }
            let ok = SetDeviceGammaRamp(hdc, ramp.as_ptr() as *const c_void).as_bool();
            let _ = ReleaseDC(None, hdc);
            ok
        }
    }
}

fn linear_ramp() -> [u16; 768] {
    let mut ramp = [0u16; 768];
    for i in 0..256 {
        let v = (i as f64 / 255.0 * 65535.0).round() as u16;
        ramp[i] = v;
        ramp[i + 256] = v;
        ramp[i + 512] = v;
    }
    ramp
}

#[tauri::command]
fn set_eye_care(state: State<'_, AppState>, enabled: bool, intensity: f64) -> Result<(), String> {
    let intensity = intensity.clamp(0.0, 1.0);
    if enabled {
        let mut orig = [0u16; 768];
        let saved = *state.eye_ramp.lock().unwrap();
        if let Some(s) = saved {
            orig = s;
        } else if !gamma::get_original(&mut orig) {
            orig = linear_ramp();
        }
        *state.eye_ramp.lock().unwrap() = Some(orig);
        let base = [1.0f64, 0.90, 0.70];
        let scales = [
            1.0 + (base[0] - 1.0) * intensity,
            1.0 + (base[1] - 1.0) * intensity,
            1.0 + (base[2] - 1.0) * intensity,
        ];
        let mut ramp = [0u16; 768];
        for i in 0..256 {
            ramp[i] = (orig[i] as f64 * scales[0]).clamp(0.0, 65535.0) as u16;
            ramp[i + 256] = (orig[i + 256] as f64 * scales[1]).clamp(0.0, 65535.0) as u16;
            ramp[i + 512] = (orig[i + 512] as f64 * scales[2]).clamp(0.0, 65535.0) as u16;
        }
        if !gamma::set_ramp(&ramp) {
            return Err(String::from("设置护眼色温失败（可能被显卡驱动拦截）"));
        }
    } else {
        let orig = state
            .eye_ramp
            .lock()
            .unwrap()
            .take()
            .unwrap_or_else(linear_ramp);
        if !gamma::set_ramp(&orig) {
            return Err(String::from("恢复色温失败"));
        }
    }
    Ok(())
}

#[tauri::command]
fn launch_tool(id: String) -> Result<(), String> {
    let (path, args) = match id.as_str() {
        "explorer" => ("explorer.exe", ""),
        "notepad" => ("notepad.exe", ""),
        "cmd" => ("cmd.exe", ""),
        "snipping" => ("SnippingTool.exe", ""),
        "calc" => ("calc.exe", ""),
        "recycle" => ("explorer.exe", "shell:RecycleBinFolder"),
        "control" => ("control.exe", ""),
        "taskmgr" => ("taskmgr.exe", ""),
        other => return Err(format!("未知工具: {other}")),
    };
    let mut cmd = std::process::Command::new(path);
    if !args.is_empty() {
        cmd.arg(args);
    }
    cmd.spawn().map_err(|e| format!("启动失败: {e}"))?;
    Ok(())
}

#[tauri::command]
fn restore_eye_care(state: State<'_, AppState>) -> Result<(), String> {
    let orig = state
        .eye_ramp
        .lock()
        .unwrap()
        .take()
        .unwrap_or_else(linear_ramp);
    if !gamma::set_ramp(&orig) {
        return Err(String::from("恢复色温失败"));
    }
    Ok(())
}

#[tauri::command]
fn set_window_size(app: AppHandle, width: f64, height: f64) -> Result<(), String> {
    let win = app
        .get_webview_window("main")
        .ok_or_else(|| String::from("找不到主窗口"))?;
    win.set_size(tauri::LogicalSize::new(width, height))
        .map_err(|e| e.to_string())
}

const AI_MODELS: &[(&str, &str)] = &[
    ("千问", "https://www.qianwen.com/"),
    ("豆包", "https://www.doubao.com/chat/"),
    ("Kimi", "https://www.kimi.com/"),
    ("智谱", "https://chatglm.cn/"),
    ("DeepSeek", "https://chat.deepseek.com/"),
];

fn ai_url_for(model: &str) -> String {
    if model.is_empty() {
        return String::from("https://www.qianwen.com/");
    }
    for (name, url) in AI_MODELS {
        if *name == model {
            return url.to_string();
        }
    }
    String::from("https://www.qianwen.com/")
}

#[tauri::command]
fn open_ai_chat(state: State<'_, AppState>, model: String, question: String) -> Result<(), String> {
    let cfg = state.config.lock().unwrap().clone();
    let base = if model == "千问" && !cfg.ai_url.trim().is_empty() {
        cfg.ai_url.clone()
    } else {
        ai_url_for(&model)
    };
    let url = if base.contains("{q}") {
        base.replace("{q}", &urlencode(&question))
    } else {
        base
    };
    let candidates = [
        std::env::var("QUARK_EXE").unwrap_or_default(),
        String::from(r"C:\Program Files\Quark\Quark.exe"),
        String::from(r"C:\Program Files (x86)\Quark\Quark.exe"),
        String::from(r"C:\Program Files\Quark\Application\Quark.exe"),
        String::from(r"C:\Program Files (x86)\Quark\Application\Quark.exe"),
    ];
    let quark = candidates
        .iter()
        .find(|p| !p.is_empty() && std::path::Path::new(p).exists());
    if let Some(exe) = quark {
        std::process::Command::new(exe)
            .arg(&url)
            .spawn()
            .map_err(|e| format!("启动夸克失败: {e}"))?;
        return Ok(());
    }
    std::process::Command::new("cmd")
        .args(["/c", "start", "", &url])
        .spawn()
        .map_err(|e| format!("打开浏览器失败: {e}"))?;
    Ok(())
}

// ---------- CDP 浏览器自动化（内置 AI 问答） ----------
const CDP_PORT: u16 = 9230;

#[derive(Serialize)]
struct AskResult {
    ok: bool,
    msg: String,
}

fn edge_path() -> Option<PathBuf> {
    [
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
    ]
    .iter()
    .map(PathBuf::from)
    .find(|p| p.exists())
}

fn launch_ai_edge(headless: bool) -> Result<(), String> {
    let Some(edge) = edge_path() else {
        return Err(String::from("未找到 Microsoft Edge"));
    };
    let local = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| String::from("."));
    let profile = format!(r"{local}\glassworkspace\edge-ai");
    let mut args = vec![
        format!("--remote-debugging-port={CDP_PORT}"),
        format!("--user-data-dir={profile}"),
        String::from("--window-size=1000,720"),
        String::from("--no-first-run"),
        String::from("--no-default-browser-check"),
    ];
    if !headless {
        args.push(String::from("--restore-last-session"));
    }
    if headless {
        args.push(String::from("--headless=new"));
    }
    let mut last_err = String::new();
    for _ in 0..3 {
        match std::process::Command::new(&edge).args(&args).spawn() {
            Ok(_) => return Ok(()),
            Err(e) => last_err = format!("启动 Edge 失败: {e}"),
        }
        std::thread::sleep(Duration::from_millis(800));
    }
    Err(last_err)
}

async fn cdp_ready(client: &reqwest::Client) -> bool {
    for _ in 0..30 {
        if client
            .get(format!("http://127.0.0.1:{CDP_PORT}/json/version"))
            .send()
            .await
            .is_ok()
        {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    false
}

type Ws = tokio_tungstenite::WebSocketStream<
    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
>;

async fn cdp_call(
    ws: &mut Ws,
    id: u64,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;
    let frame = serde_json::json!({ "id": id, "method": method, "params": params });
    ws.send(Message::Text(frame.to_string().into()))
        .await
        .map_err(|e| e.to_string())?;
    loop {
        let Some(Ok(Message::Text(t))) = ws.next().await else {
            continue;
        };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&t) else {
            continue;
        };
        if v.get("id").and_then(|i| i.as_u64()) == Some(id) {
            if let Some(err) = v.get("error") {
                return Err(err.to_string());
            }
            return Ok(v.get("result").cloned().unwrap_or(serde_json::Value::Null));
        }
    }
}

async fn wait_page_loaded(ws: &mut Ws, next_id: &mut u64) -> Result<(), String> {
    for _ in 0..50 {
        *next_id += 1;
        let r = cdp_call(
            ws,
            *next_id,
            "Runtime.evaluate",
            serde_json::json!({"expression": "document.readyState", "returnByValue": true}),
        )
        .await;
        if let Ok(v) = r {
            if v.pointer("/result/value").and_then(|x| x.as_str()) == Some("complete") {
                tokio::time::sleep(Duration::from_millis(2500)).await;
                return Ok(());
            }
        }
        tokio::time::sleep(Duration::from_millis(400)).await;
    }
    Err(String::from("页面加载超时"))
}

#[tauri::command]
async fn ask_ai_browser(model: String, question: String) -> Result<AskResult, String> {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
    use tokio_tungstenite::tungstenite::protocol::CloseFrame;

    let client = reqwest::Client::new();
    if !cdp_ready(&client).await {
        launch_ai_edge(false)?;
        if !cdp_ready(&client).await {
            return Err(String::from("浏览器启动超时"));
        }
    }

    let url = ai_url_for(&model);
    let resp = client
        .put(format!("http://127.0.0.1:{CDP_PORT}/json/new?{}", urlencode(&url)))
        .send()
        .await
        .map_err(|e| format!("创建标签页失败: {e}"))?;
    let tab: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let ws_url = tab
        .get("webSocketDebuggerUrl")
        .and_then(|v| v.as_str())
        .ok_or_else(|| String::from("无法获取调试连接"))?
        .to_string();

    let (mut ws, _) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .map_err(|e| format!("连接调试端口失败: {e}"))?;
    let mut next_id: u64 = 100;

    next_id += 1;
    cdp_call(&mut ws, next_id, "Page.enable", serde_json::json!({})).await?;
    next_id += 1;
    cdp_call(&mut ws, next_id, "Runtime.enable", serde_json::json!({})).await?;
    wait_page_loaded(&mut ws, &mut next_id).await?;

    let qstr = serde_json::to_string(&question).unwrap_or_default();
    let script = format!(
        r#"(() => {{
          const Q = {qstr};
          const box = document.querySelector('textarea') || document.querySelector('[contenteditable="true"]');
          if (!box) return {{ ok: false, msg: 'no-input' }};
          if (box.isContentEditable) {{
            box.focus();
            document.execCommand('selectAll', false, null);
            document.execCommand('insertText', false, Q);
          }} else {{
            const setter = Object.getOwnPropertyDescriptor(window.HTMLTextAreaElement.prototype, 'value')?.set;
            if (setter) setter.call(box, Q); else box.value = Q;
            box.dispatchEvent(new Event('input', {{ bubbles: true }}));
            box.dispatchEvent(new Event('change', {{ bubbles: true }}));
            box.focus();
          }}
          const btns = [...document.querySelectorAll('button')];
          const btn = btns.find(b => /发送|提问|send/i.test(b.textContent || '') && b.offsetParent !== null)
                   || btns.find(b => /发送|send/i.test(b.getAttribute('aria-label') || ''));
          if (btn) {{ btn.click(); return {{ ok: true, msg: 'button' }}; }}
          box.dispatchEvent(new KeyboardEvent('keydown', {{ key: 'Enter', code: 'Enter', keyCode: 13, which: 13, bubbles: true, cancelable: true }}));
          return {{ ok: true, msg: 'enter' }};
        }})()"#
    );

    next_id += 1;
    let res = cdp_call(
        &mut ws,
        next_id,
        "Runtime.evaluate",
        serde_json::json!({ "expression": script, "returnByValue": true }),
    )
    .await?;

    let value = res.pointer("/result/value");
    let ok = value.and_then(|v| v.get("ok")).and_then(|v| v.as_bool()).unwrap_or(false);
    let msg = value
        .and_then(|v| v.get("msg"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let _ = ws.close(Some(CloseFrame {
        code: CloseCode::Normal,
        reason: "done".into(),
    }));

    if ok {
        Ok(AskResult {
            ok: true,
            msg: format!("已在 {model} 中提问（{msg}）"),
        })
    } else if msg == "no-input" {
        Err(String::from("页面还没加载出输入框，请稍后手动输入"))
    } else {
        Err(String::from("自动提问失败"))
    }
}

// ---------- AI API 直答（OpenAI 兼容，SSE 流式） ----------
const AI_PLATFORMS: &[(&str, &str, &str)] = &[
    ("千问", "https://dashscope.aliyuncs.com/compatible-mode/v1", "qwen3.7-flash"),
    ("DeepSeek", "https://api.deepseek.com/v1", "deepseek-v4-flash"),
    ("Kimi", "https://api.moonshot.cn/v1", "kimi-k3"),
    ("智谱", "https://open.bigmodel.cn/api/paas/v4", "glm-4.7-flash"),
    ("豆包", "https://ark.cn-beijing.volces.com/api/v3", ""),
];

fn ai_base_url(model: &str) -> String {
    for (name, base, _) in AI_PLATFORMS {
        if *name == model {
            return base.to_string();
        }
    }
    String::from("https://dashscope.aliyuncs.com/compatible-mode/v1")
}

fn ai_default_model(model: &str) -> String {
    for (name, _, m) in AI_PLATFORMS {
        if *name == model && !m.is_empty() {
            return m.to_string();
        }
    }
    String::new()
}

#[derive(Clone, Serialize)]
struct AiDelta {
    delta: String,
    done: bool,
    error: Option<String>,
}

#[tauri::command]
async fn ask_ai(
    state: State<'_, AppState>,
    model: String,
    messages: Vec<serde_json::Value>,
    channel: tauri::ipc::Channel<AiDelta>,
) -> Result<(), String> {
    let cfg = state.config.lock().unwrap().clone();
    let key = cfg.ai_keys.get(&model).cloned().unwrap_or_default();
    if key.trim().is_empty() {
        return Err(String::from("missing-key"));
    }
    let api_model = cfg
        .ai_models
        .get(&model)
        .cloned()
        .filter(|m| !m.trim().is_empty())
        .unwrap_or_else(|| ai_default_model(&model));
    if api_model.trim().is_empty() {
        return Err(String::from("missing-model"));
    }

    let url = format!("{}/chat/completions", ai_base_url(&model));
    let body = serde_json::json!({
        "model": api_model.trim(),
        "messages": messages,
        "stream": true,
    });

    let resp = state
        .http
        .post(&url)
        .header("Authorization", format!("Bearer {}", key.trim()))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("请求失败: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        let msg = serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|v| {
                v.pointer("/error/message")
                    .or_else(|| v.pointer("/message"))
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| {
                text.chars().take(200).collect::<String>()
            });
        return Err(format!("API 返回 {status}: {msg}"));
    }

    use futures_util::StreamExt;
    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("读取流失败: {e}"))?;
        buf.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(pos) = buf.find("\n\n") {
            let event = buf[..pos].to_string();
            buf = buf[pos + 2..].to_string();
            for line in event.lines() {
                let Some(data) = line.strip_prefix("data:") else {
                    continue;
                };
                let data = data.trim();
                if data == "[DONE]" {
                    let _ = channel.send(AiDelta {
                        delta: String::new(),
                        done: true,
                        error: None,
                    });
                    return Ok(());
                }
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(d) = v["choices"][0]["delta"]["content"].as_str() {
                        let _ = channel.send(AiDelta {
                            delta: d.to_string(),
                            done: false,
                            error: None,
                        });
                    }
                    if let Some(err) = v["error"].as_object() {
                        let _ = channel.send(AiDelta {
                            delta: String::new(),
                            done: true,
                            error: Some(
                                err.get("message")
                                    .and_then(|m| m.as_str())
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| serde_json::to_string(err).unwrap_or_default()),
                            ),
                        });
                        return Ok(());
                    }
                }
            }
        }
    }
    let _ = channel.send(AiDelta {
        delta: String::new(),
        done: true,
        error: None,
    });
    Ok(())
}

/// 用系统默认浏览器打开 URL
#[tauri::command]
fn open_external(url: String) -> Result<(), String> {
    if url.is_empty() {
        return Ok(());
    }
    std::process::Command::new("cmd")
        .args(["/c", "start", "", &url])
        .spawn()
        .map_err(|e| format!("打开链接失败: {e}"))?;
    Ok(())
}

// ---------- 销售数据抓取（生意能手 / 来接生意） ----------
const SALES_SITES: &[(&str, &str)] = &[
    ("生意能手", "https://syzl.zhuanhua6.com"),
    ("来接生意", "https://ljsy.jywlkj.com"),
];

const SALES_TARGET: &str = "/asysmanager/xs_jifen_xiaoshou_gr.php?lm=jf&erlm=xstj";
const SALES_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/149.0.0.0 Safari/537.36";

const SALES_LOGIN: &[(&str, &str)] = &[
    ("生意能手", "https://syzl.zhuanhua6.com/asysmanager/control_loginxnew0416.php?dlcs=5f8d2a9c7e1b3064258f0d71a9c2e5b46801f3d9c5a7e2b04681f9d3c7a5e2b0"),
    ("来接生意", "https://ljsy.jywlkj.com/asysmanager/control_loginlnew1358.php?dlcs=5f8d2a9c7e1b3064253f0d71a9c2e5326801f3d1c4a7e2b04681f1d3c7a5e2b0"),
];

#[derive(Serialize, Clone)]
struct RankItem {
    rank: u32,
    name: String,
    amount: u64,
    group: String,
    follow: u32,
}

#[derive(Serialize, Clone)]
struct RechargeRecord {
    customer: String,
    time: String,
    amount: u64,
    avatar: String,
    site: String,
    note: String,
}

#[derive(Serialize, Clone)]
struct UsageItem {
    time: String,
    nickname: String,
    avatar: String,
    remain: u64,
    used: u64,
    title: String,
    site: String,
}

#[derive(Serialize, Clone)]
struct ViewItem {
    time: String,
    nickname: String,
    avatar: String,
    title: String,
    remain: u64,
    site: String,
}

#[derive(Serialize, Clone)]
struct NewFollow {
    nick: String,
    site: String,
    avatar: String,
}

#[derive(Serialize)]
struct SalesData {
    ok: bool,
    msg: String,
    login_person: String,
    self_recharge: u64,
    left_recharge: u64,
    pre_recharge: u64,
    today_follow: u32,
    today_consume: u64,
    shangji_count: u32,
    monthly_self: u64,
    monthly_left: u64,
    monthly_pre: u64,
    monthly_new_follow: u32,
    updated_at: String,
    leaderboard: Vec<RankItem>,
    recharge: std::collections::HashMap<String, Vec<RechargeRecord>>,
    usage: Vec<UsageItem>,
    views: Vec<ViewItem>,
    new_follows: Vec<NewFollow>,
    failed_sites: Vec<String>,
}

/// 尝试从销售监控（D:\销售数据监控_本机）的 cookie 文件自动读取各站点 cookie
fn auto_load_site_cookies(cfg_sites: &mut std::collections::HashMap<String, SiteCfg>) {
    let dirs = [
        String::from(r"D:\销售数据监控_本机\monitor_data\cookies"),
        std::env::var("USERPROFILE").unwrap_or_default() + r"\销售数据监控\monitor_data\cookies",
    ];
    for dir in dirs {
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        for f in rd.flatten() {
            let p = f.path();
            if p.extension().map(|e| e.to_string_lossy().to_lowercase() == "cookie").unwrap_or(false) {
                let Ok(raw) = fs::read_to_string(&p) else { continue };
                let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else { continue };
                let site = v.get("site").and_then(|x| x.as_str()).unwrap_or("").to_string();
                let cookie = v.get("cookie").and_then(|x| x.as_str()).unwrap_or("").to_string();
                if !site.is_empty() && !cookie.is_empty() {
                    let e = cfg_sites.entry(site.clone()).or_default();
                    if e.cookie.is_empty() {
                        e.cookie = cookie;
                    }
                }
            }
        }
    }
}

fn chrono_now_str() -> String {
    let now = std::time::SystemTime::now();
    let secs = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
    let days = secs / 86400;
    let (y, m, d) = civil_from_days(days as i64);
    let h = (secs % 86400) / 3600;
    let mi = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, m, d, h, mi, s)
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn html_clean(s: &str) -> String {
    regex_lite::Regex::new(r"<[^>]+>")
        .ok()
        .map(|re| re.replace_all(s, "").to_string())
        .unwrap_or_else(|| s.to_string())
        .trim()
        .to_string()
}

#[tauri::command]
async fn fetch_sales_data(state: State<'_, AppState>) -> Result<SalesData, String> {
    use regex_lite::Regex;
    let mut cfg = state.config.lock().unwrap().clone();
    auto_load_site_cookies(&mut cfg.sites);

    let mut login_person = String::new();
    let mut self_r = 0u64;
    let mut left_r = 0u64;
    let mut pre_r = 0u64;
    let mut follow_total = 0u32;
    let mut consume_total = 0u64;
    let mut shangji_total = 0u32;
    let mut monthly_self = 0u64;
    let mut monthly_left = 0u64;
    let mut monthly_pre = 0u64;
    let mut monthly_new_follow = 0u32;
    let mut recharge_by_person: std::collections::HashMap<String, Vec<RechargeRecord>> = std::collections::HashMap::new();
    let mut usage_all: Vec<UsageItem> = Vec::new();
    let mut views_all: Vec<ViewItem> = Vec::new();
    let mut new_follows_all: Vec<NewFollow> = Vec::new();
    let monitored: Vec<String> = cfg.monitored_nicks.iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    let mut name_amounts: std::collections::HashMap<String, (u64, String, u32, u32)> = std::collections::HashMap::new();
    let mut any_ok = false;
    let mut failed_sites: Vec<String> = Vec::new();

    for (name, base) in SALES_SITES {
        let cookie = cfg
            .sites
            .get(*name)
            .map(|s| s.cookie.trim().to_string())
            .unwrap_or_default();
        if cookie.is_empty() {
            failed_sites.push(name.to_string());
            continue;
        }
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{base}{SALES_TARGET}"))
            .header("User-Agent", SALES_UA)
            .header("Cookie", format!("PHPSESSID={cookie}"))
            .send()
            .await
            .map_err(|e| format!("请求 {name} 失败: {e}"))?;
        if resp.status() != reqwest::StatusCode::OK {
            failed_sites.push(name.to_string());
            continue;
        }
        let html = resp.text().await.unwrap_or_default();
        any_ok = true;

        if let Some(m) = Regex::new(r#"([\u4e00-\u9fa5]{2,4})<br>\s*<a\s+href=['"]control_loginout"#).ok().and_then(|re| re.captures(&html)) {
            if login_person.is_empty() {
                login_person = m.get(1).map(|x| x.as_str().to_string()).unwrap_or_default();
            }
        }
        let grab = |pat: &str| -> u64 {
            Regex::new(pat).ok()
                .and_then(|re| re.captures(&html))
                .and_then(|m| m.get(1))
                .and_then(|x| x.as_str().parse::<u64>().ok())
                .unwrap_or(0)
        };
        self_r += grab(r##"\$\("#allczjf"\)\.html\("(\d+)"\)"##);
        left_r += grab(r##"\$\("#allczjf_lzfp"\)\.html\("(\d+)"\)"##);
        pre_r += grab(r##"\$\("#allczjf_yfp"\)\.html\("(\d+)"\)"##);

        if let (Ok(re), Ok(re_group)) = (
            Regex::new(r">(\d+)&nbsp;([\u4e00-\u9fa5]{2,4})<br>今日充值[：:](\d*)</div>"),
            Regex::new(r"([\u4e00-\u9fa5]{2,4})：<br><a[^>]*>今日新增关注</a>[：:]*<font[^>]*>(\d+)</font>"),
        ) {
            let zh_pos = html.find("转化率：<br>今日合计");
            let jy_pos = html.find("谨言：<br>今日合计");
            let mut parse_zone = |start: usize, end: usize, group: &str| {
                let seg = &html[start.min(html.len())..end.min(html.len())];
                for cap in re.captures_iter(seg) {
                    let name_s = cap.get(2).map(|x| x.as_str().to_string()).unwrap_or_default();
                    if name_s.is_empty() { continue; }
                    let amount = cap.get(3).map(|x| x.as_str()).unwrap_or("").parse::<u64>().unwrap_or(0);
                    let rank = cap.get(1).map(|x| x.as_str()).unwrap_or("99").parse::<u32>().unwrap_or(99);
                    let e = name_amounts.entry(name_s).or_insert((0, String::from(group), 99, 0));
                    e.0 += amount;
                    e.2 = e.2.min(rank);
                }
            };
            match (zh_pos, jy_pos) {
                (Some(z), Some(j)) => {
                    parse_zone(z, j, "转化率");
                    parse_zone(j, html.len(), "谨言");
                }
                (Some(z), None) => parse_zone(z, html.len(), "转化率"),
                (None, Some(j)) => parse_zone(j, html.len(), "谨言"),
                (None, None) => parse_zone(0, html.len(), "转化率"),
            }
            for cap in re_group.captures_iter(&html) {
                let fname = cap.get(1).map(|x| x.as_str()).unwrap_or("");
                let fnum = cap.get(2).map(|x| x.as_str()).unwrap_or("0").parse::<u32>().unwrap_or(0);
                let e = name_amounts.entry(fname.to_string()).or_insert((0, String::from("转化率"), 99, 0));
                e.3 += fnum;
                if fname == login_person {
                    follow_total += fnum;
                }
            }
        }

        // 今日过商机人数 + 查看记录
        if let Ok(r2) = client
            .get(format!("{base}/asysmanager/xs_chakan_list.php?lm=tj&erlm=yckuser&page=1"))
            .header("User-Agent", SALES_UA)
            .header("Cookie", format!("PHPSESSID={cookie}"))
            .send().await
        {
            if r2.status() == reqwest::StatusCode::OK {
                if let Ok(t2) = r2.text().await {
                    if let Some(m) = Regex::new(r#"jrck3.*?\.html\(\W+(\d+)"#).ok().and_then(|re| re.captures(&t2)) {
                        shangji_total += m.get(1).map(|x| x.as_str().parse::<u32>().unwrap_or(0)).unwrap_or(0);
                    }
                    // 查看记录（两种列格式自动适配，只保留今天的）
                    let today_prefix = chrono_now_str()[..10].to_string();
                    let rows: Vec<String> = Regex::new(r"(?s)<tr[^>]*><td[^>]*>.*?</td>.*?</tr>")
                        .ok()
                        .map(|re| re.captures_iter(&t2).map(|c| c.get(0).map(|x| x.as_str().to_string()).unwrap_or_default()).collect())
                        .unwrap_or_default();
                    for row in rows {
                        let tds: Vec<String> = Regex::new(r"(?s)<td[^>]*>(.*?)</td>")
                            .ok().unwrap()
                            .captures_iter(&row)
                            .map(|c| c.get(1).map(|x| x.as_str().to_string()).unwrap_or_default())
                            .collect();
                        if tds.len() >= 11 {
                            // 序号|来源|时间|内容|头像|昵称|手机号|地区|类目|分配人|剩余积分
                            let time = html_clean(&tds[2]);
                            if !time.starts_with(&today_prefix) {
                                continue;
                            }
                            let avatar = Regex::new(r#"src=['"]([^'"]+)['"]"#).ok()
                                .and_then(|re| re.captures(&tds[4]))
                                .and_then(|m| m.get(1))
                                .map(|x| x.as_str().to_string())
                                .unwrap_or_default();
                            views_all.push(ViewItem {
                                time,
                                nickname: html_clean(&tds[5]),
                                avatar,
                                title: html_clean(&tds[3]),
                                remain: tds[10].trim().parse::<u64>().unwrap_or(0),
                                site: name.to_string(),
                            });
                        } else if tds.len() >= 9 {
                            // 序号|时间|内容|头像|昵称|地区|类目|分配人|剩余积分
                            let time = html_clean(&tds[1]);
                            if !time.starts_with(&today_prefix) {
                                continue;
                            }
                            let avatar = Regex::new(r#"src=['"]([^'"]+)['"]"#).ok()
                                .and_then(|re| re.captures(&tds[3]))
                                .and_then(|m| m.get(1))
                                .map(|x| x.as_str().to_string())
                                .unwrap_or_default();
                            views_all.push(ViewItem {
                                time,
                                nickname: html_clean(&tds[4]),
                                avatar,
                                title: html_clean(&tds[2]),
                                remain: tds[8].trim().parse::<u64>().unwrap_or(0),
                                site: name.to_string(),
                            });
                        }
                    }
                }
            }
        }

        // 本月统计：POST 同页面 + Time_select（本月1号 至 今天）
        {
            let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
            let days = secs / 86400;
            let (y, m, d) = civil_from_days(days as i64);
            let range = format!("{y}-{m}-1 0:0:0 到 {y}-{m}-{d} 0:0:0");
            if let Ok(r4) = client
                .post(format!("{base}{SALES_TARGET}"))
                .header("User-Agent", SALES_UA)
                .header("Cookie", format!("PHPSESSID={cookie}"))
                .form(&[("Time_select", range)])
                .send().await
            {
                if r4.status() == reqwest::StatusCode::OK {
                    if let Ok(t4) = r4.text().await {
                        let mgrab = |pat: &str| -> u64 {
                            Regex::new(pat).ok()
                                .and_then(|re| re.captures(&t4))
                                .and_then(|c| c.get(1))
                                .and_then(|x| x.as_str().parse::<u64>().ok())
                                .unwrap_or(0)
                        };
                        monthly_self += mgrab(r##"allczjf.*?\.html\("(\d+)"##);
                        monthly_left += mgrab(r##"allczjf_lzfp.*?\.html\("(\d+)"##);
                        monthly_pre += mgrab(r##"allczjf_yfp.*?\.html\("(\d+)"##);
                        if let Some(m) = Regex::new(r"新增关注正常使用用户数量[\s\S]*?(\d+)").ok().and_then(|re| re.captures(&t4)) {
                            monthly_new_follow += m.get(1).map(|x| x.as_str().parse::<u32>().unwrap_or(0)).unwrap_or(0);
                        }
                    }
                }
            }
        }

        // 当日充值明细：POST 今日范围（GET 页面 tfoot 为空），仅保留登录人本人
        if !login_person.is_empty() {
            let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
            let days = secs / 86400;
            let (y, m, d) = civil_from_days(days as i64);
            let next = civil_from_days(days as i64 + 1);
            let range = format!("{y}-{m}-{d} 0:0:0 到 {}-{}-{} 0:0:0", next.0, next.1, next.2);
            if let Ok(r5) = client
                .post(format!("{base}{SALES_TARGET}"))
                .header("User-Agent", SALES_UA)
                .header("Cookie", format!("PHPSESSID={cookie}"))
                .form(&[("Time_select", range)])
                .send().await
            {
                if r5.status() == reqwest::StatusCode::OK {
                    if let Ok(t5) = r5.text().await {
                        if let Ok(re_tf) = Regex::new(r"(?s)<tfoot[^>]*>(.*?)</tfoot>") {
                            for tf in re_tf.captures_iter(&t5) {
                                let tfoot_content = tf.get(1).map(|x| x.as_str()).unwrap_or("");
                                for row in Regex::new(r"(?s)<tr[^>]*>(.*?)</tr>").ok().unwrap().captures_iter(tfoot_content) {
                                    let row_content = row.get(1).map(|x| x.as_str()).unwrap_or("");
                                    let tds: Vec<String> = Regex::new(r"(?s)<td[^>]*>(.*?)</td>")
                                        .ok().unwrap()
                                        .captures_iter(row_content)
                                        .map(|c| c.get(1).map(|x| x.as_str()).unwrap_or("").to_string())
                                        .collect();
                                    if tds.len() < 7 {
                                        continue;
                                    }
                                    let salesperson = html_clean(&tds[6]);
                                    if salesperson != login_person {
                                        continue;
                                    }
                                    let amount = Regex::new(r"(\d+)").ok()
                                        .and_then(|re| re.captures(&tds[3]))
                                        .and_then(|m| m.get(1))
                                        .and_then(|x| x.as_str().parse::<u64>().ok())
                                        .unwrap_or(0);
                                    if amount == 0 {
                                        continue;
                                    }
                                    let phone = html_clean(&tds[0]);
                                    let company = html_clean(&tds[2]);
                                    let avatar = Regex::new(r#"src=['"]([^'"]+)['"]"#).ok()
                                        .and_then(|re| re.captures(&tds[1]))
                                        .and_then(|m| m.get(1))
                                        .map(|x| x.as_str().to_string())
                                        .unwrap_or_default();
                                    let note = if tds.len() > 7 { html_clean(&tds[7]) } else { String::new() };
                                    let customer = if company.chars().count() > 4 { company } else { phone };
                                    recharge_by_person.entry(salesperson).or_default().push(RechargeRecord {
                                        customer,
                                        time: html_clean(&tds[5]),
                                        amount,
                                        avatar,
                                        site: name.to_string(),
                                        note,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        // 新关注监控：扫描新用户认领页面，匹配监控昵称（td[1]=头像, td[2]=昵称）
        if !monitored.is_empty() {
            if let Ok(rn) = client
                .get(format!("{base}/asysmanager/xs_user_newuserrl.php?lm=us&erlm=xyhrl"))
                .header("User-Agent", SALES_UA)
                .header("Cookie", format!("PHPSESSID={cookie}"))
                .send().await
            {
                if rn.status() == reqwest::StatusCode::OK {
                    if let Ok(tn) = rn.text().await {
                        if let Ok(re_rows) = Regex::new(r"(?s)<tr[^>]*>(.*?)</tr>") {
                            for row in re_rows.captures_iter(&tn) {
                                let body = row.get(1).map(|x| x.as_str()).unwrap_or("");
                                let tds: Vec<String> = Regex::new(r"(?s)<td[^>]*>(.*?)</td>")
                                    .ok().unwrap()
                                    .captures_iter(body)
                                    .map(|c| c.get(1).map(|x| x.as_str()).unwrap_or("").to_string())
                                    .collect();
                                if tds.len() < 3 {
                                    continue;
                                }
                                let nick_clean = html_clean(&tds[2]);
                                if nick_clean.is_empty() || !monitored.contains(&nick_clean) {
                                    continue;
                                }
                                let avatar = Regex::new(r#"src=['"]([^'"]+)['"]"#).ok()
                                    .and_then(|re| re.captures(&tds[1]))
                                    .and_then(|m| m.get(1))
                                    .map(|x| x.as_str().to_string())
                                    .unwrap_or_default();
                                new_follows_all.push(NewFollow {
                                    nick: nick_clean,
                                    site: name.to_string(),
                                    avatar,
                                });
                            }
                        }
                    }
                }
            }
        }

        // 今日消费 + 使用记录（积分使用情况）
        if !login_person.is_empty() {
            if let Ok(r3) = client
                .get(format!("{base}/asysmanager/xs_jifen_syong.php?lm=jf&erlm=jfsy"))
                .header("User-Agent", SALES_UA)
                .header("Cookie", format!("PHPSESSID={cookie}"))
                .send().await
            {
                if r3.status() == reqwest::StatusCode::OK {
                    if let Ok(t3) = r3.text().await {
                        let pat = format!(r#"\d+&nbsp;{}<br>今日消费[：:](\d*)"#, regex_lite::escape(&login_person));
                        if let Some(m) = Regex::new(&pat).ok().and_then(|re| re.captures(&t3)) {
                            consume_total += m.get(1).map(|x| x.as_str()).unwrap_or("").parse::<u64>().unwrap_or(0);
                        }
                        let av = |td: &str| -> String {
                            Regex::new(r#"src=['"]([^'"]+)['"]"#).ok()
                                .and_then(|re| re.captures(td))
                                .and_then(|m| m.get(1))
                                .map(|x| x.as_str().to_string())
                                .unwrap_or_default()
                        };
                        let rows: Vec<String> = Regex::new(r"(?s)<tr><td>(\d+)</td>(.*?)</tr>")
                            .ok()
                            .map(|re| re.captures_iter(&t3).map(|c| c.get(2).map(|x| x.as_str().to_string()).unwrap_or_default()).collect())
                            .unwrap_or_default();
                        if t3.contains("赠送剩余积分") {
                            for body in rows {
                                let tds: Vec<String> = Regex::new(r"(?s)<td>(.*?)</td>")
                                    .ok().unwrap()
                                    .captures_iter(&body)
                                    .map(|c| c.get(1).map(|x| x.as_str().to_string()).unwrap_or_default())
                                    .collect();
                                if tds.len() >= 12 {
                                    let used = tds[8].trim().parse::<u64>().unwrap_or(0);
                                    if used == 0 { continue; }
                                    usage_all.push(UsageItem {
                                        time: html_clean(&tds[0]),
                                        nickname: html_clean(&tds[3]),
                                        avatar: av(&tds[2]),
                                        remain: tds[4].trim().parse::<u64>().unwrap_or(0),
                                        used,
                                        title: html_clean(&tds[11]),
                                        site: name.to_string(),
                                    });
                                }
                            }
                        } else {
                            for body in rows {
                                let tds: Vec<String> = Regex::new(r"(?s)<td>(.*?)</td>")
                                    .ok().unwrap()
                                    .captures_iter(&body)
                                    .map(|c| c.get(1).map(|x| x.as_str().to_string()).unwrap_or_default())
                                    .collect();
                                if tds.len() >= 10 {
                                    let used = tds[8].trim().parse::<u64>().unwrap_or(0);
                                    if used == 0 { continue; }
                                    usage_all.push(UsageItem {
                                        time: html_clean(&tds[0]),
                                        nickname: html_clean(&tds[3]),
                                        avatar: av(&tds[2]),
                                        remain: tds[5].trim().parse::<u64>().unwrap_or(0),
                                        used,
                                        title: html_clean(&tds[9]),
                                        site: name.to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if !any_ok {
        let has_cookie = cfg.sites.values().any(|s| !s.cookie.trim().is_empty());
        return Ok(SalesData {
            ok: false,
            msg: if has_cookie {
                String::from("已读取到本地 Cookie 但已失效（会话过期），请点击配置重新登录")
            } else {
                String::from("未找到站点 Cookie，请点击配置")
            },
            login_person: String::new(),
            self_recharge: 0,
            left_recharge: 0,
            pre_recharge: 0,
            today_follow: 0,
            today_consume: 0,
            shangji_count: 0,
            monthly_self: 0,
            monthly_left: 0,
            monthly_pre: 0,
            monthly_new_follow: 0,
            updated_at: String::new(),
            leaderboard: vec![],
            recharge: std::collections::HashMap::new(),
            usage: vec![],
            views: vec![],
            new_follows: vec![],
            failed_sites,
        });
    }

    let mut leaderboard: Vec<(RankItem, u32)> = name_amounts
        .iter()
        .map(|(n, (amount, group, rank, follow))| {
            (
                RankItem {
                    rank: 0,
                    name: n.clone(),
                    amount: *amount,
                    group: group.clone(),
                    follow: *follow,
                },
                *rank,
            )
        })
        .collect();
    leaderboard.sort_by(|a, b| {
        let ga = if a.0.group == "谨言" { 1 } else { 0 };
        let gb = if b.0.group == "谨言" { 1 } else { 0 };
        if ga != gb {
            ga.cmp(&gb)
        } else {
            b.0.amount.cmp(&a.0.amount).then(a.1.cmp(&b.1))
        }
    });
    let leaderboard: Vec<RankItem> = leaderboard
        .into_iter()
        .enumerate()
        .map(|(i, (mut item, _))| {
            item.rank = (i + 1) as u32;
            item
        })
        .collect();

    Ok(SalesData {
        ok: true,
        msg: String::new(),
        login_person,
        self_recharge: self_r,
        left_recharge: left_r,
        pre_recharge: pre_r,
        today_follow: follow_total,
        today_consume: consume_total,
        shangji_count: shangji_total,
        monthly_self,
        monthly_left,
        monthly_pre,
        monthly_new_follow,
        updated_at: chrono_now_str(),
        leaderboard,
        recharge: recharge_by_person,
        usage: usage_all,
        views: views_all,
        new_follows: new_follows_all,
        failed_sites,
    })
}

/// 千问视觉模型识别验证码（4 位字符）
async fn ocr_captcha(state: &State<'_, AppState>, img_base64: &str) -> Result<String, String> {
    let cfg = state.config.lock().unwrap().clone();
    let key = cfg.ai_keys.get("千问").cloned().unwrap_or_default();
    if key.trim().is_empty() {
        return Err(String::from("请先配置千问 API Key（用于识别验证码）"));
    }
    let model = String::from("qwen3-vl-flash");
    let body = serde_json::json!({
        "model": model,
        "messages": [{
            "role": "user",
            "content": [
                {"type": "text", "text": "识别图片中的验证码字符，只输出字符本身（4位字母或数字），不要任何其他内容。"},
                {"type": "image_url", "image_url": {"url": format!("data:image/png;base64,{img_base64}")}}
            ]
        }]
    });
    let resp = state
        .http
        .post("https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", key.trim()))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("识别请求失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("识别接口返回 {}", resp.status()));
    }
    let j: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let text = j["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();
    let chars: String = text
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .chars()
        .take(4)
        .collect();
    if chars.chars().count() == 4 {
        Ok(chars)
    } else {
        Err(format!("识别结果异常: {text}"))
    }
}

/// 浏览器自动化全自动登录：填账号密码 → 识别验证码 → 提交 → 捕获 PHPSESSID 保存
#[tauri::command]
async fn auto_login_site(app: AppHandle, state: State<'_, AppState>, site: String) -> Result<String, String> {
    let cfg0 = state.config.lock().unwrap().clone();
    let site_cfg = cfg0.sites.get(&site).cloned().unwrap_or_default();
    if site_cfg.username.trim().is_empty() {
        return Err(String::from("未配置账号，请先在「配置 Cookie」面板填写账号密码"));
    }
    if site_cfg.password.trim().is_empty() {
        return Err(String::from("未配置密码，请先在「配置 Cookie」面板填写账号密码"));
    }
    let username = site_cfg.username.trim().to_string();
    let password = site_cfg.password.trim().to_string();

    let Some((_, login_url)) = SALES_LOGIN.iter().find(|(n, _)| *n == site) else {
        return Err(format!("未知站点: {site}"));
    };
    let Some((_, base)) = SALES_SITES.iter().find(|(n, _)| *n == site) else {
        return Err(format!("未知站点: {site}"));
    };
    let host = base.trim_start_matches("https://").to_string();

    let client = reqwest::Client::new();
    if !cdp_ready(&client).await {
        launch_ai_edge(true)?;
        if !cdp_ready(&client).await {
            return Err(String::from("浏览器启动超时"));
        }
    }

    let resp = client
        .put(format!("http://127.0.0.1:{CDP_PORT}/json/new?{}", urlencode(login_url)))
        .send()
        .await
        .map_err(|e| format!("创建标签页失败: {e}"))?;
    let tab: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let ws_url = tab
        .get("webSocketDebuggerUrl")
        .and_then(|v| v.as_str())
        .ok_or_else(|| String::from("无法获取调试连接"))?
        .to_string();

    let (mut ws, _) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .map_err(|e| format!("连接调试端口失败: {e}"))?;
    let mut next_id: u64 = 300;
    next_id += 1;
    cdp_call(&mut ws, next_id, "Network.enable", serde_json::json!({})).await?;
    next_id += 1;
    cdp_call(&mut ws, next_id, "Page.enable", serde_json::json!({})).await?;
    next_id += 1;
    cdp_call(&mut ws, next_id, "Runtime.enable", serde_json::json!({})).await?;
    wait_page_loaded(&mut ws, &mut next_id).await?;
    tokio::time::sleep(Duration::from_millis(1500)).await;

    let u_json = serde_json::to_string(&username).unwrap_or_default();
    let p_json = serde_json::to_string(&password).unwrap_or_default();
    let fill_script = format!(
        r#"(() => {{
          const setv = (sel, v) => {{
            const el = document.querySelector(sel);
            if (!el) return false;
            const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value')?.set;
            if (setter) setter.call(el, v); else el.value = v;
            el.dispatchEvent(new Event('input', {{ bubbles: true }}));
            el.dispatchEvent(new Event('change', {{ bubbles: true }}));
            return true;
          }};
          const okU = setv('#SiteControl_LoginName', {u_json});
          const okP = setv('#SiteControl_LoginPass', {p_json});
          return okU && okP;
        }})()"#
    );

    let mut last_err = String::from("未知错误");
    for attempt in 0..5 {
        next_id += 1;
        cdp_call(
            &mut ws,
            next_id,
            "Runtime.evaluate",
            serde_json::json!({"expression": fill_script, "returnByValue": true}),
        )
        .await?;

        next_id += 1;
        let shot = cdp_call(
            &mut ws,
            next_id,
            "Runtime.evaluate",
            serde_json::json!({"expression": "(() => { const el = document.getElementById('checkpic'); if (!el) return null; const r = el.getBoundingClientRect(); return {x: r.x, y: r.y, w: r.width, h: r.height}; })()", "returnByValue": true}),
        )
        .await?;
        let (x, y, w, h) = match shot.pointer("/result/value") {
            Some(v) => (
                v.get("x").and_then(|n| n.as_f64()).unwrap_or(0.0),
                v.get("y").and_then(|n| n.as_f64()).unwrap_or(0.0),
                v.get("w").and_then(|n| n.as_f64()).unwrap_or(120.0),
                v.get("h").and_then(|n| n.as_f64()).unwrap_or(40.0),
            ),
            None => (0.0, 0.0, 120.0, 40.0),
        };
        next_id += 1;
        let cap = cdp_call(
            &mut ws,
            next_id,
            "Page.captureScreenshot",
            serde_json::json!({"format": "png", "clip": {"x": x, "y": y, "width": w, "height": h, "scale": 2}}),
        )
        .await?;
        let b64 = cap.get("data").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if b64.is_empty() {
            last_err = String::from("截图验证码失败");
            continue;
        }

        let code = match ocr_captcha(&state, &b64).await {
            Ok(c) => c,
            Err(e) => {
                last_err = e;
                next_id += 1;
                let _ = cdp_call(&mut ws, next_id, "Runtime.evaluate", serde_json::json!({"expression": "changing(); 'ok'"})).await;
                continue;
            }
        };

        let code_json = serde_json::to_string(&code).unwrap_or_default();
        let submit_script = format!(
            r#"(() => {{
              const el = document.querySelector('#Login_CheckCode');
              if (!el) return 'no-input';
              const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value')?.set;
              if (setter) setter.call(el, {code_json}); else el.value = {code_json};
              el.dispatchEvent(new Event('input', {{ bubbles: true }}));
              el.dispatchEvent(new Event('change', {{ bubbles: true }}));
              if (typeof postdata === 'function') {{ postdata(); return 'submitted-' + {code_json}; }}
              return 'no-postdata';
            }})()"#
        );
        next_id += 1;
        cdp_call(
            &mut ws,
            next_id,
            "Runtime.evaluate",
            serde_json::json!({"expression": submit_script, "returnByValue": true}),
        )
        .await?;

        let mut logged_in = false;
        for _ in 0..12 {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            next_id += 1;
            if let Ok(cur) = cdp_call(
                &mut ws,
                next_id,
                "Runtime.evaluate",
                serde_json::json!({"expression": "location.href", "returnByValue": true}),
            )
            .await
            {
                let url = cur
                    .pointer("/result/value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !url.contains("control_login") && !url.is_empty() {
                    logged_in = true;
                    break;
                }
            }
        }
        if !logged_in {
            last_err = format!("第 {} 次尝试：验证码识别可能错误，已自动重试", attempt + 1);
            next_id += 1;
            let _ = cdp_call(&mut ws, next_id, "Runtime.evaluate", serde_json::json!({"expression": "changing(); 'ok'"})).await;
            continue;
        }

        tokio::time::sleep(Duration::from_millis(800)).await;
        for _ in 0..5 {
            next_id += 1;
            if let Ok(r) = cdp_call(&mut ws, next_id, "Network.getCookies", serde_json::json!({})).await {
                if let Some(cookies) = r.get("cookies").and_then(|c| c.as_array()) {
                    for ck in cookies {
                        let name = ck.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        if name != "PHPSESSID" {
                            continue;
                        }
                        let domain = ck.get("domain").and_then(|v| v.as_str()).unwrap_or("");
                        let value = ck.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        if value.is_empty() || !domain.contains(&host) {
                            continue;
                        }
                        let vresp = client
                            .get(format!("{base}{SALES_TARGET}"))
                            .header("User-Agent", SALES_UA)
                            .header("Cookie", format!("PHPSESSID={value}"))
                            .send()
                            .await;
                        if let Ok(vr) = vresp {
                            if vr.status() == reqwest::StatusCode::OK {
                                let mut cfg = state.config.lock().unwrap();
                                let e = cfg.sites.entry(site.clone()).or_default();
                                e.base_url = base.to_string();
                                e.cookie = value.clone();
                                if let Ok(p) = config_path(&app) {
                                    if let Ok(raw) = serde_json::to_string_pretty(&*cfg) {
                                        let _ = fs::write(p, raw);
                                    }
                                }
                                return Ok(value);
                            }
                        }
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(1000)).await;
        }
        last_err = String::from("登录成功但未捕获到 Cookie");
    }
    Err(format!("自动登录失败（{last_err}）"))
}

// ---------- 屏幕级通知窗口（正上方 / 右上方） ----------
static NOTIFY_WIN: OnceLock<std::sync::Mutex<Option<tauri::WebviewWindow>>> = OnceLock::new();

/// 在电脑屏幕的指定位置弹出独立置顶通知（不依赖主窗口位置）
#[tauri::command]
async fn show_screen_notify(app: AppHandle, pos: String, title: String, lines: Vec<String>, avatars: Option<Vec<String>>, seconds: u64) -> Result<(), String> {
    use tauri::WebviewUrl;
    let holder = NOTIFY_WIN.get_or_init(|| std::sync::Mutex::new(None));
    let win = {
        let mut guard = holder.lock().unwrap();
        if let Some(w) = guard.as_ref() {
            w.clone()
        } else {
            let w = tauri::WebviewWindowBuilder::new(&app, "notify-win", WebviewUrl::App("notify.html".into()))
                .title("通知")
                .inner_size(380.0, 240.0)
                .decorations(false)
                .transparent(true)
                .always_on_top(true)
                .skip_taskbar(true)
                .resizable(false)
                .shadow(false)
                .build()
                .map_err(|e| e.to_string())?;
            *guard = Some(w.clone());
            w
        }
    };

    let (mw, mh) = match win.current_monitor() {
        Ok(Some(m)) => {
            let sz = m.size();
            (sz.width as f64, sz.height as f64)
        }
        _ => (1920.0, 1080.0),
    };
    let (ww, wh) = win.outer_size().map(|s| (s.width as f64, s.height as f64)).unwrap_or((380.0, 240.0));
    let (x, y) = if pos == "right" {
        (mw - ww - 24.0, 40.0)
    } else if pos == "left" {
        (24.0, mh - wh - 60.0)
    } else {
        ((mw - ww) / 2.0, 30.0)
    };
    win.set_position(tauri::PhysicalPosition::new(x as i32, y as i32))
        .map_err(|e| e.to_string())?;

    let data = serde_json::json!({ "title": title, "lines": lines, "avatars": avatars.unwrap_or_default() });
    let js = format!("window.__setNotify ? window.__setNotify({}) : (window.__pendingNotify = {});", data.to_string(), data.to_string());
    // 先显示窗口，再注入内容（首次创建时页面可能未加载，eval 失败不中断）
    win.show().map_err(|e| e.to_string())?;
    let mut evaled = false;
    for _ in 0..10 {
        if win.eval(&js).is_ok() {
            evaled = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    if !evaled {
        let _ = win.eval(&js);
    }
    // 强制置顶到所有置顶窗口之上（主窗口也是置顶，Win32 直接提升 z 序，不抢焦点）
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
        };
        let hwnd = win.hwnd();
        unsafe {
            if let Ok(h) = hwnd {
                let _ = SetWindowPos(h, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            }
        }
    }
    let _ = win.set_always_on_top(true);

    // seconds <= 0 表示不自动消失（只能点击关闭）
    if seconds > 0 {
        let w2 = win.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(seconds.max(3))).await;
            let _ = w2.hide();
        });
    }
    Ok(())
}

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}
fn emoji_for(name: &str) -> String {
    let n = name.to_lowercase();
    if n.contains("chrome") { return "🌐".into() }
    if n.contains("edge") { return "🧭".into() }
    if n.contains("firefox") { return "🦊".into() }
    if n.contains("wechat") || n.contains("微信") { return "💬".into() }
    if n.contains("qq") { return "🐧".into() }
    if n.contains("visual studio") || n.contains("code") { return "💻".into() }
    if n.contains("typora") { return "📝".into() }
    if n.contains("word") { return "📄".into() }
    if n.contains("excel") { return "📊".into() }
    if n.contains("powerpoint") { return "📽".into() }
    if n.contains("obsidian") || n.contains("notion") { return "📓".into() }
    if n.contains("steam") || n.contains("epic") { return "🎮".into() }
    if n.contains("photoshop") || n.contains("illustrator") { return "🎨".into() }
    if n.contains("xshell") { return "🖥".into() }
    if n.contains("terminal") || n.contains("powershell") || n.contains("cmd") { return "⌨".into() }
    if n.contains("spotify") || n.contains("music") || n.contains("网易云") || n.contains("qq音乐") { return "🎵".into() }
    if n.contains("bilibili") || n.contains("哔哩") { return "📺".into() }
    if n.contains("wps") || n.contains("office") { return "📋".into() }
    if n.contains("百度") || n.contains("baidu") { return "🔍".into() }
    if n.contains("迅雷") || n.contains("thunder") { return "⚡".into() }
    if n.contains("微信开发者") { return "🛠".into() }
    if n.contains("github") || n.contains("git") { return "🐙".into() }
    "📦".into()
}

fn scan_dir_files(dir: &std::path::Path, max_depth: usize) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![(dir.to_path_buf(), 0usize)];
    while let Some((d, depth)) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&d) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    if depth < max_depth {
                        stack.push((p, depth + 1));
                    }
                } else {
                    out.push(p);
                }
            }
        }
    }
    out
}

fn scan_registry_apps(push: &mut impl FnMut(String, String)) {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};
    use winreg::RegKey;

    let roots = [
        (
            HKEY_CURRENT_USER,
            r"Software\Microsoft\Windows\CurrentVersion\Uninstall",
        ),
        (
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        ),
        (
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        ),
    ];
    for (root, sub) in roots {
        let Ok(key) = RegKey::predef(root).open_subkey_with_flags(sub, KEY_READ) else {
            continue;
        };
        for sub_name in key.enum_keys().flatten() {
            let Ok(sk) = key.open_subkey_with_flags(&sub_name, KEY_READ) else {
                continue;
            };
            let Ok(name) = sk.get_value::<String, _>("DisplayName") else {
                continue;
            };
            let name = name.trim();
            if name.is_empty() || name.contains("更新") || name.starts_with("卸载") {
                continue;
            }
            let mut path = sk
                .get_value::<String, _>("DisplayIcon")
                .unwrap_or_default()
                .trim()
                .trim_matches('"')
                .split(',')
                .next()
                .unwrap_or("")
                .trim_matches('"')
                .to_string();
            if path.is_empty() {
                if let Ok(loc) = sk.get_value::<String, _>("InstallLocation") {
                    let loc = loc.trim();
                    if !loc.is_empty() {
                        path = format!(r"{loc}\{}.exe", name);
                    }
                }
            }
            if path.is_empty() {
                continue;
            }
            push(name.to_string(), path);
        }
    }
}

#[tauri::command]
fn scan_apps() -> Vec<AppItem> {
    let mut apps: Vec<AppItem> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut push = |name: String, path: String| {
        if !name.is_empty() && seen.insert(name.clone()) {
            apps.push(AppItem {
                name,
                path,
                emoji: String::new(),
                args: None,
            });
        }
    };

    // 1. 开始菜单快捷方式（用户 + 系统目录）
    let mut start_dirs: Vec<String> = Vec::new();
    if let Ok(ad) = std::env::var("APPDATA") {
        start_dirs.push(ad + r"\Microsoft\Windows\Start Menu\Programs");
    }
    if let Ok(pd) = std::env::var("ProgramData") {
        start_dirs.push(pd + r"\Microsoft\Windows\Start Menu\Programs");
    }
    // 1.1 桌面快捷方式（用户 + 公共桌面）
    if let Ok(dt) = std::env::var("USERPROFILE") {
        start_dirs.push(dt + r"\Desktop");
    }
    if let Ok(pd) = std::env::var("PUBLIC") {
        start_dirs.push(pd + r"\Desktop");
    }
    for dir in start_dirs {
        for f in scan_dir_files(std::path::Path::new(&dir), 3) {
            if f.extension().map(|e| e.to_string_lossy().to_lowercase() == "lnk").unwrap_or(false) {
                if let Some(stem) = f.file_stem() {
                    let name = stem.to_string_lossy().to_string();
                    if name.starts_with("卸载") || name.contains("Uninstall") || name.starts_with("删除") {
                        continue;
                    }
                    push(name, f.to_string_lossy().to_string());
                }
            }
        }
    }

    // 1.2 注册表已安装程序（覆盖 D 盘/自定义安装位置的软件，如 QQ）
    scan_registry_apps(&mut push);

    // 2. Program Files / (x86) 每个一级目录的主 exe（体积最大）
    for pf in [r"C:\Program Files", r"C:\Program Files (x86)"] {
        if let Ok(rd) = std::fs::read_dir(pf) {
            for d in rd.flatten() {
                let p = d.path();
                if !p.is_dir() {
                    continue;
                }
                let mut best: Option<(String, u64)> = None;
                if let Ok(files) = std::fs::read_dir(&p) {
                    for f in files.flatten() {
                        let fp = f.path();
                        if fp.extension().map(|e| e.to_string_lossy().to_lowercase() == "exe").unwrap_or(false) {
                            if let Ok(md) = f.metadata() {
                                if best.as_ref().map(|(_, s)| md.len() > *s).unwrap_or(true) {
                                    best = Some((fp.to_string_lossy().to_string(), md.len()));
                                }
                            }
                        }
                    }
                }
                if let Some((path, _)) = best {
                    if let Some(name) = p.file_name() {
                        push(name.to_string_lossy().to_string(), path);
                    }
                }
            }
        }
    }

    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    for app in &mut apps {
        if app.emoji.is_empty() {
            app.emoji = emoji_for(&app.name);
        }
    }
    apps
}

#[cfg(target_os = "windows")]
fn extract_icon_png(exe: &str, out: &std::path::Path) -> Result<(), String> {
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetObjectW, SelectObject,
        BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, HGDIOBJ,
    };
    use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
    use windows::Win32::UI::Shell::{
        SHFILEINFOW, SHGetFileInfoW, SHGFI_FLAGS, SHGFI_ICON, SHGFI_LARGEICON,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        DestroyIcon, DrawIconEx, GetIconInfo, DI_NORMAL, HICON, ICONINFO,
    };
    unsafe {
        let path: Vec<u16> = exe.encode_utf16().chain(std::iter::once(0)).collect();
        let mut sfi: SHFILEINFOW = std::mem::zeroed();
        SHGetFileInfoW(
            windows::core::PCWSTR(path.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut sfi as *mut _),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_FLAGS(SHGFI_ICON.0 | SHGFI_LARGEICON.0),
        );
        let icon = sfi.hIcon;
        if icon.is_invalid() {
            return Err(String::from("无图标"));
        }

        let mut ii: ICONINFO = std::mem::zeroed();
        if GetIconInfo(icon, &mut ii).is_ok() {
            let mut bm: BITMAP = std::mem::zeroed();
            GetObjectW(
                HGDIOBJ(ii.hbmColor.0),
                std::mem::size_of::<BITMAP>() as i32,
                Some(&mut bm as *mut _ as *mut _),
            );
            let (w, h) = (bm.bmWidth.max(1), bm.bmHeight.max(1));
            let dc = CreateCompatibleDC(None);
            let mut bi: BITMAPINFO = std::mem::zeroed();
            bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bi.bmiHeader.biWidth = w;
            bi.bmiHeader.biHeight = -h;
            bi.bmiHeader.biPlanes = 1;
            bi.bmiHeader.biBitCount = 32;
            bi.bmiHeader.biCompression = 0;
            let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
            let dib = match CreateDIBSection(None, &bi, DIB_RGB_COLORS, &mut bits, None, 0) {
                Ok(d) => d,
                Err(e) => {
                    DeleteDC(dc);
                    let _ = DeleteObject(HGDIOBJ(ii.hbmColor.0));
                    let _ = DeleteObject(HGDIOBJ(ii.hbmMask.0));
                    DestroyIcon(icon);
                    return Err(format!("创建位图失败: {e}"));
                }
            };
            let old = SelectObject(dc, HGDIOBJ(dib.0));
            DrawIconEx(dc, 0, 0, icon, w, h, 0, None, DI_NORMAL).ok();
            SelectObject(dc, old);
            let len = (w * h * 4) as usize;
            let src = std::slice::from_raw_parts(bits as *const u8, len);
            let mut rgba = vec![0u8; len];
            for i in 0..(w * h) as usize {
                rgba[i * 4] = src[i * 4 + 2];
                rgba[i * 4 + 1] = src[i * 4 + 1];
                rgba[i * 4 + 2] = src[i * 4];
                rgba[i * 4 + 3] = src[i * 4 + 3];
            }
            let file = std::fs::File::create(out).map_err(|e| e.to_string())?;
            let mut encoder = png::Encoder::new(file, w as u32, h as u32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
            writer.write_image_data(&rgba).map_err(|e| e.to_string())?;
            writer.finish().map_err(|e| e.to_string())?;
            let _ = DeleteObject(HGDIOBJ(dib.0));
            let _ = DeleteObject(HGDIOBJ(ii.hbmColor.0));
            let _ = DeleteObject(HGDIOBJ(ii.hbmMask.0));
            DeleteDC(dc);
            DestroyIcon(icon);
            Ok(())
        } else {
            DestroyIcon(icon);
            Err(String::from("读取图标信息失败"))
        }
    }
}

#[tauri::command]
fn get_app_icon(app: AppHandle, path: String, name: String) -> Result<String, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("icons");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let safe: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    let png_path = dir.join(format!("{safe}.png"));
    if !png_path.exists() {
        extract_icon_png(&path, &png_path).map_err(|e| format!("提取图标失败: {e}"))?;
    }
    Ok(png_path.to_string_lossy().to_string())
}

#[tauri::command]
fn save_config(app: AppHandle, state: State<'_, AppState>, cfg: Config) -> Result<(), String> {
    let path = config_path(&app)?;
    let json = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    *state.config.lock().unwrap() = cfg;
    Ok(())
}

fn toggle_window(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        match win.is_visible() {
            Ok(true) => {
                let _ = win.hide();
            }
            _ => {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn spawn_ctrl_triple_detector(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
        let mut pressed = false;
        let mut count = 0u32;
        let mut last = std::time::Instant::now();
        let mut cooldown = std::time::Instant::now();
        loop {
            std::thread::sleep(std::time::Duration::from_millis(20));
            let down = unsafe { GetAsyncKeyState(0x11) } as i32 & 0x8000 != 0;
            if down && !pressed {
                pressed = true;
                if cooldown.elapsed().as_millis() < 600 {
                    continue;
                }
                let now = std::time::Instant::now();
                if now.duration_since(last).as_millis() < 1500 {
                    count += 1;
                } else {
                    count = 1;
                }
                last = now;
                if count >= 3 {
                    count = 0;
                    cooldown = std::time::Instant::now();
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        toggle_window(&app);
                    });
                }
            } else if !down && pressed {
                pressed = false;
            }
        }
    });
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .manage(AppState::new())
        .setup(|app| {
            let cfg = load_config(&app.handle());
            *app.state::<AppState>().config.lock().unwrap() = cfg;

            use tauri::menu::{MenuBuilder, MenuItemBuilder};
            use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
            let show_i = MenuItemBuilder::with_id("show", "显示工作台").build(app)?;
            let quit_i = MenuItemBuilder::with_id("quit", "退出").build(app)?;
            let menu = MenuBuilder::new(app)
                .items(&[&show_i, &quit_i])
                .build()?;
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => toggle_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_window(tray.app_handle());
                    }
                })
                .build(app)?;

            let _window = app.get_webview_window("main");

            #[cfg(target_os = "windows")]
            spawn_ctrl_triple_detector(app.handle().clone());

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" || window.label() == "notify-win" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            get_sysinfo,
            get_temps,
            get_weather,
            get_volume,
            set_volume,
            set_window_opacity,
            set_window_size,
            set_eye_care,
            launch_app,
            launch_or_activate,
            launch_tool,
            open_config,
            reload_config,
            open_ai_chat,
            save_config,
            scan_apps,
            restore_eye_care,
            get_app_icon,
            clean_memory,
            clean_junk,
            ask_ai_browser,
            ask_ai,
            open_external,
            fetch_sales_data,
            auto_login_site,
            show_screen_notify
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                if let Some(orig) = app_handle.state::<AppState>().eye_ramp.lock().unwrap().take() {
                    let _ = gamma::set_ramp(&orig);
                }
            }
        });
}
