<script lang="ts">
  import { formatTokens, formatUsd } from "./format";
  import type { SessionCost } from "./types";

  let { cost }: { cost: SessionCost | null } = $props();

  const rows = $derived(
    cost
      ? [
          { label: "Girdi", value: cost.tokens.input },
          { label: "Çıktı", value: cost.tokens.output },
          { label: "Cache okuma", value: cost.tokens.cacheRead },
          { label: "Cache yazma · 5 dk", value: cost.tokens.cacheWrite5m },
          { label: "Cache yazma · 1 sa", value: cost.tokens.cacheWrite1h },
        ].filter((r) => r.value > 0)
      : [],
  );

  const total = $derived(rows.reduce((sum, r) => sum + r.value, 0));
</script>

{#if cost}
  <div class="cost">
    <div class="headline">
      <span class="label">
        Bu oturum
        {#if cost.partial}<span class="warn" title="Fiyatı bilinmeyen model var">en az</span>{/if}
      </span>
      <span class="usd">{formatUsd(cost.usd)}</span>
    </div>

    <div class="rows">
      {#each rows as row (row.label)}
        <div class="row">
          <span>{row.label}</span>
          <span class="num">{formatTokens(row.value)}</span>
        </div>
      {/each}
      <div class="row total">
        <span>Toplam</span>
        <span class="num">{formatTokens(total)} token</span>
      </div>
    </div>

    {#if cost.models.length > 1}
      <div class="models">
        {#each cost.models as m (m.model)}
          <div class="row">
            <span class="model">{m.model}</span>
            <span class="num">{m.usd === null ? "fiyatı yok" : formatUsd(m.usd)}</span>
          </div>
        {/each}
      </div>
    {/if}

    <p class="note">API liste fiyatlarına göre tahmin — aboneliğinde bu tutar tahsil edilmez.</p>
  </div>
{/if}

<style>
  .cost {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-top: 10px;
    margin-top: 2px;
    border-top: 1px solid var(--border);
  }

  .headline {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }

  .label {
    font-size: 12px;
    font-weight: 560;
  }

  .warn {
    font-size: 10px;
    font-weight: 600;
    color: var(--warn);
    margin-left: 4px;
  }

  .usd {
    font-size: 16px;
    font-weight: 620;
    letter-spacing: -0.02em;
    font-variant-numeric: tabular-nums;
  }

  .rows,
  .models {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .models {
    padding-top: 7px;
    border-top: 1px solid var(--border);
  }

  .row {
    display: flex;
    justify-content: space-between;
    font-size: 10.5px;
    color: var(--text-muted);
  }

  .row.total {
    padding-top: 3px;
    color: var(--text);
    font-weight: 560;
  }

  .num {
    font-variant-numeric: tabular-nums;
  }

  .model {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 62%;
  }

  .note {
    margin: 0;
    font-size: 9.5px;
    line-height: 1.4;
    color: var(--text-muted);
    opacity: 0.8;
  }
</style>
