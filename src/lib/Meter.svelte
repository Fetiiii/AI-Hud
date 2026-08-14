<script lang="ts">
  let {
    label,
    pct,
    subtitle = "",
    compact = false,
    ondblclick,
  }: {
    label: string;
    pct: number | null;
    subtitle?: string;
    compact?: boolean;
    ondblclick?: () => void;
  } = $props();

  const clamped = $derived(pct === null ? 0 : Math.max(0, Math.min(100, pct)));
  const tone = $derived(
    pct === null ? "unknown" : clamped >= 90 ? "danger" : clamped >= 70 ? "warn" : "ok",
  );
</script>

<div
  class="meter"
  class:compact
  class:clickable={!!ondblclick}
  role="button"
  tabindex="0"
  ondblclick={ondblclick ?? (() => {})}
>
  <div class="head">
    <span class="label">{label}</span>
    <span class="value" class:tone-danger={tone === "danger"} class:tone-warn={tone === "warn"}>
      {pct === null ? "—" : `%${Math.round(pct)}`}
    </span>
  </div>
  <div class="track">
    <div
      class="fill"
      class:tone-ok={tone === "ok"}
      class:tone-warn={tone === "warn"}
      class:tone-danger={tone === "danger"}
      class:tone-unknown={tone === "unknown"}
      style={`width: ${tone === "unknown" ? 100 : clamped}%`}
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
    gap: 4px;
    padding: 2px 0;
    border-radius: var(--radius-sm);
  }

  .meter.clickable {
    cursor: pointer;
  }

  .head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }

  .label {
    font-size: 12px;
    font-weight: 590;
    color: var(--text);
  }

  .value {
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    color: var(--text-muted);
    font-weight: 590;
  }

  .value.tone-warn {
    color: var(--warn);
  }

  .value.tone-danger {
    color: var(--danger);
  }

  .track {
    height: 6px;
    border-radius: 999px;
    background: var(--track);
    overflow: hidden;
  }

  .compact .track {
    height: 5px;
  }

  .fill {
    height: 100%;
    border-radius: 999px;
    transition: width 0.4s ease;
  }

  .fill.tone-ok {
    background: var(--ok);
  }

  .fill.tone-warn {
    background: var(--warn);
  }

  .fill.tone-danger {
    background: var(--danger);
  }

  .fill.tone-unknown {
    background: repeating-linear-gradient(
      45deg,
      var(--track),
      var(--track) 6px,
      transparent 6px,
      transparent 12px
    );
  }

  .subtitle {
    font-size: 10.5px;
    color: var(--text-muted);
  }
</style>
