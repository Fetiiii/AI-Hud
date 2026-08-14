import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { emptySnapshot, type Snapshot } from "./types";

class SnapshotStore {
  data = $state<Snapshot>(emptySnapshot);
  loading = $state(true);
  refreshing = $state(false);
  started = false;

  async start() {
    if (this.started) return;
    this.started = true;

    try {
      this.data = await invoke<Snapshot>("get_snapshot");
    } catch {
      // Backend not ready yet; the push event below will catch us up.
    } finally {
      this.loading = false;
    }

    await listen<Snapshot>("snapshot-updated", (event) => {
      this.data = event.payload;
      this.loading = false;
      this.refreshing = false;
    });
  }

  async refresh() {
    this.refreshing = true;
    try {
      this.data = await invoke<Snapshot>("refresh_now");
    } finally {
      this.refreshing = false;
    }
  }
}

export const snapshot = new SnapshotStore();
