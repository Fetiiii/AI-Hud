export interface UsageWindow {
  utilizationPct: number;
  resetsAt: string | null;
  /** Provider-reported window name, derived from its actual duration. Null
   *  when the provider doesn't report one — see `windowLabel` in format.ts. */
  label: string | null;
  windowMinutes: number | null;
}

export interface ProviderUsage {
  fiveHour: UsageWindow | null;
  sevenDay: UsageWindow | null;
  sevenDayOpus: UsageWindow | null;
  sevenDaySonnet: UsageWindow | null;
  planType: string | null;
}

export interface ContextUsage {
  provider: "claude" | "codex";
  tokens: number;
  contextWindow: number;
  project: string | null;
}

export interface TokenTotals {
  input: number;
  output: number;
  cacheRead: number;
  /** 5-minute-TTL cache writes. */
  cacheWrite5m: number;
  /** 1-hour-TTL cache writes, which bill higher. */
  cacheWrite1h: number;
}

export interface ModelCost {
  model: string;
  tokens: TokenTotals;
  /** Null when the model has no published price — shown as unpriced rather
   *  than silently counted as free. */
  usd: number | null;
}

export interface SessionCost {
  tokens: TokenTotals;
  usd: number;
  /** True when some model couldn't be priced, so `usd` is a lower bound. */
  partial: boolean;
  models: ModelCost[];
}

export interface Snapshot {
  claudeUsage: ProviderUsage | null;
  codexUsage: ProviderUsage | null;
  claudeContext: ContextUsage | null;
  codexContext: ContextUsage | null;
  claudeCost: SessionCost | null;
  codexCost: SessionCost | null;
  errors: string[];
  updatedAtMs: number;
}

export const emptySnapshot: Snapshot = {
  claudeUsage: null,
  codexUsage: null,
  claudeContext: null,
  codexContext: null,
  claudeCost: null,
  codexCost: null,
  errors: [],
  updatedAtMs: 0,
};
