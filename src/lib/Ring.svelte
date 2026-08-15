<script lang="ts">
  import { toneFor, toneColor } from "./tone";

  let {
    pct,
    size = 62,
    stroke = 5,
    providerVar,
  }: {
    pct: number | null;
    size?: number;
    stroke?: number;
    providerVar: string;
  } = $props();

  const radius = $derived((size - stroke) / 2);
  const circumference = $derived(2 * Math.PI * radius);
  const clamped = $derived(pct === null ? 0 : Math.max(0, Math.min(100, pct)));
  const offset = $derived(circumference * (1 - clamped / 100));
  const tone = $derived(toneFor(pct));
  const color = $derived(toneColor(tone, providerVar));
</script>

<div class="ring" style={`width:${size}px;height:${size}px`}>
  <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} aria-hidden="true">
    <circle
      class="bg"
      cx={size / 2}
      cy={size / 2}
      r={radius}
      fill="none"
      stroke-width={stroke}
    />
    <circle
      class="val"
      cx={size / 2}
      cy={size / 2}
      r={radius}
      fill="none"
      stroke-width={stroke}
      stroke={color}
      stroke-linecap="round"
      stroke-dasharray={circumference}
      stroke-dashoffset={pct === null ? circumference : offset}
    />
  </svg>
  <span class="pct" style={tone === "normal" ? "" : `color:${color}`}>
    {pct === null ? "—" : `%${Math.round(pct)}`}
  </span>
</div>

<style>
  .ring {
    position: relative;
    flex-shrink: 0;
  }

  svg {
    transform: rotate(-90deg);
    display: block;
  }

  .bg {
    stroke: var(--track);
  }

  .val {
    transition: stroke-dashoffset 0.55s cubic-bezier(0.32, 0.72, 0, 1);
  }

  @media (prefers-reduced-motion: reduce) {
    .val {
      transition: none;
    }
  }

  .pct {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    font-size: 15px;
    font-weight: 620;
    letter-spacing: -0.02em;
    font-variant-numeric: tabular-nums;
    color: var(--text);
  }
</style>
