import { request, sendNative } from './protocol.js';
const translations = {
  en:{title:'Send to NDM',urls:'Direct URLs, one per line',extract:'Find links on this page',queue:'Add to queue',now:'Download now',hint:'Only selected links are shared. NDM does not read your cookies or browsing history.',connected:'Desktop connected',offline:'Open NDM and register the native host to connect.',accepted:'Accepted by NDM.',none:'No HTTP(S) links found.',failed:'Could not send links: ',extractError:'This page does not allow link extraction.'},
  fa:{title:'ارسال به NDM',urls:'لینک مستقیم؛ هر لینک در یک خط',extract:'پیداکردن لینک‌های این صفحه',queue:'افزودن به صف',now:'شروع دانلود',hint:'فقط لینک‌های انتخاب‌شده ارسال می‌شوند. NDM کوکی‌ها یا تاریخچهٔ مرور را نمی‌خواند.',connected:'برنامه متصل است',offline:'NDM را باز کنید و میزبان Native Messaging را ثبت کنید.',accepted:'لینک‌ها در NDM پذیرفته شدند.',none:'لینک HTTP یا HTTPS پیدا نشد.',failed:'ارسال لینک ناموفق بود: ',extractError:'استخراج لینک از این صفحه مجاز نیست.'},
};
let {language='en'}=await chrome.storage.local.get('language');
let text=translations[language]||translations.en;
function translate(){text=translations[language]||translations.en;document.documentElement.lang=language;document.documentElement.dir=language==='fa'?'rtl':'ltr';document.querySelector('#language').value=language;document.querySelectorAll('[data-i18n]').forEach(node=>node.textContent=text[node.dataset.i18n]);}
translate();
document.querySelector('#language').addEventListener('change',async event=>{language=event.target.value;await chrome.storage.local.set({language});translate();});
const {pendingUrls=[]}=await chrome.storage.session.get('pendingUrls');
document.querySelector('#urls').value=pendingUrls.join('\n');
await chrome.storage.session.remove('pendingUrls');
try{await sendNative(request('ping'));document.querySelector('#connection').textContent=text.connected;}catch{document.querySelector('#connection').textContent=text.offline;}
document.querySelector('#extract').addEventListener('click',async()=>{
  try{
    const [tab]=await chrome.tabs.query({active:true,currentWindow:true});
    const results=await chrome.scripting.executeScript({target:{tabId:tab.id},func:()=>[...new Set([...document.querySelectorAll('a[href],video[src],audio[src],source[src]')].map(n=>n.href||n.src).filter(s=>/^https?:\/\//i.test(s)))].slice(0,100)});
    const urls=results[0]?.result||[];const list=document.querySelector('#links');list.replaceChildren();
    if(!urls.length){document.querySelector('#result').textContent=text.none;return;}
    for(const url of urls){const label=document.createElement('label');const input=document.createElement('input');input.type='checkbox';input.value=url;const span=document.createElement('span');span.dir='ltr';span.textContent=url;label.append(input,span);list.append(label);}
  }catch{document.querySelector('#result').textContent=text.extractError;}
});
async function send(start){
  const buttons=[document.querySelector('#now'),document.querySelector('#queue')];buttons.forEach(b=>b.disabled=true);
  try{
    const manual=document.querySelector('#urls').value.split(/\r?\n/).map(s=>s.trim()).filter(Boolean);
    const selected=[...document.querySelectorAll('#links input:checked')].map(n=>n.value);
    await sendNative(request('add',[...new Set([...manual,...selected])],start));
    document.querySelector('#result').textContent=text.accepted;document.querySelector('#urls').value='';document.querySelector('#links').replaceChildren();
  }catch(error){document.querySelector('#result').textContent=text.failed+String(error.message||error);}
  finally{buttons.forEach(b=>b.disabled=false);}
}
document.querySelector('#now').addEventListener('click',()=>send(true));
document.querySelector('#queue').addEventListener('click',()=>send(false));
