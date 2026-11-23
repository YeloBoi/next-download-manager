import { extensionApi } from './api.js';

export function nativeFailure(error) {
  const message = String(error?.message ?? error ?? '');
  if (/could not open NDM|did not acknowledge the request/i.test(message)) return 'launch';
  if (/host not found|no such native application|not registered|forbidden|not allowed to connect|access.*denied/i.test(message)) return 'setup';
  if (/invalid response|invalid bridge message/i.test(message)) return 'version';
  if (/message limit/i.test(message)) return 'batch';
  return 'delivery';
}

export function validateUrls(urls) {
  if (!Array.isArray(urls) || urls.length < 1 || urls.length > 100) throw new Error('Select between 1 and 100 links.');
  const normalized = urls.map(input => {
    if (typeof input !== 'string' || input.length > 16384) throw new Error('Invalid URL.');
    const url = new URL(input.trim());
    if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password) throw new Error('Only HTTP(S) URLs without credentials are supported.');
    return url.href;
  });
  if (new Set(normalized).size !== normalized.length) throw new Error('Duplicate link selected.');
  return normalized;
}
export function request(kind, urls = [], start = false) {
  return { version: 1, request_id: crypto.randomUUID(), kind, urls:kind==='add'?validateUrls(urls):[], start, created_at:Math.floor(Date.now()/1000) };
}
export async function sendNative(message) {
  if (new TextEncoder().encode(JSON.stringify(message)).length > 256 * 1024) throw new Error('Selected links exceed the native message limit. Send a smaller batch.');
  const response = await extensionApi().runtime.sendNativeMessage('local.ndm.bridge', message);
  if (response?.version === 1 && response.request_id === '' && response.accepted === false &&
      nativeFailure(response.error) === 'setup') throw new Error(response.error);
  if (response?.version !== 1 || response.request_id !== message.request_id || typeof response.accepted !== 'boolean') throw new Error('Invalid response from NDM.');
  if (!response.accepted) throw new Error(response.error || 'NDM did not accept the request.');
  return response;
}
