<script lang="ts">
  import "../app.css";
  import { snapshot } from "$lib/snapshot.svelte";

  let { children } = $props();

  snapshot.start();
</script>

{@render children()}
