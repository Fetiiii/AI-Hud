<script lang="ts">
  import { onMount, tick } from "svelte";
  import { snapshot } from "$lib/snapshot.svelte";
  import ProviderTile from "$lib/ProviderTile.svelte";
  import ProviderPanel from "$lib/ProviderPanel.svelte";
  import ClaudeMark from "$lib/icons/ClaudeMark.svelte";
  import CodexMark from "$lib/icons/CodexMark.svelte";
  import { toneFor, toneColor } from "$lib/tone";
  import { formatUpdatedAt } from "$lib/format";
  import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";

  const s = $derived(snapshot.data);

  /** One window, three sizes — the detail view grows this HUD rather than
   *  opening a second, system-decorated window beside it. */
  type Mode = "mini" | "normal" | "detail";
  let mode = $state<Mode>("normal");
  let card = $state<HTMLElement | null>(null);

  /**
   * One width for every mode, deliberately.
   *
   * A window grows from its top-left, and Wayland gives a client no way to
   * compensate: `outerPosition()` reports 0,0 and `setPosition()` is ignored
   * outright (measured on this compositor), because placement is the
   * compositor's alone. So a HUD parked against the right edge that widened
   * on expand would push its own content off the display with no way to
   * correct for it. Keeping the width fixed sidesteps the problem entirely -
   * expanding only ever grows downward, and the HUD stays where it was put.
   */
  const WIDTH = 320;
  /** .shell's 1px top + bottom border, which sits outside .card's box. */
  const SHELL_BORDER_Y = 2;


  /** A dblclick is preceded by a click, so a bare click handler would toggle
   *  on the way to opening detail. Hold the single-click action just long
   *  enough to see whether a second click lands. */
  const DOUBLE_CLICK_GRACE_MS = 230;
  let pendingClick: ReturnType<typeof setTimeout> | null = null;

  function cancelPending() {
    if (pendingClick) {
      clearTimeout(pendingClick);
      pendingClick = null;
    }
  }

  function isInteractive(event: MouseEvent) {
    return !!(event.target as HTMLElement).closest("button");
  }

  function onClick(event: MouseEvent) {
    if (isInteractive(event) || event.detail > 1) return;
    cancelPending();
    pendingClick = setTimeout(() => {
      pendingClick = null;
      mode = mode === "mini" ? "normal" : mode === "normal" ? "mini" : "normal";
    }, DOUBLE_CLICK_GRACE_MS);
  }

  function onDoubleClick(event: MouseEvent) {
    if (isInteractive(event)) return;
    cancelPending();
    mode = mode === "detail" ? "normal" : "detail";
  }

  /** Last size we asked the OS for, so a re-measure that lands on the same
   *  numbers doesn't trigger another resize (and another re-measure). */
  let applied = { w: 0, h: 0 };
  let settling = false;
  /** A mode change that arrived mid-resize; run it once the current one ends
   *  rather than dropping it (clicking fast would otherwise strand the HUD at
   *  the wrong size). */
  let queued: Mode | null = null;

  /**
   * Resize, then pin the window to exactly that size by making its minimum
   * and maximum the same.
   *
   * Fighting the window manager after the fact does not work: watching for
   * an unexpected resize and setting the size back just produced an endless
   * `2560x1400 -> 320x259 -> 2560x1400` loop, because KWin re-applied the
   * maximise every time. Equal min and max instead make the state
   * unreachable - the window manager has no size it is allowed to give the
   * window, so edge-tiling, maximising, and dragging a border all become
   * no-ops, while our own resize still works because we move the constraints
   * with it.
   *
   * Order matters: the constraints have to be released before a size outside
   * them can be applied, and re-applied after.
   */
  async function pinSizeTo(
    win: ReturnType<typeof getCurrentWindow>,
    width: number,
    height: number,
  ) {
    await win.setMinSize(null);
    await win.setMaxSize(null);
    await win.setSize(new LogicalSize(width, height));
    await win.setMinSize(new LogicalSize(width, height));
    await win.setMaxSize(new LogicalSize(width, height));
  }

  /**
   * Resize in one step, at the right moment.
   *
   * NOTE: every command used here needs a matching entry in
   * `src-tauri/capabilities/default.json`. A missing permission is rejected
   * at the IPC boundary as a promise rejection, so an un-awaited `setSize`
   * fails *silently* - the window simply never moves, with nothing in any
   * log. That is exactly what happened with `core:window:allow-set-size`,
   * and it masqueraded as a rendering bug for several rounds.
   *
   * Let the DOM swap and lay out before measuring, so the height read is the
   * one the new content actually needs. Only the height varies — the width is
   * fixed for every mode — so this is a single resize with nothing to
   * re-measure afterwards.
   */
  async function settleSize(target: Mode) {
    if (settling) {
      queued = target;
      return;
    }
    settling = true;
    try {
      const win = getCurrentWindow();
      const width = WIDTH;

      await tick(); // Svelte swaps the content
      await new Promise((r) => requestAnimationFrame(() => r(null))); // browser lays it out
      if (!card) return;

      // .card sits inside .shell, whose 1px border is outside the measurement.
      const height = Math.ceil(card.getBoundingClientRect().height) + SHELL_BORDER_Y;
      if (height <= SHELL_BORDER_Y) return;
      if (applied.w === width && applied.h === height) return;

      applied = { w: width, h: height };
      await pinSizeTo(win, width, height);
    } finally {
      settling = false;
      if (queued !== null) {
        const next = queued;
        queued = null;
        settleSize(next);
      }
    }
  }

  // Re-measure when the mode changes, and when new data changes how tall the
  // content is (a longer reset string, a provider coming online).
  $effect(() => {
    const target = mode;
    void s.updatedAtMs;
    void snapshot.refreshing;
    settleSize(target);
  });

  // No hide-on-blur. This is a HUD, not a menu-bar popover: it is meant to
  // sit on screen showing the numbers, so vanishing the moment the user
  // clicks their editor - and having to fetch it back from the tray - fights
  // its whole purpose. Hiding is the tray menu's job.
  onMount(() => cancelPending);

  const claudeMain = $derived(s.claudeUsage?.fiveHour ?? null);
  const codexMain = $derived(s.codexUsage?.fiveHour ?? s.codexUsage?.sevenDay ?? null);

  const miniUnits = $derived([
    { key: "claude", pct: claudeMain?.utilizationPct ?? null, icon: ClaudeMark, v: "--claude-tint" },
    { key: "codex", pct: codexMain?.utilizationPct ?? null, icon: CodexMark, v: "--codex-tint" },
  ]);
