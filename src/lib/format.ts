export function timeUntil(iso: string | null | undefined): string {
  if (!iso) return "";
  const target = new Date(iso).getTime();
  if (Number.isNaN(target)) return "";
  const diffMs = target - Date.now();
  if (diffMs <= 0) return "yenilendi";

  const mins = Math.round(diffMs / 60000);
  if (mins < 1) return "az sonra";
  if (mins < 60) return `${mins} dk sonra`;

  const hours = Math.floor(mins / 60);
  const remMins = mins % 60;
  if (hours < 24) {
    return remMins > 0 ? `${hours} sa ${remMins} dk sonra` : `${hours} sa sonra`;
  }

  const days = Math.floor(hours / 24);
  const remHours = hours % 24;
  return remHours > 0 ? `${days} gün ${remHours} sa sonra` : `${days} gün sonra`;
}

/** A window's display name: what the provider reported, else the slot's
 *  conventional name. Never assume the slot — a free Codex plan's primary
 *  window is monthly, not 5-hourly. */
export function windowLabel(
  window: { label: string | null } | null | undefined,
  fallback: string,
): string {
  return window?.label ?? fallback;
}

/** Small amounts keep their cents visible — "$0.00" for a real four-cent
 *  session reads as broken, so sub-dollar figures get more precision. */
export function formatUsd(n: number): string {
  if (n === 0) return "$0";
  if (n < 0.01) return "<$0.01";
  if (n < 10) return `$${n.toFixed(2)}`;
  return `$${n.toFixed(1)}`;
}

export function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1000).toFixed(1)}K`;
  return `${n}`;
}

export function formatUpdatedAt(ms: number): string {
  if (!ms) return "henüz güncellenmedi";
  const diff = Date.now() - ms;
  if (diff < 5_000) return "az önce";
  if (diff < 60_000) return `${Math.round(diff / 1000)} sn önce güncellendi`;
  return `${Math.round(diff / 60000)} dk önce güncellendi`;
}
