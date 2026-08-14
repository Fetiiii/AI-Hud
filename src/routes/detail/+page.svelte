<script lang="ts">
  import { snapshot } from "$lib/snapshot.svelte";
  import ProviderPanel from "$lib/ProviderPanel.svelte";
  import { formatUpdatedAt } from "$lib/format";

  const s = $derived(snapshot.data);
</script>

<div class="page">
  <header>
    <h1>AI HUD</h1>
    <button class="refresh" onclick={() => snapshot.refresh()} disabled={snapshot.refreshing}>
      {snapshot.refreshing ? "Yenileniyor…" : "Yenile"}
    </button>
  </header>

  <div class="panels">
    <ProviderPanel
      title="Claude Code"
      usage={s.claudeUsage}
      context={s.claudeContext}
      notConfiguredHint="Claude Code girişi bulunamadı. `claude` komutunu bir kez çalıştırıp giriş yapman yeterli."
    />
    <ProviderPanel
      title="Codex"
      usage={s.codexUsage}
      context={s.codexContext}
      notConfiguredHint="Codex girişi bulunamadı ya da kullanım uç noktası bu hesap için henüz doğrulanmadı."
    />
  </div>

  {#if s.errors.length > 0}
    <section class="errors">
      <h2>Notlar</h2>
      <ul>
        {#each s.errors as err (err)}
          <li>{err}</li>
        {/each}
      </ul>
    </section>
  {/if}

  <footer>
    <span>{formatUpdatedAt(s.updatedAtMs)}</span>
  </footer>
</div>

<style>
  .page {
    min-height: 100vh;
    background: var(--surface-solid);
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  h1 {
    margin: 0;
    font-size: 16px;
    font-weight: 700;
  }

  .refresh {
    background: var(--accent);
    color: white;
    border: none;
    border-radius: var(--radius-sm);
    padding: 6px 12px;
    font-size: 12px;
    font-weight: 600;
  }

  .refresh:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .panels {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .errors {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: 12px 16px;
  }

  .errors h2 {
    margin: 0 0 6px;
    font-size: 12px;
    font-weight: 650;
    color: var(--text-muted);
  }

  .errors ul {
    margin: 0;
    padding-left: 16px;
    font-size: 11.5px;
    color: var(--text-muted);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  footer {
    margin-top: auto;
    font-size: 11px;
    color: var(--text-muted);
    text-align: center;
  }
</style>
