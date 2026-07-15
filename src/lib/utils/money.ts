export function blankToNull(value: string): string | null {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

export type MoneyFormatStyle = 'us' | 'eu';

export const DEFAULT_MONEY_FORMAT: MoneyFormatStyle = 'us';

const FORMAT_STYLES: Record<MoneyFormatStyle, { group: string; decimal: string }> = {
  us: { group: ',', decimal: '.' },
  eu: { group: '.', decimal: ',' },
};

export function todayIsoDate(reference = new Date()): string {
  const offsetMinutes = reference.getTimezoneOffset();
  const localTime = new Date(reference.getTime() - offsetMinutes * 60_000);
  return localTime.toISOString().slice(0, 10);
}

export function moneyFormatFromSetting(value: string | null | undefined): MoneyFormatStyle {
  return value?.trim().toLowerCase() === 'eu' ? 'eu' : 'us';
}

function groupIntegerDigits(value: string, separator: string): string {
  return value.replace(/\B(?=(\d{3})+(?!\d))/g, separator);
}

function parseMinorAmountWithStyle(value: string, _style: MoneyFormatStyle): number {
  const normalized = value.trim().replace(/\s+/g, '');
  if (!normalized) {
    throw new Error('Amount is required');
  }

  const sign = normalized.startsWith('-') ? -1 : 1;
  const unsigned = sign < 0 ? normalized.slice(1) : normalized;

  let decimalSeparator: string | null = null;
  let groupingSeparator: string | null = null;

  const lastComma = unsigned.lastIndexOf(',');
  const lastDot = unsigned.lastIndexOf('.');

  if (lastComma !== -1 && lastDot !== -1) {
    if (lastComma > lastDot) {
      decimalSeparator = ',';
      groupingSeparator = '.';
    } else {
      decimalSeparator = '.';
      groupingSeparator = ',';
    }
  } else if (lastComma !== -1) {
    const digitsAfter = unsigned.length - lastComma - 1;
    if (digitsAfter === 1 || digitsAfter === 2) {
      decimalSeparator = ',';
    } else if (digitsAfter === 3) {
      groupingSeparator = ',';
    } else {
      throw new Error('Amount must use at most 2 decimal places');
    }
  } else if (lastDot !== -1) {
    const digitsAfter = unsigned.length - lastDot - 1;
    if (digitsAfter === 1 || digitsAfter === 2) {
      decimalSeparator = '.';
    } else if (digitsAfter === 3) {
      groupingSeparator = '.';
    } else {
      throw new Error('Amount must use at most 2 decimal places');
    }
  }

  let normalizedNumber = unsigned;
  if (groupingSeparator) {
    normalizedNumber = normalizedNumber.replaceAll(groupingSeparator, '');
  }

  if (!decimalSeparator) {
    if (!/^\d+$/.test(normalizedNumber)) {
      throw new Error('Amount must be a decimal number');
    }

    return sign * Number.parseInt(normalizedNumber, 10) * 100;
  }

  const parts = normalizedNumber.split(decimalSeparator);
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

export function formatMinorAmount(amountMinor: number, style: MoneyFormatStyle = DEFAULT_MONEY_FORMAT): string {
  const absolute = BigInt(amountMinor < 0 ? -amountMinor : amountMinor);
  const whole = (absolute / 100n).toString();
  const fraction = (absolute % 100n).toString().padStart(2, '0');
  const { group, decimal } = FORMAT_STYLES[style];
  const prefix = amountMinor < 0 ? '-' : '';
  return `${prefix}${groupIntegerDigits(whole, group)}${decimal}${fraction}`;
}

export function parseMinorAmount(value: string, style: MoneyFormatStyle = DEFAULT_MONEY_FORMAT): number {
  try {
    return parseMinorAmountWithStyle(value, style);
  } catch (styleError) {
    const fallbackStyle = style === 'us' ? 'eu' : 'us';
    try {
      return parseMinorAmountWithStyle(value, fallbackStyle);
    } catch {
      throw styleError;
    }
  }
}
