import { validateUrls } from './protocol.js';

async function updateMenu() {
  const { language = 'en' } = await chrome.storage.local.get('language');
  await chrome.contextMenus.removeAll();
  chrome.contextMenus.create({ id:'ndm-link', title:language==='fa'?'دانلود با NDM':'Download with NDM', contexts:['link','video','audio'], targetUrlPatterns:['http://*/*','https://*/*'] });
}
chrome.runtime.onInstalled.addListener(updateMenu);
chrome.runtime.onStartup.addListener(updateMenu);
chrome.storage.onChanged.addListener(changes=>{if(changes.language)void updateMenu();});
chrome.contextMenus.onClicked.addListener(async info=>{
  try {
    const urls=validateUrls([info.linkUrl||info.srcUrl]);
    await chrome.storage.session.set({ pendingUrls:urls });
    try { await chrome.action.openPopup(); }
    catch { await chrome.windows.create({ url:chrome.runtime.getURL('popup.html'), type:'popup', width:470, height:600 }); }
  } catch(error) { console.warn('NDM: the selected link is not supported.'); }
});
