<script lang="ts">
  let { size = 15 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 20 20" class="mark codex" aria-hidden="true">
  <circle cx="10" cy="10" r="10" class="bg" />
  <path
    class="glyph"
    fill-rule="evenodd"
    clip-rule="evenodd"
    d="M10 4.6a5.4 5.4 0 1 0 0 10.8 5.4 5.4 0 0 0 0-10.8Zm0 1.9a3.5 3.5 0 1 1 0 7 3.5 3.5 0 0 1 0-7Z"
  />
</svg>

<style>
  .mark {
    flex-shrink: 0;
  }

  .bg {
    fill: var(--codex-tint, #10a37f);
    opacity: 0.16;
  }

  .glyph {
    fill: var(--codex-tint, #10a37f);
  }
</style>
