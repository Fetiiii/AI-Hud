export interface UsageWindow {
  utilizationPct: number;
  resetsAt: string | null;
}

export interface ProviderUsage {
  fiveHour: UsageWindow | null;
  sevenDay: UsageWindow | null;
  sevenDayOpus: UsageWindow | null;
  sevenDaySonnet: UsageWindow | null;
}

export interface ContextUsage {
  provider: "claude" | "codex";
  tokens: number;
  contextWindow: number;
  project: string | null;
}

export interface Snapshot {
  claudeUsage: ProviderUsage | null;
  codexUsage: ProviderUsage | null;
  claudeContext: ContextUsage | null;
  codexContext: ContextUsage | null;
  errors: string[];
  updatedAtMs: number;
}

export const emptySnapshot: Snapshot = {
  claudeUsage: null,
  codexUsage: null,
  claudeContext: null,
  codexContext: null,
  errors: [],
  updatedAtMs: 0,
};
