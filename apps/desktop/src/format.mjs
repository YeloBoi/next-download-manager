export function bytes(value) {
  if (value == null) return '—';
  if (value < 1024) return `${value} B`;
  const units = ['KiB', 'MiB', 'GiB', 'TiB'];
  let index = -1;
  do { value /= 1024; index++; } while (value >= 1024 && index < units.length - 1);
  return `${value.toFixed(1)} ${units[index]}`;
}
export function percent(download) {
  if (download.total == null) return null;
  if (download.total === 0) return download.status === 'completed' ? 100 : 0;
  return Math.min(100, Math.max(0, download.downloaded / download.total * 100));
}
export function eta(download) {
  if (download.total == null || download.speed <= 0 || !['downloading', 'connecting'].includes(download.status)) return '—';
  const seconds = Math.max(0, Math.ceil((download.total - download.downloaded) / download.speed));
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
  return `${Math.floor(seconds / 3600)}h ${Math.floor(seconds % 3600 / 60)}m`;
}
export function escapeHtml(value) {
  return String(value ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
}
export function parseLinks(value) { return value.split(/\r?\n/).map(s => s.trim()).filter(Boolean); }
