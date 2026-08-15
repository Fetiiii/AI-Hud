// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    configure_webkit();

    ai_hud_lib::run()
}

/// WebKitGTK-only knobs - no other platform uses that renderer. Must run
/// before the webview is created, so before `run()`.
#[cfg(target_os = "linux")]
fn configure_webkit() {
    // webkit2gtk's DMA-BUF renderer fails to allocate GBM buffers on this
    // machine, and re-enabling it kills the app immediately with
    // `Gdk-Message: Error 71 dispatching to Wayland display`. So the software
    // path stays.
    //
    // That path has its own cost - with a transparent window it does not
    // clear the previous frame - which is why popover/+page.svelte paints an
    // opaque shell over the whole window, repainting every pixel each frame.
    //
    // Not disabling compositing mode here, deliberately: it was added while
    // chasing the stacked-content bug, whose real cause turned out to be a
    // missing `core:window:allow-set-size` capability, and turning it off
    // costs the window its transparent corners. It also saves no measurable
    // memory - the GPU driver libraries are mapped by GTK, not by WebKit.
    //
    // AI_HUD_GPU=1 re-tries the DMA-BUF renderer, to check whether a driver
    // update has fixed it.
    let want_gpu = std::env::var("AI_HUD_GPU").is_ok_and(|v| v == "1");
    if !want_gpu && std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
}
