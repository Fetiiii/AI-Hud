export type Tone = "normal" | "warn" | "critical" | "unknown";

/** Fill state for a usage meter. Below the warn threshold a meter carries its
 *  provider's own colour rather than a semantic "good" green — the old
 *  green/amber/red traffic light made an idle HUD look like a monitoring
 *  dashboard. Colour only escalates when there's something to notice. */
export function toneFor(pct: number | null | undefined): Tone {
  if (pct === null || pct === undefined || Number.isNaN(pct)) return "unknown";
  if (pct >= 90) return "critical";
  if (pct >= 70) return "warn";
  return "normal";
}

/** The CSS colour a meter should paint, given its provider's accent. */
export function toneColor(tone: Tone, providerVar: string): string {
  switch (tone) {
    case "critical":
      return "var(--danger)";
    case "warn":
      return "var(--warn)";
    case "normal":
      return `var(${providerVar})`;
    case "unknown":
      return "var(--track)";
  }
}
