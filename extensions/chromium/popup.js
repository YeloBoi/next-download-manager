import { extensionApi } from './api.js';
const api = extensionApi();
const translations = {
  en:{title:'Send to NDM',urls:'Direct URLs, one per line',extract:'Find links on this page',queue:'Add all to queue',now:'Download all',hint:'Only selected links are shared. NDM does not read your cookies or browsing history.',connected:'Desktop connected',offline:'Waiting for NDM. Open the updated desktop app; the extension reconnects automatically.',accepted:'Accepted by NDM. A paused queue stays paused; use Start queue in NDM.',none:'No HTTP(S) links found.',failed:'Links are saved. Check setup, then retry.',extractError:'This page does not allow link extraction.',collect:'Add to selection',selection:'Saved selection',clear:'Clear selection',remove:'Remove',up:'Move up',down:'Move down',empty:'No saved links. Collect links from any tab.',invalid:'Enter valid HTTP(S) links; keep the total at 100 or fewer.'},
  fa:{title:'ارسال به NDM',urls:'لینک مستقیم؛ هر لینک در یک خط',extract:'پیداکردن لینک‌های این صفحه',queue:'افزودن همه به صف',now:'دانلود همه',hint:'فقط لینک‌های انتخاب‌شده ارسال می‌شوند. NDM کوکی‌ها یا تاریخچهٔ مرور را نمی‌خواند.',connected:'برنامه متصل است',offline:'در انتظار NDM؛ نسخهٔ جدید برنامه را اجرا کنید. ارتباط به‌صورت خودکار برقرار می‌شود.',accepted:'لینک‌ها در NDM پذیرفته شدند. صف مکث‌شده منتظر می‌ماند؛ در NDM شروع صف را بزنید.',none:'لینک HTTP یا HTTPS پیدا نشد.',failed:'لینک‌ها ذخیره‌اند. تنظیم اتصال را بررسی و دوباره ارسال کنید.',extractError:'استخراج لینک از این صفحه مجاز نیست.',collect:'افزودن به انتخاب‌ها',selection:'انتخاب‌های ذخیره‌شده',clear:'پاک‌کردن انتخاب‌ها',remove:'حذف',up:'انتقال به بالا',down:'انتقال به پایین',empty:'لینکی ذخیره نشده است. از هر تب لینک جمع کنید.',invalid:'لینک معتبر HTTP یا HTTPS وارد کنید؛ حداکثر ۱۰۰ لینک مجاز است.'},
};
let {language='en'}=await api.storage.local.get('language');
let text=translations[language]||translations.en;
let saved=[];
let pending=false;
function translate(){text=translations[language]||translations.en;document.documentElement.lang=language;document.documentElement.dir=language==='fa'?'rtl':'ltr';document.querySelector('#language').value=language;document.querySelectorAll('[data-i18n]').forEach(node=>node.textContent=text[node.dataset.i18n]);}
translate();
async function operation(operation, values={}) {
  const result=await api.runtime.sendMessage({type:'selection',operation,...values});
  if(!result?.ok)throw new Error(text.failed);
  return result;
}
function renderSelection() {
  const list=document.querySelector('#selection');list.replaceChildren();
  if(!saved.length){const row=document.createElement('li');row.textContent=text.empty;list.append(row);}
  saved.forEach((url,index)=>{
    const row=document.createElement('li');
    const details=document.createElement('details');const summary=document.createElement('summary');
    const parsed=new URL(url);summary.textContent=parsed.hostname+' · '+(parsed.pathname.split('/').pop()||'/');summary.dir='auto';
    const source=document.createElement('span');source.textContent=url;source.dir='ltr';details.append(summary,source);row.append(details);
    for(const [action,offset,label] of [['move',-1,'up'],['move',1,'down'],['remove',0,'remove']]) {
      const button=document.createElement('button');button.textContent=text[label];button.disabled=pending||(action==='move'&&(offset<0?index===0:index===saved.length-1));
      button.addEventListener('click',()=>busy(async()=>{saved=(await operation(action,{url,offset})).urls;}));row.append(button);
    }
    list.append(row);
  });
}
async function loadSelection(){const result=await operation('get');saved=result.urls;renderSelection();if(result.lastResult==='failed')document.querySelector('#result').textContent=text.failed;}
async function busy(action){
  if(pending)return;pending=true;document.querySelectorAll('button').forEach(button=>button.disabled=true);
  try{await action();}catch{document.querySelector('#result').textContent=text.failed;}
  finally{pending=false;document.querySelectorAll('button').forEach(button=>button.disabled=false);renderSelection();}
}
document.querySelector('#language').addEventListener('change',async event=>{language=event.target.value;await api.storage.local.set({language});translate();renderSelection();});
function renderConnection(value) {
  document.querySelector('#connection').textContent=value?.connected?text.connected:text.offline;
}
renderConnection();
void api.runtime.sendMessage({type:'connection'}).then(renderConnection,()=>renderConnection());
await loadSelection();
api.storage.onChanged.addListener(changes=>{if(changes.selection&&!pending)void loadSelection();if(changes.connectionStatus)renderConnection(changes.connectionStatus.newValue);});
document.querySelector('#extract').addEventListener('click',async()=>{
  try{
    const [tab]=await api.tabs.query({active:true,currentWindow:true});
    const results=await api.scripting.executeScript({target:{tabId:tab.id},func:()=>[...new Set([...document.querySelectorAll('a[href],video[src],audio[src],source[src]')].map(n=>n.href||n.src).filter(s=>/^https?:\/\//i.test(s)))].slice(0,100)});
    const urls=results[0]?.result||[];const list=document.querySelector('#links');list.replaceChildren();
    if(!urls.length){document.querySelector('#result').textContent=text.none;return;}
    for(const url of urls){const label=document.createElement('label');const input=document.createElement('input');input.type='checkbox';input.value=url;const span=document.createElement('span');span.dir='ltr';span.textContent=url;label.append(input,span);list.append(label);}
  }catch{document.querySelector('#result').textContent=text.extractError;}
});
async function collect(){
  const manual=document.querySelector('#urls').value.split(/\r?\n/).map(s=>s.trim()).filter(Boolean);
  const selected=[...document.querySelectorAll('#links input:checked')].map(n=>n.value);
  const urls=[...new Set([...manual,...selected])];
  if(urls.length){
    try{saved=(await operation('add',{urls})).urls;}
    catch{document.querySelector('#result').textContent=text.invalid;return false;}
    document.querySelector('#urls').value='';document.querySelector('#links').replaceChildren();
  }
  return true;
}
async function send(start){if(!await collect())return;const result=await operation('send',{start});saved=result.urls;document.querySelector('#result').textContent=text.accepted;}
document.querySelector('#collect').addEventListener('click',()=>busy(collect));
document.querySelector('#clear').addEventListener('click',()=>busy(async()=>{saved=(await operation('clear')).urls;}));
document.querySelector('#now').addEventListener('click',()=>busy(()=>send(true)));
document.querySelector('#queue').addEventListener('click',()=>busy(()=>send(false)));
