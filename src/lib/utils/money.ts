export function blankToNull(value: string): string | null {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

export function todayIsoDate(reference = new Date()): string {
  const offsetMinutes = reference.getTimezoneOffset();
  const localTime = new Date(reference.getTime() - offsetMinutes * 60_000);
  return localTime.toISOString().slice(0, 10);
}

export function formatMinorAmount(amountMinor: number): string {
  const absolute = Math.abs(amountMinor);
  const whole = Math.trunc(absolute / 100);
  const fraction = String(absolute % 100).padStart(2, '0');
  const prefix = amountMinor < 0 ? '-' : '';
  return `${prefix}${whole}.${fraction}`;
}

export function parseMinorAmount(value: string): number {
  const normalized = value.trim().replaceAll(',', '');
  if (!normalized) {
    throw new Error('Amount is required');
  }

  const sign = normalized.startsWith('-') ? -1 : 1;
  const unsigned = sign < 0 ? normalized.slice(1) : normalized;
  const parts = unsigned.split('.');

  if (parts.length > 2) {
    throw new Error('Amount must be a decimal number');
  }

  const [wholePart, fractionPart = ''] = parts;
  if (!/^\d+$/.test(wholePart) || !/^\d*$/.test(fractionPart) || fractionPart.length > 2) {
    throw new Error('Amount must use at most 2 decimal places');
  }

  const whole = Number.parseInt(wholePart, 10);
  const fraction = Number.parseInt((fractionPart + '00').slice(0, 2), 10);

  return sign * (whole * 100 + fraction);
}
