// Tauri doesn't have a Node.js server to do proper SSR
// so we use adapter-static with a fallback to index.html to put the site in SPA mode
// See: https://svelte.dev/docs/kit/single-page-apps
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
export const ssr = false;
export const prerender = true;
// Tauri's asset protocol resolves "/popover" as a directory (index.html
// inside it), not a flat "popover.html" file - match that here so each
// window's `url` in tauri.conf.json actually resolves in production builds.
export const trailingSlash = "always";
