import { validateUrls } from './protocol.js';

// Storage contains URLs only. Mutations are serialized by the service worker.
export function mergeSelection(current, incoming) {
  const urls = [...new Set([...current, ...incoming].map(url => validateUrls([url])[0]))];
  return urls.length ? validateUrls(urls) : [];
}

export function changeSelection(current, operation, url, offset = 0) {
  const urls = [...current];
  const index = urls.indexOf(url);
  if (index < 0) throw new Error('This link is no longer selected.');
  if (operation === 'remove') urls.splice(index, 1);
  else if (operation === 'move' && [-1, 1].includes(offset)) {
    const target = index + offset;
    if (target >= 0 && target < urls.length) [urls[index], urls[target]] = [urls[target], urls[index]];
  } else throw new Error('Invalid selection operation.');
  return urls;
}
