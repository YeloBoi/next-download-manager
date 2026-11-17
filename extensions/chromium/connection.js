import { request } from './protocol.js';

// The background owns this port; closing the popup cannot close the desktop link.
export function createConnection(api, timers = globalThis) {
  let port, pending, retry, stopped = false;
  let status = { connected: false, reason: 'connecting' };
  function publish(connected, reason) {
    status = { connected, reason };
    void api.storage.local.set({ connectionStatus: status }).catch(() => {});
    return status;
  }
  function schedule() {
    timers.clearTimeout(retry);
    if (!stopped) retry = timers.setTimeout(() => { void check(); }, 5000);
  }
  function finish(connected, reason) {
    const value = publish(connected, reason);
    const waiting = pending;
    pending = undefined;
    if (waiting) { timers.clearTimeout(waiting.timeout); waiting.resolve(value); }
    schedule();
    return value;
  }
  function connect() {
    if (port) return port;
    const active = api.runtime.connectNative('local.ndm.bridge');
    port = active;
    active.onMessage.addListener(message => {
      if (stopped || port !== active || !pending || message?.request_id !== pending.id) return;
      if (message.version !== 1 || typeof message.accepted !== 'boolean') return;
      finish(message.accepted, message.accepted ? 'connected' : 'desktop-unavailable');
    });
    active.onDisconnect.addListener(() => {
      // Reading lastError acknowledges Chromium's disconnect diagnostic.
      void api.runtime.lastError;
      if (stopped || port !== active) return;
      port = undefined;
      finish(false, 'host-unavailable');
    });
    return active;
  }
  function check() {
    if (stopped) return Promise.resolve(status);
    if (pending) return pending.promise;
    timers.clearTimeout(retry);
    let active;
    try { active = connect(); }
    catch { return Promise.resolve(finish(false, 'host-unavailable')); }
    const message = request('ping');
    let resolve;
    const promise = new Promise(done => { resolve = done; });
    pending = { id: message.request_id, promise, resolve,
      timeout: timers.setTimeout(() => {
        if (port !== active) return;
        port = undefined;
        finish(false, 'host-unavailable');
        active.disconnect();
      }, 5000) };
    try { active.postMessage(message); }
    catch {
      port = undefined;
      finish(false, 'host-unavailable');
      active.disconnect();
    }
    return promise;
  }
  return {
    start() { void check(); }, check,
    stop() {
      stopped = true;
      timers.clearTimeout(retry);
      if (pending) { timers.clearTimeout(pending.timeout); pending.resolve(status); pending = undefined; }
      port?.disconnect(); port = undefined;
    },
  };
}
