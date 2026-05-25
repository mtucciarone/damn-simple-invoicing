export interface CapturedError {
  title: string;
  name: string;
  message: string;
  stack: string | null;
  cause: string | null;
}

function stringifyValue(value: unknown): string {
  if (value === null || value === undefined) {
    return 'Unknown error';
  }

  if (typeof value === 'string') {
    return value;
  }

  if (typeof value === 'number' || typeof value === 'boolean' || typeof value === 'bigint') {
    return String(value);
  }

  if (value instanceof Error) {
    return value.message || value.name || 'Unknown error';
  }

  if (typeof value === 'object') {
    try {
      return JSON.stringify(value, null, 2);
    } catch {
      return Object.prototype.toString.call(value);
    }
  }

  return String(value);
}

function extractStack(value: unknown): string | null {
  if (value instanceof Error) {
    return value.stack ?? null;
  }

  if (typeof value === 'object' && value !== null && 'stack' in value) {
    const stack = (value as { stack?: unknown }).stack;
    return typeof stack === 'string' && stack.trim().length > 0 ? stack : null;
  }

  return null;
}

function extractCause(value: unknown): string | null {
  if (value instanceof Error) {
    if (value.cause === undefined || value.cause === null) {
      return null;
    }

    return stringifyValue(value.cause);
  }

  if (typeof value === 'object' && value !== null && 'cause' in value) {
    const cause = (value as { cause?: unknown }).cause;
    if (cause === undefined || cause === null) {
      return null;
    }

    return stringifyValue(cause);
  }

  return null;
}

function extractName(value: unknown): string {
  if (value instanceof Error && value.name.trim().length > 0) {
    return value.name;
  }

  if (typeof value === 'object' && value !== null && 'name' in value) {
    const name = (value as { name?: unknown }).name;
    if (typeof name === 'string' && name.trim().length > 0) {
      return name;
    }
  }

  return 'Error';
}

export function describeError(error: unknown, title: string): CapturedError {
  return {
    title,
    name: extractName(error),
    message: stringifyValue(error),
    stack: extractStack(error),
    cause: extractCause(error),
  };
}
