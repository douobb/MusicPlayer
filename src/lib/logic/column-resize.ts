const STORAGE_KEY = 'musicplayer-column-widths-v2';

export const DEFAULT_WIDTHS = [0.38, 0.3, 0.16, 0.16];
export const MIN_WIDTHS = [0.12, 0.1, 0.06, 0.06];

export function loadColumnWidths(): number[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [...DEFAULT_WIDTHS];
    const parsed = JSON.parse(raw);
    if (
      !Array.isArray(parsed) ||
      parsed.length !== 4 ||
      parsed.some((v: unknown) => typeof v !== 'number')
    ) {
      return [...DEFAULT_WIDTHS];
    }
    return parsed;
  } catch {
    return [...DEFAULT_WIDTHS];
  }
}

export function saveColumnWidths(widths: number[]): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(widths));
}

export function resetColumnWidths(): number[] {
  localStorage.removeItem(STORAGE_KEY);
  return [...DEFAULT_WIDTHS];
}
