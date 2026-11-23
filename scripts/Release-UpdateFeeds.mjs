import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { readFile, readdir, stat, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

const [tag, directory] = process.argv.slice(2);
if (!/^v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(tag)) throw new Error('Invalid release tag');
const version = tag.slice(1);
const files = await readdir(directory);
const manifest = async browser => JSON.parse(await readFile(`extensions/${browser}/manifest.json`, 'utf8'));
const chromium = await manifest('chromium');
const firefox = await manifest('firefox');
const gradle = await readFile('apps/android/app/build.gradle', 'utf8');
const code = Number(gradle.match(/^\s*versionCode (\d+)$/m)?.[1]);
if (chromium.version_name !== version || !Number.isSafeInteger(code) || !code ||
    !gradle.includes(`versionName '${version}'`)) throw new Error('Update feed version mismatch');

async function asset(name) {
  const path = join(directory, name);
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(path)) hash.update(chunk);
  return { name, size: (await stat(path)).size, sha256: hash.digest('hex') };
}

const apk = await asset(`NDM-${version}-android.apk`);
const zipChromium = await asset(`NDM-${version}-chromium.zip`);
const zipFirefox = await asset(`NDM-${version}-firefox.zip`);
const xpi = `NDM-${version}-firefox.xpi`;
const signed = files.includes(xpi) ? await asset(xpi) : null;
const base = `https://github.com/YeloBoi/next-download-manager/releases/download/${tag}`;
const gecko = firefox.browser_specific_settings.gecko;
const feeds = {
  'ndm-updates.json': {
    tag,
    android: { code, ...apk },
    chromium: { version: chromium.version, ...zipChromium },
    firefox: { version: firefox.version, ...(signed ?? zipFirefox) },
  },
  'firefox-updates.json': {
    addons: {
      [gecko.id]: {
        updates: signed ? [{
          version: firefox.version,
          update_link: `${base}/${signed.name}`,
          update_hash: `sha256:${signed.sha256}`,
          applications: { gecko: { strict_min_version: gecko.strict_min_version } },
        }] : [],
      },
    },
  },
};
for (const [name, feed] of Object.entries(feeds)) {
  await writeFile(join(directory, name), JSON.stringify(feed, null, 2) + '\n');
}
