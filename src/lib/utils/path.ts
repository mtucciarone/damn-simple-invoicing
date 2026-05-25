export function parentDirectory(path: string): string {
  const normalized = path.trim();
  if (!normalized) {
    return '';
  }

  if (normalized === '/') {
    return '/';
  }

  if (/^[A-Za-z]:[\\/]*$/.test(normalized)) {
    return `${normalized.slice(0, 2)}\\`;
  }

  const trimmed = normalized.replace(/[\\/]+$/, '');
  if (!trimmed) {
    return '';
  }

  const separatorIndex = Math.max(trimmed.lastIndexOf('/'), trimmed.lastIndexOf('\\'));
  if (separatorIndex < 0) {
    return '';
  }

  if (separatorIndex === 0) {
    return '/';
  }

  if (/^[A-Za-z]:/.test(trimmed) && separatorIndex === 2) {
    return `${trimmed.slice(0, 2)}\\`;
  }

  return trimmed.slice(0, separatorIndex);
}

export function joinLocalPath(base: string, segment: string): string {
  const trimmedBase = base.trim().replace(/[\\/]+$/, '');
  const trimmedSegment = segment.trim().replace(/^[\\/]+/, '');

  if (!trimmedBase) {
    return base.trim().startsWith('/') ? `/${trimmedSegment}` : trimmedSegment;
  }

  if (/^[A-Za-z]:$/.test(trimmedBase)) {
    return `${trimmedBase}\\${trimmedSegment}`;
  }

  const separator = trimmedBase.includes('\\') && !trimmedBase.includes('/') ? '\\' : '/';
  return `${trimmedBase}${separator}${trimmedSegment}`;
}
