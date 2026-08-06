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

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
struct Config {
    city: String,
    lat: Option<f64>,
    lon: Option<f64>,
    ai_url: String,
    off_time: String,
    apps: Vec<AppItem>,
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
            fs::write(&path, serde_json::to_string_pretty(&Config::default()).unwrap()).ok();
            Config::default()
        }
    }
}

#[tauri::command]
fn get_config(state: State<'_, AppState>) -> Config {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
async fn get_sysinfo() -> SysInfo {
    let mut sys = System::new_all();
    sys.refresh_memory();
    sys.refresh_cpu_usage();
    tokio::time::sleep(Duration::from_millis(250)).await;
    sys.refresh_cpu_usage();

    let disks = Disks::new_with_refreshed_list();
    let c_disk = disks
        .iter()
        .find(|d| d.mount_point().to_string_lossy().to_uppercase().starts_with("C:"))
        .or_else(|| disks.iter().max_by_key(|d| d.total_space()));
    let (disk_total, disk_used) = match c_disk {
        Some(d) => (d.total_space(), d.total_space().saturating_sub(d.available_space())),
        None => (0, 0),
    };

    SysInfo {
        cpu: sys.global_cpu_usage(),
        mem_used: sys.used_memory(),
        mem_total: sys.total_memory(),
        disk_used,
        disk_total,
    }
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

#[tauri::command]
async fn get_temps() -> Result<Temps, String> {
    let cpu = tauri::async_runtime::spawn_blocking(|| temps::cpu_temp_celsius())
        .await
        .ok()
        .flatten();
    let disk = tauri::async_runtime::spawn_blocking(|| temps::disk_temp_celsius())
        .await
        .ok()
        .flatten();
    Ok(Temps { cpu, disk })
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
                if window.label() == "main" {
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
            launch_tool,
            open_config,
            reload_config,
            open_ai_chat,
            save_config,
            scan_apps,
            restore_eye_care,
            get_app_icon
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
