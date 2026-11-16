import test from 'node:test';
import assert from 'node:assert/strict';
import { bytes, percent, eta, escapeHtml, parseLinks } from '../apps/desktop/src/format.mjs';

test('unknown file sizes never produce invented percentages or ETAs',()=>{
  assert.equal(percent({total:null,downloaded:900}),null);
  assert.equal(eta({total:null,downloaded:900,speed:100,status:'downloading'}),'—');
  assert.equal(percent({total:0,downloaded:0,status:'completed'}),100);
  assert.equal(bytes(null),'—');assert.equal(bytes(0),'0 B');assert.equal(bytes(1024),'1.0 KiB');
});
test('server-controlled filenames are escaped before insertion into HTML',()=>{
  assert.equal(escapeHtml('<img src=x onerror="bad()">'), '&lt;img src=x onerror=&quot;bad()&quot;&gt;');
});
test('batch links keep ordering and ignore blank lines',()=>{
  assert.deepEqual(parseLinks(' https://a.test/a\r\n\nhttps://b.test/b\n'),['https://a.test/a','https://b.test/b']);
});
