import { request, sendNative, validateUrls } from './protocol.js';
import { mergeSelection, changeSelection } from './selection.js';

let mutations = Promise.resolve();
function serialize(action) {
  const result = mutations.then(action);
  mutations = result.catch(() => {});
  return result;
}
async function selection() {
  const { selection = [] } = await chrome.storage.local.get('selection');
  return mergeSelection([], selection);
}
async function save(urls) {
  await chrome.storage.local.set({ selection: urls });
  await chrome.action.setBadgeText({ text: urls.length ? String(urls.length) : '' });
  return urls;
}
async function send(urls, start) {
  if (!urls.length) throw new Error('Select between 1 and 100 links.');
  try {
    await sendNative(request('add', urls, start));
    // Remove only the acknowledged URLs; unrelated selections remain.
    await save((await selection()).filter(url => !urls.includes(url)));
    await chrome.storage.local.set({ lastResult: 'accepted' });
    await chrome.action.setBadgeBackgroundColor({ color: '#245d9e' });
  } catch {
    await chrome.storage.local.set({ lastResult: 'failed' });
    await chrome.action.setBadgeBackgroundColor({ color: '#ab3045' });
    throw new Error('NDM did not acknowledge these links. They are saved; open NDM, check setup and retry.');
  }
}
async function heartbeat() {
  try { await sendNative(request('ping')); } catch { /* A closed app is not an uninstalled extension. */ }
}
async function setup() {
  const { language = 'en' } = await chrome.storage.local.get('language');
  await chrome.contextMenus.removeAll();
  for (const [id, en, fa] of [
    ['ndm-link', 'Download with NDM', 'دانلود با NDM'],
    ['ndm-select', 'Add link to NDM selection', 'افزودن لینک به انتخاب‌های NDM'],
  ]) chrome.contextMenus.create({ id, title: language === 'fa' ? fa : en, contexts: ['link', 'video', 'audio'], targetUrlPatterns: ['http://*/*', 'https://*/*'] });
  await save(await selection());
  await chrome.alarms.create('ndm-connection', { periodInMinutes: 1 });
  void heartbeat();
}
chrome.runtime.onInstalled.addListener(() => { void serialize(setup); });
chrome.runtime.onStartup.addListener(() => { void serialize(setup); });
chrome.storage.onChanged.addListener(changes => { if (changes.language) void serialize(setup); });
chrome.alarms.onAlarm.addListener(alarm => { if (alarm.name === 'ndm-connection') void heartbeat(); });
chrome.contextMenus.onClicked.addListener(info => {
  void serialize(async () => {
    const urls = validateUrls([info.linkUrl || info.srcUrl]);
    await save(mergeSelection(await selection(), urls));
    if (info.menuItemId === 'ndm-link') await send(urls, true);
  }).catch(async () => {
    await chrome.storage.local.set({ lastResult: 'failed' });
    await chrome.action.setBadgeText({ text: '!' });
    await chrome.action.setBadgeBackgroundColor({ color: '#ab3045' });
  });
});
chrome.runtime.onMessage.addListener((message, sender, reply) => {
  if (sender.id !== chrome.runtime.id || !sender.url?.startsWith(chrome.runtime.getURL(''))) return;
  if (message?.type !== 'selection') return;
  void serialize(async () => {
    let urls = await selection();
    if (message.operation === 'add') urls = await save(mergeSelection(urls, validateUrls(message.urls)));
    else if (['remove', 'move'].includes(message.operation)) urls = await save(changeSelection(urls, message.operation, message.url, message.offset));
    else if (message.operation === 'clear') urls = await save([]);
    else if (message.operation === 'send') { await send(urls, message.start === true); urls = await selection(); }
    else if (message.operation !== 'get') throw new Error('Invalid selection operation.');
    const { lastResult } = await chrome.storage.local.get('lastResult');
    return { ok: true, urls, lastResult };
  }).then(reply, () => reply({ ok: false }));
  return true;
});
