// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // webkit2gtk's DMA-BUF renderer failed to allocate GBM buffers here while
    // the HUD was still positioning itself explicitly, so we fell back to the
    // software path. That fallback has its own cost: with a transparent
    // window it does not clear the previous frame, so resizing the HUD leaves
    // the old content painted underneath the new one.
    //
    // Re-enabling it was tried and does not work here: the app dies
    // immediately with `Gdk-Message: Error 71 dispatching to Wayland
    // display`. So the software path stays, and the repaint problem is
    // handled in the frontend instead - popover/+page.svelte paints an opaque
    // shell over the whole window so every pixel is redrawn each frame.
    //
    // AI_HUD_GPU=1 re-tries the DMA-BUF renderer, for checking whether a
    // driver update has fixed it. Must be decided before the webview is
    // created, so before run().
    let want_gpu = std::env::var("AI_HUD_GPU").is_ok_and(|v| v == "1");
    if !want_gpu {
        if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
        // Deliberately NOT disabling compositing mode here. It was added
        // while chasing the stacked-content bug, whose real cause turned out
        // to be a missing `core:window:allow-set-size` capability - and
        // turning compositing off costs the window its transparent corners,
        // painting an opaque square behind each rounded edge.
    }
    ai_hud_lib::run()
}
