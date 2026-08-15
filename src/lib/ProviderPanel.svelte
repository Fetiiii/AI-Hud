<script lang="ts">
  import type { Component } from "svelte";
  import Meter from "./Meter.svelte";
  import CostBreakdown from "./CostBreakdown.svelte";
  import { timeUntil, formatTokens, windowLabel } from "./format";
  import type { ContextUsage, ProviderUsage, SessionCost } from "./types";

  let {
    title,
    icon,
    providerVar = "--accent",
    usage,
    context,
    cost = null,
    notConfiguredHint,
  }: {
    title: string;
    icon?: Component<{ size?: number }>;
    providerVar?: string;
    usage: ProviderUsage | null;
    context: ContextUsage | null;
    cost?: SessionCost | null;
    notConfiguredHint: string;
  } = $props();

  const contextPct = $derived(context ? (context.tokens / context.contextWindow) * 100 : null);
  const Icon = $derived(icon);

  function reset(window: { resetsAt: string | null } | null | undefined): string {
    return window?.resetsAt ? `sıfırlanma: ${timeUntil(window.resetsAt)}` : "";
  }
</script>

<section class="panel">
  <h2>
    {#if Icon}
      <Icon size={16} />
    {/if}
    {title}
    {#if usage?.planType}
      <span class="plan">{usage.planType}</span>
    {/if}
  </h2>

  {#if !usage && !context}
    <p class="empty">{notConfiguredHint}</p>
  {:else}
    <div class="meters">
      {#if usage?.fiveHour}
        <Meter
          label={windowLabel(usage.fiveHour, "5 saatlik limit")}
          pct={usage.fiveHour.utilizationPct}
          subtitle={reset(usage.fiveHour)}
          {providerVar}
        />
      {/if}
      {#if usage?.sevenDay}
        <Meter
          label={windowLabel(usage.sevenDay, "Haftalık limit")}
          pct={usage.sevenDay.utilizationPct}
          subtitle={reset(usage.sevenDay)}
          {providerVar}
        />
      {/if}
      {#if usage?.sevenDayOpus}
        <Meter
          label="Haftalık · Opus"
          pct={usage.sevenDayOpus.utilizationPct}
          subtitle={reset(usage.sevenDayOpus)}
          {providerVar}
        />
      {/if}
      {#if usage?.sevenDaySonnet}
        <Meter
          label="Haftalık · Sonnet"
          pct={usage.sevenDaySonnet.utilizationPct}
          subtitle={reset(usage.sevenDaySonnet)}
          {providerVar}
        />
      {/if}
      <Meter
        label="Context (aktif oturum)"
        pct={contextPct}
        subtitle={context
          ? `${formatTokens(context.tokens)} / ${formatTokens(context.contextWindow)} token${context.project ? ` · ${context.project}` : ""}`
          : "aktif oturum bulunamadı"}
        {providerVar}
      />
    </div>
    <CostBreakdown {cost} />
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
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 0 0 12px;
    font-size: 13px;
    font-weight: 640;
  }

  .plan {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
    background: var(--track);
    border-radius: 999px;
    padding: 2px 7px;
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
