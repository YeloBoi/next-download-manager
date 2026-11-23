const releases = 'https://github.com/YeloBoi/next-download-manager/releases';
const tagPattern = /^v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/;
const extensionVersion = /^\d{1,5}(?:\.\d{1,5}){0,3}$/;

export function newerExtension(feed, installed, browser) {
  const entry = feed?.[browser];
  if (!['chromium', 'firefox'].includes(browser) || !tagPattern.test(feed?.tag) ||
      !extensionVersion.test(installed) || !extensionVersion.test(entry?.version)) return null;
  const current = installed.split('.').map(Number), available = entry.version.split('.').map(Number);
  let diff = 0;
  for (let i = 0; i < 4 && !diff; i++) diff = (available[i] || 0) - (current[i] || 0);
  if (diff <= 0) return null;
  const expected = `NDM-${feed.tag.slice(1)}-${browser}`;
  if (entry.name !== `${expected}.zip` && !(browser === 'firefox' && entry.name === `${expected}.xpi`)) return null;
  return { tag: feed.tag, url: `${releases}/download/${feed.tag}/${entry.name}` };
}

export async function extensionFeed(storage) {
  const key = 'ndmReleaseFeed';
  const { [key]: saved } = await storage.get(key);
  if (Date.now() - saved?.time < 60 * 60 * 1000) return saved.feed;
  try {
    const response = await fetch(`${releases}/latest/download/ndm-updates.json`, {
      signal: AbortSignal.timeout(6000), credentials: 'omit', referrerPolicy: 'no-referrer',
    });
    if (!response.ok) return saved?.feed;
    const feed = await response.json();
    await storage.set({ [key]: { time: Date.now(), feed } });
    return feed;
  } catch { return saved?.feed; }
}
