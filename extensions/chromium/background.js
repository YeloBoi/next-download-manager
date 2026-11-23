import { extensionApi } from './api.js';
const api = extensionApi();
import { nativeFailure, request, sendNative, validateUrls } from './protocol.js';
import { mergeSelection, changeSelection } from './selection.js';
import { createConnection } from './connection.js';

export const connection = createConnection(api);

let mutations = Promise.resolve();
function serialize(action) {
  const result = mutations.then(action);
  mutations = result.catch(() => {});
  return result;
}
async function selection() {
  const { selection = [] } = await api.storage.local.get('selection');
  return mergeSelection([], selection);
}
async function save(urls) {
  await api.storage.local.set({ selection: urls });
  await api.action.setBadgeText({ text: urls.length ? String(urls.length) : '' });
  return urls;
}
async function send(urls, start) {
  if (!urls.length) throw new Error('Select between 1 and 100 links.');
  try {
    await sendNative(request('add', urls, start));
    // Remove only the acknowledged URLs; unrelated selections remain.
    await save((await selection()).filter(url => !urls.includes(url)));
    await api.storage.local.set({ lastResult: 'accepted' });
    await api.action.setBadgeBackgroundColor({ color: '#245d9e' });
  } catch (error) {
    const reason = nativeFailure(error);
    await api.storage.local.set({ lastResult: 'failed', lastFailure: reason });
    await api.action.setBadgeBackgroundColor({ color: '#ab3045' });
    return reason;
  }
}
async function setup() {
  const { language = 'en' } = await api.storage.local.get('language');
  await api.contextMenus.removeAll();
  for (const [id, en, fa] of [
    ['ndm-link', 'Download with NDM', 'دانلود با NDM'],
    ['ndm-select', 'Add link to NDM selection', 'افزودن لینک به انتخاب‌های NDM'],
  ]) api.contextMenus.create({ id, title: language === 'fa' ? fa : en, contexts: ['link', 'video', 'audio'], targetUrlPatterns: ['http://*/*', 'https://*/*'] });
  await save(await selection());
  await api.alarms.create('ndm-connection', { periodInMinutes: 1 });
  connection.start();
}
const setupMenus = () => { void serialize(setup).catch(error => console.error('NDM extension setup failed:', error)); };
api.runtime.onInstalled.addListener(setupMenus);
api.runtime.onStartup.addListener(setupMenus);
api.storage.onChanged.addListener(changes => { if (changes.language) setupMenus(); });
api.alarms.onAlarm.addListener(alarm => { if (alarm.name === 'ndm-connection') connection.start(); });
api.contextMenus.onClicked.addListener(info => {
  if (!['ndm-link', 'ndm-select'].includes(info.menuItemId)) return;
  void serialize(async () => {
    const urls = validateUrls([info.linkUrl || info.srcUrl]);
    await save(mergeSelection(await selection(), urls));
    if (info.menuItemId === 'ndm-link' && await send(urls, true)) await api.action.setBadgeText({ text: '!' });
  }).catch(async () => {
    await api.storage.local.set({ lastResult: 'failed', lastFailure: 'invalid' });
    await api.action.setBadgeText({ text: '!' });
    await api.action.setBadgeBackgroundColor({ color: '#ab3045' });
  }).catch(() => {});
});
api.runtime.onMessage.addListener((message, sender, reply) => {
  if (sender.id !== api.runtime.id || !sender.url?.startsWith(api.runtime.getURL(''))) return;
  if (message?.type === 'connection') {
    void connection.check().then(reply);
    return true;
  }
  if (message?.type !== 'selection') return;
  void serialize(async () => {
    let urls = await selection();
    if (message.operation === 'add') urls = await save(mergeSelection(urls, validateUrls(message.urls)));
    else if (['remove', 'move'].includes(message.operation)) urls = await save(changeSelection(urls, message.operation, message.url, message.offset));
    else if (message.operation === 'clear') urls = await save([]);
    else if (message.operation === 'send') {
      if (!urls.length) return { ok: false, reason: 'empty' };
      const reason = await send(urls, message.start === true);
      if (reason) return { ok: false, reason };
      urls = await selection();
    }
    else if (message.operation !== 'get') throw new Error('Invalid selection operation.');
    const { lastResult, lastFailure } = await api.storage.local.get(['lastResult', 'lastFailure']);
    return { ok: true, urls, lastResult, lastFailure };
  }).then(reply, () => reply({ ok: false }));
  return true;
});
connection.start();