</script>

<svelte:body onclick={onClick} ondblclick={onDoubleClick} />

<!-- .shell fills the whole window and paints it; .card is what gets measured.
     See the styles below for why the two must be separate. -->
<div class="shell">
<div class="card" bind:this={card}>
  <header data-tauri-drag-region="deep">
    <span class="grip" aria-hidden="true"><i></i><i></i><i></i></span>
    <span class="title">AI HUD</span>
    {#if mode !== "mini"}
      <button
        class="grow"
        onclick={() => (mode = mode === "detail" ? "normal" : "detail")}
        title={mode === "detail" ? "Küçült" : "Detayları aç"}
        aria-label={mode === "detail" ? "Küçült" : "Detayları aç"}
      >
        {mode === "detail" ? "⤡" : "⤢"}
      </button>
    {/if}
  </header>

  {#if mode === "mini"}
    <div class="mini">
      {#each miniUnits as unit (unit.key)}
        {@const Icon = unit.icon}
        {@const tone = toneFor(unit.pct)}
        <span class="unit">
          <Icon size={13} />
          <span
            class="num"
            style={tone === "warn" || tone === "critical" ? `color:${toneColor(tone, unit.v)}` : ""}
          >
            {unit.pct === null ? "—" : `%${Math.round(unit.pct)}`}
          </span>
        </span>
      {/each}
    </div>
  {:else if mode === "normal"}
    <div class="tiles">
      <ProviderTile
        name="Claude"
        icon={ClaudeMark}
        providerVar="--claude-tint"
        usage={s.claudeUsage}
        context={s.claudeContext}
        fallbackWindowName="5 saatlik"
        emptyHint="bağlı değil"
      />
      <ProviderTile
        name="Codex"
        icon={CodexMark}
        providerVar="--codex-tint"
        usage={s.codexUsage}
        context={s.codexContext}
        fallbackWindowName="5 saatlik"
        emptyHint="bağlı değil"
      />
    </div>
    <footer>
      <span>tek tık · küçült</span>
      <span>çift tık · detay</span>
    </footer>
  {:else}
    <div class="detail">
      <ProviderPanel
        title="Claude Code"
        icon={ClaudeMark}
        providerVar="--claude-tint"
        usage={s.claudeUsage}
        context={s.claudeContext}
        cost={s.claudeCost}
        notConfiguredHint="Claude Code girişi bulunamadı. `claude` komutunu bir kez çalıştırıp giriş yapman yeterli."
      />
      <ProviderPanel
        title="Codex"
        icon={CodexMark}
        providerVar="--codex-tint"
        usage={s.codexUsage}
        context={s.codexContext}
        cost={s.codexCost}
        notConfiguredHint="Codex girişi bulunamadı. `codex` komutunu bir kez çalıştırıp giriş yapman yeterli."
      />

      {#if s.errors.length > 0}
        <section class="notes">
          <h3>Notlar</h3>
          <ul>
            {#each s.errors as err (err)}
              <li>{err}</li>
            {/each}
          </ul>
        </section>
      {/if}

      <div class="detail-foot">
        <span>{formatUpdatedAt(s.updatedAtMs)}</span>
        <button class="refresh" onclick={() => snapshot.refresh()} disabled={snapshot.refreshing}>
          {snapshot.refreshing ? "Yenileniyor…" : "Yenile"}
        </button>
      </div>
    </div>
  {/if}
</div>
</div>

<style>
  :global(html),
  :global(body) {
    background: transparent;
    overflow: hidden;
    height: 100%;
  }

  /*
   * No backdrop-filter here, deliberately.
   *
   * The DMA-BUF renderer crashes on this machine (Gdk Error 71), so the
   * webview runs on webkit2gtk's software path. There, backdrop-filter on a
   * transparent window samples the window's own *previous frame* as its
   * backdrop — so every resize blended the old mode's content into the new
   * one and all three states appeared stacked together. It also painted a
   * dark square in each rounded corner, because the filtered region is a
   * rectangle and the corner falls outside the radius.
   *
   * It was never buying anything either: backdrop-filter can only blur
   * content inside the page, never the desktop behind the window, and there
   * is no in-page content behind this panel. So it costs the bug and returns
   * nothing. `--bg` carries the panel's own colour instead.
   *
   * `.shell` still fills the whole window and paints every pixel of it, so
   * nothing stale can survive a resize. It must stay a separate element from
   * `.card`: `.card` is what gets measured to decide the window height, and
   * giving *it* `height: 100%` would make the measurement depend on the size
   * it is trying to produce.
   */
  .shell {
    height: 100%;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    overflow: hidden;
    box-sizing: border-box;
    /* No CSS box-shadow. The window is sized to exactly this element, so a
       shadow has no room to fall outside it — all it does is darken the
       transparent pixels in the rounded corners, which reads as four
       off-colour patches against the wallpaper. The window's drop shadow is
       the compositor's job (`"shadow": true` in tauri.conf.json). */
  }

  .card {
    display: flex;
    flex-direction: column;
    cursor: default;
  }

  header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 9px 13px 5px;
    cursor: grab;
    flex-shrink: 0;
  }

  header:active {
    cursor: grabbing;
  }

  .grip {
    display: flex;
    gap: 3px;
    opacity: 0.4;
  }

  .grip i {
    width: 3px;
    height: 3px;
    border-radius: 50%;
    background: var(--text);
  }

  .title {
    flex: 1;
    font-size: 11.5px;
    font-weight: 600;
    color: var(--text-muted);
  }

  .grow {
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 12px;
    line-height: 1;
    padding: 3px 5px;
    border-radius: 6px;
  }

  .grow:hover {
    background: var(--surface);
    color: var(--text);
  }

  .tiles {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 9px;
    padding: 2px 13px 0;
  }

  footer {
    display: flex;
    justify-content: space-between;
    padding: 9px 15px 11px;
    font-size: 9.5px;
    color: var(--text-muted);
    opacity: 0.75;
  }

  .mini {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 18px;
    padding: 1px 16px 13px;
  }

  .unit {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .num {
    font-size: 19px;
    font-weight: 360;
    letter-spacing: -0.03em;
    font-variant-numeric: tabular-nums;
    color: var(--text);
  }

  .detail {
    display: flex;
    flex-direction: column;
    gap: 11px;
    padding: 3px 14px 13px;
  }

  .notes {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: 10px 14px;
  }

  .notes h3 {
    margin: 0 0 5px;
    font-size: 11px;
    font-weight: 620;
    color: var(--text-muted);
  }

  .notes ul {
    margin: 0;
    padding-left: 15px;
    font-size: 11px;
    color: var(--text-muted);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .detail-foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 10.5px;
    color: var(--text-muted);
  }

  .refresh {
    background: var(--surface);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 4px 11px;
    font-size: 11px;
    font-weight: 560;
    transition: opacity 0.15s ease;
  }

  .refresh:hover:not(:disabled) {
    background: var(--track);
  }

  .refresh:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
