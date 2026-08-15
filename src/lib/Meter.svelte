<script lang="ts">
  import { toneFor, toneColor } from "./tone";

  let {
    label,
    pct,
    subtitle = "",
    providerVar = "--accent",
  }: {
    label: string;
    pct: number | null;
    subtitle?: string;
    providerVar?: string;
  } = $props();

  const clamped = $derived(pct === null ? 0 : Math.max(0, Math.min(100, pct)));
  const tone = $derived(toneFor(pct));
  const color = $derived(toneColor(tone, providerVar));
</script>

<div class="meter">
  <div class="head">
    <span class="label">{label}</span>
    <span class="value" style={tone === "normal" || tone === "unknown" ? "" : `color:${color}`}>
      {pct === null ? "—" : `%${Math.round(pct)}`}
    </span>
  </div>
  <div class="track">
    <div
      class="fill"
      class:idle={pct === null}
      style={`width:${pct === null ? 100 : clamped}%;background:${color}`}
    ></div>
  </div>
  {#if subtitle}
    <div class="subtitle">{subtitle}</div>
  {/if}
</div>

<style>
  .meter {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }

  .label {
    font-size: 12px;
    font-weight: 560;
    color: var(--text);
  }

  .value {
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    font-weight: 620;
    color: var(--text);
  }

  .track {
    height: 4px;
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

  .subtitle {
    font-size: 10.5px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }
</style>
