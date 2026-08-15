<script lang="ts">
  let { size = 15 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 20 20" class="mark claude" aria-hidden="true">
  <circle cx="10" cy="10" r="10" class="bg" />
  <path
    class="glyph"
    d="M10 4.2 L11.35 8.6 L15.8 10 L11.35 11.4 L10 15.8 L8.65 11.4 L4.2 10 L8.65 8.6 Z"
  />
</svg>

<style>
  .mark {
    flex-shrink: 0;
  }

  .bg {
    fill: var(--claude-tint, #d97757);
    opacity: 0.16;
  }

  .glyph {
    fill: var(--claude-tint, #d97757);
  }
</style>
