import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { validateUrls, request } from '../extensions/chromium/protocol.js';

test('native protocol rejects credentials, unsupported schemes, duplicates and large batches',()=>{
  for(const urls of [[],['file:///etc/passwd'],['https://user:secret@example.com'],['https://a.test/','https://a.test/'],Array(101).fill('https://a.test/')])assert.throws(()=>validateUrls(urls));
  assert.deepEqual(validateUrls(['https://a.test/file?token=abc']),['https://a.test/file?token=abc']);
  const value=request('add',['https://a.test/file'],false);assert.equal(value.version,1);assert.equal(value.start,false);assert.equal(value.kind,'add');
});
test('extension has no persistent site, cookies, history or download interception permission',async()=>{
  const manifest=JSON.parse(await readFile(new URL('../extensions/chromium/manifest.json',import.meta.url)));
  assert.equal(manifest.manifest_version,3);
  assert.equal(manifest.host_permissions,undefined);
  for(const permission of ['cookies','history','downloads','webRequest'])assert.ok(!manifest.permissions.includes(permission));
});
