import test from 'node:test';
import assert from 'node:assert/strict';
import { createConnection } from '../extensions/chromium/connection.js';

function fixture() {
  const tasks = new Map(), ports = [], saved = [];
  let sequence = 0, online = true, drop = false;
  const timers = {
    setTimeout(fn, delay) { const id = ++sequence; tasks.set(id, { fn, delay }); return id; },
    clearTimeout(id) { tasks.delete(id); },
  };
  const api = {
    storage: { local: { set: async value => { saved.push(structuredClone(value.connectionStatus)); } } },
    runtime: { connectNative(host) {
      assert.equal(host, 'local.ndm.bridge');
      let message, disconnect;
      const port = {
        onMessage: { addListener(fn) { message = fn; } },
        onDisconnect: { addListener(fn) { disconnect = fn; } },
        postMessage(value) { if (!drop) queueMicrotask(() => message({ version: 1, request_id: value.request_id, accepted: online })); },
        disconnect() { disconnect(); },
        reply(value) { message(value); },
      };
      ports.push(port); return port;
    } },
  };
  const connection = createConnection(api, timers);
  const flush = () => new Promise(resolve => setImmediate(resolve));
  return { connection, ports, saved, flush,
    online(value) { online = value; }, drop(value) { drop = value; },
    async tick(delay) {
      for (const [id, task] of [...tasks]) if (task.delay === delay) { tasks.delete(id); task.fn(); }
      await flush();
    },
  };
}

test('background connects without a popup and keeps the port across desktop restarts', async () => {
  const f = fixture();
  try {
    f.connection.start(); await f.flush();
    assert.deepEqual(f.saved.at(-1), { connected: true, reason: 'connected' });
    f.online(false); await f.tick(5000);
    assert.deepEqual(f.saved.at(-1), { connected: false, reason: 'desktop-unavailable' });
    f.online(true); await f.tick(5000);
    assert.equal(f.saved.at(-1).connected, true);
    assert.equal(f.ports.length, 1, 'desktop restart must reuse the background-owned native port');
    await Promise.all([f.connection.check(), f.connection.check()]);
    assert.equal(f.ports.length, 1);
  } finally { f.connection.stop(); }
});

test('native host disconnect reconnects automatically without opening a popup', async () => {
  const f = fixture();
  try {
    f.connection.start(); await f.flush();
    f.ports[0].disconnect();
    assert.equal(f.saved.at(-1).connected, false);
    await f.tick(5000);
    assert.equal(f.ports.length, 2);
    assert.equal(f.saved.at(-1).connected, true);
  } finally { f.connection.stop(); }
});

test('unmatched responses cannot manufacture a connection and silent ports time out', async () => {
  const f = fixture(); f.drop(true);
  try {
    const result = f.connection.check();
    f.ports[0].reply({ version: 1, request_id: 'unrelated', accepted: true });
    assert.equal(f.saved.length, 0);
    await f.tick(5000);
    assert.deepEqual(await result, { connected: false, reason: 'host-unavailable' });
    f.drop(false); await f.tick(5000);
    assert.equal(f.saved.at(-1).connected, true);
  } finally { f.connection.stop(); }
});
