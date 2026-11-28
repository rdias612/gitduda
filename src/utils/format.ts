import { msg } from '@lit/localize';
import { getAppLocale } from '../i18n/index.ts';

const relativeTimeFormats = new Map<string, Intl.RelativeTimeFormat>();

/**
 * Format a date relative to now
 */
export function formatRelativeTime(timestamp: number): string {
  if (typeof timestamp !== 'number' || !isFinite(timestamp)) return msg('unknown');
  const now = Date.now();
  const diff = now - timestamp * 1000;
  const seconds = Math.floor(diff / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);
  const weeks = Math.floor(days / 7);
  const months = Math.floor(days / 30);
  const years = Math.floor(days / 365);

  if (seconds < 60) return msg('just now');
  const locale = getAppLocale();
  let format = relativeTimeFormats.get(locale);
  if (!format) {
    format = new Intl.RelativeTimeFormat(locale, { style: 'narrow' });
    relativeTimeFormats.set(locale, format);
  }
  if (minutes < 60) return format.format(-minutes, 'minute');
  if (hours < 24) return format.format(-hours, 'hour');
  if (days < 7) return format.format(-days, 'day');
  if (days < 30) return format.format(-weeks, 'week');
  if (days < 365) return format.format(-months, 'month');
  return format.format(-years, 'year');
}

/**
 * Format a date in the active locale
 */
export function formatDate(timestamp: number): string {
  return new Date(timestamp * 1000).toLocaleDateString(getAppLocale());
}

/**
 * Format a date with time
 */
export function formatDateTime(timestamp: number): string {
  return new Date(timestamp * 1000).toLocaleString(getAppLocale());
}

/**
 * Truncate a string with ellipsis
 */
export function truncate(str: string, maxLength: number): string {
  if (str.length <= maxLength) return str;
  return str.slice(0, maxLength - 1) + '…';
}

/**
 * Format a commit SHA (short)
 */
export function formatSha(sha: string, length = 7): string {
  return sha.slice(0, length);
}

/**
 * Format file size
 */
export function formatFileSize(bytes: number): string {
  const units = ['B', 'KB', 'MB', 'GB'];
  let size = bytes;
  let unitIndex = 0;

  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex++;
  }

  const digits = unitIndex > 0 ? 1 : 0;
  const formatted = size.toLocaleString(getAppLocale(), {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
    useGrouping: false,
  });
  return `${formatted} ${units[unitIndex]}`;
}

/**
 * Format a number with separators
 */
export function formatNumber(num: number): string {
  return num.toLocaleString(getAppLocale());
}

/**
 * Pluralize a word based on count
 */
export function pluralize(count: number, singular: string, plural?: string): string {
  const word = count === 1 ? singular : (plural ?? singular + 's');
  return `${count} ${word}`;
}
