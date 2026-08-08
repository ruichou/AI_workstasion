#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // 关闭 GPU 硬件加速，解决透明/毛玻璃窗口在部分显卡驱动上 WebView2 GPU 进程崩溃闪退。
    // 附加到已有变量之后，不覆盖开发模式注入的 --remote-debugging-port。
    let existing = std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").unwrap_or_default();
    let mut args = existing.split_whitespace().collect::<Vec<_>>();
    if !args.iter().any(|a| *a == "--disable-gpu") {
        args.push("--disable-gpu");
        args.push("--disable-gpu-compositing");
    }
    unsafe {
        std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", args.join(" "));
    }
    floating_workspace::run()
}