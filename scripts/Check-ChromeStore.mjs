import assert from 'node:assert/strict';
import { resolve } from 'node:path';
import { chromium } from 'playwright';

const source = resolve(process.argv[2]);
const context = await chromium.launchPersistentContext('', {
  channel: 'chromium',
  headless: true,
  args: [`--disable-extensions-except=${source}`, `--load-extension=${source}`],
});
try {
  const worker = context.serviceWorkers()[0]
    ?? await context.waitForEvent('serviceworker', { timeout: 15_000 });
  const page = await context.newPage();
  const errors = [];
  const remoteRequests = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('request', request => {
    if (/^https?:/.test(request.url())) remoteRequests.push(request.url());
  });
  await page.goto(new URL('/popup.html', worker.url()).href);
  const manifest = await page.evaluate(() => chrome.runtime.getManifest());
  assert.equal(manifest.manifest_version, 3);
  assert.equal(manifest.host_permissions, undefined);
  assert.equal(manifest.key, undefined);

  await page.locator('#urls').fill('https://example.com/ndm-store-smoke.zip');
  await page.locator('#collect').click();
  const selection = page.locator('#selection li details');
  await selection.waitFor();
  await page.reload();
  await selection.waitFor();
  assert.match(await selection.innerText(), /ndm-store-smoke/);
  assert.equal(await page.locator('#extension-update').isVisible(), false);
  assert.deepEqual(errors, []);
  assert.deepEqual(remoteRequests, []);
  console.log('Chrome store package loaded and retained a selected link across reload.');
} finally {
  await context.close();
}
