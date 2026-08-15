<script lang="ts">
  import type { Component } from "svelte";
  import Ring from "./Ring.svelte";
  import { timeUntil, formatTokens, windowLabel } from "./format";
  import { toneFor, toneColor } from "./tone";
  import type { ContextUsage, ProviderUsage } from "./types";

  let {
    name,
    icon,
    providerVar,
    usage,
    context,
    fallbackWindowName,
    emptyHint,
  }: {
    name: string;
    icon: Component<{ size?: number }>;
    providerVar: string;
    usage: ProviderUsage | null;
    context: ContextUsage | null;
    /** Used only when the provider reports no window duration of its own. */
    fallbackWindowName: string;
    emptyHint: string;
  } = $props();

  /** The window the ring shows: the primary one when present, else whatever
   *  the provider does report. On a free Codex plan the only window is
   *  monthly and lands in the primary slot; on Claude it's the 5-hour one. */
  const main = $derived(usage?.fiveHour ?? usage?.sevenDay ?? null);
  const contextPct = $derived(
    context ? (context.tokens / context.contextWindow) * 100 : null,
  );
  const contextTone = $derived(toneFor(contextPct));
  const contextColor = $derived(toneColor(contextTone, providerVar));
  const Icon = $derived(icon);
</script>

<div class="tile">
  <div class="who">
    <Icon size={13} />
    <span>{name}</span>
  </div>

  {#if main}
    <Ring pct={main.utilizationPct} {providerVar} />
    <div class="meta">
      <span class="window">{windowLabel(main, fallbackWindowName)}</span>
      {#if main.resetsAt}
        <span class="reset">{timeUntil(main.resetsAt)}</span>
      {/if}
    </div>
  {:else}
    <Ring pct={null} {providerVar} />
    <div class="meta">
      <span class="window muted">{emptyHint}</span>
    </div>
  {/if}

  <div class="ctx">
    <div class="ctx-head">
      <span>Context</span>
      <span class="ctx-val" style={contextTone === "unknown" ? "" : `color:${contextColor}`}>
        {contextPct === null ? "—" : `%${Math.round(contextPct)}`}
      </span>
    </div>
    <div class="track">
      <div
        class="fill"
        class:idle={contextPct === null}
        style={`width:${contextPct === null ? 100 : Math.min(100, contextPct)}%;background:${contextColor}`}
      ></div>
    </div>
    <span class="ctx-sub">
      {context
        ? `${formatTokens(context.tokens)} / ${formatTokens(context.contextWindow)}`
        : "oturum yok"}
    </span>
  </div>
</div>

<style>
  .tile {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 12px 10px 11px;
    background: var(--surface);
    border-radius: 13px;
    min-width: 0;
  }

  .who {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    font-weight: 620;
    color: var(--text-muted);
  }

  .meta {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1px;
    text-align: center;
    min-height: 26px;
  }

  .window {
    font-size: 11px;
    font-weight: 560;
    color: var(--text);
  }

  .window.muted {
    color: var(--text-muted);
    font-weight: 500;
  }

  .reset {
    font-size: 10px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .ctx {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding-top: 8px;
    border-top: 1px solid var(--border);
  }

  .ctx-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    font-size: 10.5px;
    color: var(--text-muted);
  }

  .ctx-val {
    font-weight: 620;
    font-variant-numeric: tabular-nums;
    color: var(--text);
  }

  .track {
    height: 3px;
    border-radius: 999px;
    background: var(--track);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    border-radius: 999px;
    transition: width 0.5s cubic-bezier(0.32, 0.72, 0, 1);
  }

  .fill.idle {
    opacity: 0.25;
  }

  @media (prefers-reduced-motion: reduce) {
    .fill {
      transition: none;
    }
  }

  .ctx-sub {
    font-size: 10px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }
</style>
