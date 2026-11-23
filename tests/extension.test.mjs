import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { validateUrls, request, sendNative } from '../extensions/chromium/protocol.js';
import { mergeSelection, changeSelection } from '../extensions/chromium/selection.js';
import { extensionApi } from '../extensions/chromium/api.js';
import { newerExtension } from '../extensions/chromium/updates.js';

test('Firefox manifest uses module background scripts and a fixed native messaging ID',async()=>{
  const firefox=JSON.parse(await readFile(new URL('../extensions/firefox/manifest.json',import.meta.url)));
  const chromium=JSON.parse(await readFile(new URL('../extensions/chromium/manifest.json',import.meta.url)));
  assert.deepEqual(firefox.background,{scripts:['background.js'],type:'module'});
  assert.equal(firefox.browser_specific_settings.gecko.id,'ndm@yeloboi.github.io');
  assert.equal(firefox.background.service_worker,undefined);
  assert.equal(firefox.minimum_chrome_version,undefined);
  assert.equal(firefox.version,chromium.version);
  assert.deepEqual(firefox.permissions,chromium.permissions);
  globalThis.browser={runtime:{firefox:true}};
  globalThis.chrome={runtime:{chromium:true}};
  assert.equal(extensionApi(),globalThis.browser);
  delete globalThis.browser;
  assert.equal(extensionApi(),globalThis.chrome);
  delete globalThis.chrome;
});

test('native protocol rejects credentials, unsupported schemes, duplicates and large batches',()=>{
  for(const urls of [[],['file:///etc/passwd'],['https://user:secret@example.com'],['https://a.test/','https://a.test/'],Array(101).fill('https://a.test/')])assert.throws(()=>validateUrls(urls));
  assert.deepEqual(validateUrls(['https://a.test/file?token=abc']),['https://a.test/file?token=abc']);
  const value=request('add',['https://a.test/file'],false);assert.equal(value.version,1);assert.equal(value.start,false);assert.equal(value.kind,'add');
});
test('extension only has persistent host access for its release feed',async()=>{
  const manifest=JSON.parse(await readFile(new URL('../extensions/chromium/manifest.json',import.meta.url)));
  assert.equal(manifest.manifest_version,3);
  assert.deepEqual(manifest.host_permissions,['https://github.com/*','https://release-assets.githubusercontent.com/*']);
  for(const permission of ['cookies','history','downloads','webRequest'])assert.ok(!manifest.permissions.includes(permission));
});

test('persistent selection deduplicates canonical URLs and keeps explicit order',()=>{
  const selected=mergeSelection(['https://a.test'],['https://a.test/','https://b.test/','https://c.test/']);
  assert.deepEqual(selected,['https://a.test/','https://b.test/','https://c.test/']);
  assert.deepEqual(changeSelection(selected,'move','https://c.test/',-1),['https://a.test/','https://c.test/','https://b.test/']);
  assert.deepEqual(changeSelection(selected,'remove','https://b.test/'),['https://a.test/','https://c.test/']);
  assert.deepEqual(selected,['https://a.test/','https://b.test/','https://c.test/']);
  assert.throws(()=>mergeSelection([],Array.from({length:101},(_,i)=>`https://a.test/${i}`)));
  assert.throws(()=>mergeSelection([],['javascript:alert(1)']));
  assert.throws(()=>changeSelection(selected,'move','https://a.test/',10));
});

test('native protocol rejects oversized batches before contacting the host and mismatched replies',async()=>{
  let calls=0;
  globalThis.chrome={runtime:{sendNativeMessage:async(_host,message)=>{calls++;return {...message,request_id:'wrong',accepted:true};}}};
  await assert.rejects(sendNative(request('add',Array.from({length:30},(_,i)=>`https://a.test/${i}?${'a'.repeat(10000)}`))),/message limit/);
  assert.equal(calls,0);
  await assert.rejects(sendNative(request('ping')),/Invalid response/);
  assert.equal(calls,1);
  delete globalThis.chrome;
});
test('extension updates use numeric versions and the matching GitHub release asset',()=>{
  const feed={tag:'v0.1.0-alpha.11',chromium:{version:'0.1.10',name:'NDM-0.1.0-alpha.11-chromium.zip'},
    firefox:{version:'0.1.10',name:'NDM-0.1.0-alpha.11-firefox.xpi'}};
  assert.match(newerExtension(feed,'0.1.9','chromium').url,/alpha\.11-chromium\.zip$/);
  assert.match(newerExtension(feed,'0.1.9','firefox').url,/alpha\.11-firefox\.xpi$/);
  assert.equal(newerExtension(feed,'0.1.10','firefox'),null);
  assert.equal(newerExtension(feed,'0.2','chromium'),null);
  assert.equal(newerExtension({...feed,chromium:{...feed.chromium,name:'https://example.com/foreign.zip'}},'0.1.9','chromium'),null);
});
