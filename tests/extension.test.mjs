import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { validateUrls, request, sendNative } from '../extensions/chromium/protocol.js';
import { mergeSelection, changeSelection } from '../extensions/chromium/selection.js';

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
