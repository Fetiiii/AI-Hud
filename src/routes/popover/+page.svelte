<script lang="ts">
  import { snapshot } from "$lib/snapshot.svelte";
  import Meter from "$lib/Meter.svelte";
  import { timeUntil, formatTokens } from "$lib/format";
  import { invoke } from "@tauri-apps/api/core";

  const s = $derived(snapshot.data);

  function openDetail() {
    invoke("open_detail");
  }

  const claudeContextPct = $derived(
    s.claudeContext
      ? (s.claudeContext.tokens / s.claudeContext.contextWindow) * 100
      : null,
  );
</script>

<div class="card">
  <header data-tauri-drag-region>
    <span class="dot"></span>
    <span class="title">AI HUD</span>
    <button class="expand" onclick={openDetail} title="Detayları aç" aria-label="Detayları aç">
      ⤢
    </button>
  </header>

  <div class="body">
    <Meter
      label="Claude · 5 saatlik"
      pct={s.claudeUsage?.fiveHour?.utilizationPct ?? null}
      subtitle={s.claudeUsage?.fiveHour?.resetsAt
        ? timeUntil(s.claudeUsage.fiveHour.resetsAt)
        : "bağlı değil"}
      compact
      ondblclick={openDetail}
    />
    <Meter
      label="Claude · Haftalık"
      pct={s.claudeUsage?.sevenDay?.utilizationPct ?? null}
      subtitle={s.claudeUsage?.sevenDay?.resetsAt ? timeUntil(s.claudeUsage.sevenDay.resetsAt) : ""}
      compact
      ondblclick={openDetail}
    />
    <Meter
      label="Context (aktif oturum)"
      pct={claudeContextPct}
      subtitle={s.claudeContext
        ? `${formatTokens(s.claudeContext.tokens)} / ${formatTokens(s.claudeContext.contextWindow)} token`
        : "aktif oturum yok"}
      compact
      ondblclick={openDetail}
    />
    <Meter
      label="Codex · 5 saatlik"
      pct={s.codexUsage?.fiveHour?.utilizationPct ?? null}
      subtitle={s.codexUsage ? timeUntil(s.codexUsage.fiveHour?.resetsAt) : "yapılandırılmadı"}
      compact
      ondblclick={openDetail}
    />
  </div>

  <footer>
    <span class="hint">çift tık → detay</span>
  </footer>
</div>

<style>
  :global(html),
  :global(body) {
    background: transparent;
  }

  .card {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg);
    backdrop-filter: blur(24px) saturate(1.4);
    -webkit-backdrop-filter: blur(24px) saturate(1.4);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow);
    overflow: hidden;
  }

  header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 10px 12px 6px;
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
  }

  .title {
    font-size: 12px;
    font-weight: 650;
    flex: 1;
  }

  .expand {
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 13px;
    line-height: 1;
    padding: 3px 5px;
    border-radius: 6px;
  }

  .expand:hover {
    background: var(--surface);
    color: var(--text);
  }

  .body {
    flex: 1;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 10px;
    padding: 4px 14px 8px;
  }

  footer {
    padding: 2px 14px 10px;
  }

  .hint {
    font-size: 10px;
    color: var(--text-muted);
  }
</style>
