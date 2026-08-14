<script lang="ts">
  import Meter from "./Meter.svelte";
  import { timeUntil, formatTokens } from "./format";
  import type { ContextUsage, ProviderUsage } from "./types";

  let {
    title,
    usage,
    context,
    notConfiguredHint,
  }: {
    title: string;
    usage: ProviderUsage | null;
    context: ContextUsage | null;
    notConfiguredHint: string;
  } = $props();

  const contextPct = $derived(context ? (context.tokens / context.contextWindow) * 100 : null);
</script>

<section class="panel">
  <h2>{title}</h2>

  {#if !usage && !context}
    <p class="empty">{notConfiguredHint}</p>
  {:else}
    <div class="meters">
      <Meter
        label="5 saatlik limit"
        pct={usage?.fiveHour?.utilizationPct ?? null}
        subtitle={usage?.fiveHour?.resetsAt ? `sıfırlanma: ${timeUntil(usage.fiveHour.resetsAt)}` : ""}
      />
      <Meter
        label="Haftalık limit"
        pct={usage?.sevenDay?.utilizationPct ?? null}
        subtitle={usage?.sevenDay?.resetsAt ? `sıfırlanma: ${timeUntil(usage.sevenDay.resetsAt)}` : ""}
      />
      {#if usage?.sevenDayOpus}
        <Meter
          label="Haftalık · Opus"
          pct={usage.sevenDayOpus.utilizationPct}
          subtitle={usage.sevenDayOpus.resetsAt ? `sıfırlanma: ${timeUntil(usage.sevenDayOpus.resetsAt)}` : ""}
        />
      {/if}
      {#if usage?.sevenDaySonnet}
        <Meter
          label="Haftalık · Sonnet"
          pct={usage.sevenDaySonnet.utilizationPct}
          subtitle={usage.sevenDaySonnet.resetsAt ? `sıfırlanma: ${timeUntil(usage.sevenDaySonnet.resetsAt)}` : ""}
        />
      {/if}
      <Meter
        label="Context (aktif oturum)"
        pct={contextPct}
        subtitle={context
          ? `${formatTokens(context.tokens)} / ${formatTokens(context.contextWindow)} token${context.project ? ` · ${context.project}` : ""}`
          : "aktif oturum bulunamadı"}
      />
    </div>
  {/if}
</section>

<style>
  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: 14px 16px;
  }

  h2 {
    margin: 0 0 10px;
    font-size: 13px;
    font-weight: 650;
  }

  .meters {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .empty {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
  }
</style>
