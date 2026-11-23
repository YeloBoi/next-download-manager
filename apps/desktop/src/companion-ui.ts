import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { bytes, escapeHtml as e } from './format.mjs';
import type { Key } from './i18n';

type Translate = (key: Key) => string;
type Device = { id: string; name: string; online: boolean; remember: boolean };
type Transfer = { id: string; device: string; name: string; to_pc: boolean; bytes: number; total: number; speed: number; phase: 'ready'|'paused'|'complete'|'failed'; error: string|null };
type View = { enabled: boolean; devices: Device[]; transfers: Transfer[] };
type Usage = { device: 'pc'|'mobile'; hour: number; bytes: number; category: string };
type Update = { version: string; notes: string; installable: boolean; portable: boolean };
type QR = { path: string; size: number; expires: number; host: string };

const categories: Key[] = ['video','audio','images','apps','games','archives','documents','other'];
const windows = [24,168,720];
const periodLabels: Key[] = ['last24','last7','last30'];
const buttons = (key: Key, action: string, t: Translate, extras='') => `<button type="button" data-action="${action}" ${extras}>${e(t(key))}</button>`;
const preferredKey = 'ndm-update-reminder';

export function companionUI(native: boolean, t: Translate, toast: (message: string) => void) {
  let view: View | null = null;
  let chart: Usage[] = [];
  let qr: QR | null = null;
  let qrExpired = false;
  let period = 24;
  let category = '';
  let next: Update | null = null;
  let snooze = { version:'', until:0 };
  let installing = false;
  let updateProgress: [number, number] = [0, 0];
  let checking = false;
  let polling = false;
  let lastChart = 0;
  let previous: View | null = null;
  let graphMarkup = '';
  let phoneMarkup = '';
  let transferMarkup = '';
  try { snooze = JSON.parse(localStorage.getItem(preferredKey) || 'null') ?? snooze; } catch { /* Previous preference was unreadable. */ }

  function settings() {
    return `<section class="settings-section"><div><h2>${e(t('phone'))}</h2><p>${e(t('phoneHint'))}</p></div><div id="phone-panel" class="settings-fields"></div></section>
      <section class="settings-section"><div><h2>${e(t('filesShared'))}</h2><p>${e(t('transferHint'))}</p></div><div id="transfer-panel" class="settings-fields"></div></section>
      <section class="settings-section"><div><h2>${e(t('usage'))}</h2><p>${e(t('usageHint'))}</p></div><div class="settings-fields"><div class="usage-filters"><select id="usage-window" aria-label="${e(t('usage'))}">${windows.map((value,i)=>`<option value="${value}" ${value===period?'selected':''}>${e(t(periodLabels[i]))}</option>`).join('')}</select><select id="usage-category" aria-label="${e(t('extensionType'))}"><option value="">${e(t('allTypes'))}</option>${categories.map(key=>`<option value="${key}" ${category===key?'selected':''}>${e(t(key))}</option>`).join('')}</select></div><div id="usage-chart"></div></div></section>
      <div class="settings-tools">${buttons('checkUpdates','feature-check-update',t,native?'':'disabled')}</div>`;
  }

  function paintAfterShell() {
    graphMarkup = phoneMarkup = transferMarkup = '';
    document.querySelector<HTMLSelectElement>('#usage-window')?.addEventListener('change',event=>{period=Number((event.target as HTMLSelectElement).value);paintGraph();});
    document.querySelector<HTMLSelectElement>('#usage-category')?.addEventListener('change',event=>{category=(event.target as HTMLSelectElement).value;paintGraph();});
    paintBanner();
    paintSettings();
    if (native && document.querySelector('#phone-panel')) void poll();
  }

  function paintGraph() {
    const panel=document.querySelector('#usage-chart');
    if (!panel) return;
    const hour=Math.floor(Date.now()/3600000);
    const buckets=period===24 ? 12 : period/24;
    const width=period/buckets;
    const since=hour-period+1;
    const data=Array.from({length:buckets},()=>({pc:0,mobile:0}));
    for(const row of chart) {
      if (row.hour<since||row.hour>hour||(category&&category!==row.category)) continue;
      if(row.device==='pc'||row.device==='mobile') data[Math.floor((row.hour-since)/width)][row.device]+=row.bytes;
    }
    const totals=data.reduce((acc,v)=>({pc:acc.pc+v.pc,mobile:acc.mobile+v.mobile}),{pc:0,mobile:0});
    const max=Math.max(1,...data.map(v=>v.pc+v.mobile));
    const date=(h:number)=>new Intl.DateTimeFormat(document.documentElement.lang,period===24?{hour:'numeric',minute:'2-digit'}:{month:'short',day:'numeric'}).format(h*3600000);
    const markup=`<div class="usage-summary"><span>${e(t('usageTotal'))}</span><strong dir="ltr">${bytes(totals.pc+totals.mobile)}</strong>
      <span class="usage-key"><i class="usage-pc"></i>${e(t('usagePc'))} <b dir="ltr">${bytes(totals.pc)}</b></span><span class="usage-key"><i class="usage-mobile"></i>${e(t('usagePhone'))} <b dir="ltr">${bytes(totals.mobile)}</b></span></div>
      ${totals.pc+totals.mobile?`<div class="usage-plot" dir="ltr" aria-label="${e(t('usage'))}">${data.map((item,i)=>{
        const title=`${date(since+i*width)} · ${t('usagePc')} ${bytes(item.pc)} · ${t('usagePhone')} ${bytes(item.mobile)}`;
        return `<div class="usage-column" tabindex="0" aria-label="${e(title)}" title="${e(title)}"><i class="usage-pc" style="height:${item.pc/max*100}%"></i><i class="usage-mobile" style="height:${item.mobile/max*100}%"></i></div>`;
      }).join('')}</div><div class="usage-axis" dir="ltr"><span>${e(date(since))}</span><span>${e(date(hour))}</span></div>`:`<p class="feature-muted">${e(t('noUsage'))}</p>`}`;
    if (markup!==graphMarkup) {graphMarkup=markup;panel.innerHTML=markup;}
  }

  function paintSettings() {
    const phone=document.querySelector('#phone-panel');
    if (!phone) return;
    if (qr&&Date.now()>=qr.expires*1000) { qr=null; qrExpired=true; }
    const devices=view?.devices||[];
    const markup=`<div class="feature-actions">${buttons(view?.enabled?'phoneDisable':'phoneEnable','feature-enable-phone',t,(!native||!view)?'disabled':'')}${view?.enabled?buttons('pairPhone','feature-pair-phone',t):''}</div>
      ${qr?`<figure class="pair-qr"><svg viewBox="0 0 ${qr.size} ${qr.size}" aria-label="${e(t('pairPhone'))}" role="img"><path d="${e(qr.path)}"></path></svg><figcaption>${e(t('pairHelp'))}<br><span dir="ltr">${e(qr.host)}</span></figcaption></figure>`:qrExpired?`<p class="feature-muted">${e(t('pairExpired'))}</p>`:''}
      ${devices.length?devices.map(d=>`<article class="feature-row"><div><strong>${e(d.name)}</strong><small class="${d.online?'feature-online':''}">${e(t(d.online?'online':'offline'))}</small></div><div class="feature-actions">${buttons('sendFiles','feature-send',t,`data-id="${e(d.id)}" ${view?.enabled?'':'disabled'}`)}${buttons('revoke','feature-revoke',t,`data-id="${e(d.id)}"`)}</div></article>`).join(''):`<p class="feature-muted">${e(t('noPaired'))}</p>`}`;
    if(markup!==phoneMarkup) {phoneMarkup=markup;phone.innerHTML=markup;}
    const panel=document.querySelector('#transfer-panel');
    const transfers=view?.transfers||[];
    const transferHtml=transfers.length?transfers.map(x=>{
      const id=`data-id="${e(x.id)}"`;
      const ratio=x.phase==='complete'?100:x.total?Math.min(100,x.bytes/x.total*100):0;
      const label=x.phase==='complete'?'completed':x.phase==='failed'?'failed':x.phase==='paused'?'paused':x.speed>0?'transfer':'waitingPhone';
      return `<article class="feature-row transfer-item"><div><strong title="${e(x.name)}">${e(x.name)}</strong><small>${e(t(x.to_pc?'incoming':'outgoing'))} · ${e(t(label))}</small><progress value="${ratio}" max="100" aria-label="${e(t('status'))}"></progress><small dir="ltr">${Math.floor(ratio)}% · ${bytes(x.bytes)} / ${bytes(x.total)}${x.speed>0?` · ${bytes(x.speed)}/s`:''}</small>${x.error?`<small class="form-error">${e(x.error)}</small>`:''}</div><div class="feature-actions">${x.phase==='complete'?(x.to_pc?buttons('open','feature-transfer-open',t,id)+buttons('reveal','feature-transfer-reveal',t,id):''):buttons(x.phase==='ready'?'cancel':'retry',x.phase==='ready'?'feature-transfer-cancel':'feature-transfer-retry',t,id)}${x.phase!=='ready'?buttons('clear','feature-transfer-clear',t,id):''}</div></article>`;
    }).join(''):`<p class="feature-muted">${e(t('noTransfers'))}</p>`;
    if(panel&&transferHtml!==transferMarkup){
      const active=document.activeElement as HTMLButtonElement|null;
      const focus=panel.contains(active)?[active?.dataset.id,active?.dataset.action]:null;
      transferMarkup=transferHtml;
      panel.innerHTML=transferHtml;
      if(focus) [...panel.querySelectorAll<HTMLButtonElement>('button')].find(b=>b.dataset.id===focus[0]&&b.dataset.action===focus[1])?.focus({preventScroll:true});
    }
    paintGraph();
  }

  async function poll(report = false) {
    if(polling)return;
    polling=true;
    try {
      view=await invoke<View>('companion_view');
      if (previous) {
        const finished=new Set(previous.transfers.filter(x=>x.phase==='complete').map(x=>x.id));
        const completed=view.transfers.find(x=>x.phase==='complete'&&!finished.has(x.id));
        if(completed)toast(`${t(completed.to_pc?'receivedFile':'sentFile')}: ${completed.name}`);
      }
      if(qr&&view.devices.some(d=>!previous?.devices.some(p=>p.id===d.id&&p.online)&&d.online))qr=null;
      previous=view;
      if(Date.now()-lastChart>12000&&document.querySelector('#usage-chart')) {
        chart=await invoke<Usage[]>('download_usage');
        lastChart=Date.now();
      }
    } catch(error) { if(report)toast(String(error)); }
    finally {polling=false;paintSettings();}
  }

  function paintBanner() {
    const banner=document.querySelector<HTMLElement>('#update-banner');
    const mark=document.querySelector<HTMLElement>('#update-marker');
    if(!banner)return;
    const hidden=!!next&&snooze.version===next.version&&Date.now()<snooze.until;
    if(mark) mark.hidden=!hidden;
    banner.hidden=!next||hidden;
    if(!next||hidden){banner.innerHTML='';delete banner.dataset.markup;return;}
    const prefix=`<div><strong>${e(t('updateLabel'))}</strong> <span dir="ltr">v${e(next.version)}</span>`;
    const [received,total]=updateProgress;
    const markup=installing?`${prefix}<small>${total&&received>=total?e(t('applyingUpdate')):`${bytes(received)} / ${total?bytes(total):'…'}`}</small></div><progress max="${total||1}" ${total?`value="${received}"`:''} aria-label="${e(t('update'))}"></progress>`:
      `${prefix}${next.portable?`<small>${e(t('updatePortable'))}</small>`:''}</div><div class="feature-actions">${buttons('whatsNew','feature-update-notes',t)}${buttons('later','feature-update-later',t)}${buttons(next.installable?'update':'getUpdate','feature-install',t,'class="primary"')}</div>`;
    if(banner.dataset.markup!==markup){banner.dataset.markup=markup;banner.innerHTML=markup;}
  }

  async function checkUpdate(manual=false) {
    if (installing||checking)return;
    checking=true;
    try {
      next=await invoke<Update|null>('check_update');
      if(manual&&next){snooze.until=0;localStorage.setItem(preferredKey,JSON.stringify(snooze));}
      if(manual&&!next)toast(t('checkUpdatesNone'));
      paintBanner();
    } catch(error) {if(manual)toast(`${t('checkUpdatesError')} ${String(error)}`);}
    finally {checking=false;}
  }

  async function handle(action: string, node: HTMLElement) {
    if (!native) return;
    const id=node.dataset.id;
    if(action==='feature-enable-phone') {
      await invoke('companion_enable',{enabled:!view?.enabled});
      qr=null;
      qrExpired=false;
    } else if(action==='feature-pair-phone') {
      qr=await invoke<QR>('companion_pair');
      qrExpired=false;
    } else if(action==='feature-revoke') {
      await invoke('companion_revoke',{id});
    } else if(action==='feature-send') {
      const files=await open({multiple:true,directory:false});
      if(Array.isArray(files)&&files.length)await invoke('companion_send',{device:id,files});
    } else if(action.startsWith('feature-transfer-')) {
      await invoke('companion_transfer',{id,action:action.slice('feature-transfer-'.length)});
    } else if(action==='feature-check-update') {
      await checkUpdate(true);return;
    } else if(action==='feature-update-later'&&next) {
      snooze={version:next.version,until:Date.now()+86400000};
      localStorage.setItem(preferredKey,JSON.stringify(snooze));paintBanner();return;
    } else if(action==='feature-update-notes'&&next) {
      const dialog=document.querySelector<HTMLDialogElement>('#release-dialog')!;
      dialog.innerHTML=`<form method="dialog"><div class="dialog-heading"><h2 id="release-title">${e(t('whatsNew'))} · <bdi>v${e(next.version)}</bdi></h2></div><pre class="release-notes">${e(next.notes)}</pre><div class="dialog-footer"><button>${e(t('close'))}</button></div></form>`;
      dialog.showModal();return;
    } else if(action==='feature-install'&&next) {
      if(!next.installable){await invoke('open_release');return;}
      if(installing)return;
      installing=true;updateProgress=[0,0];paintBanner();
      try { await invoke('install_update',{version:next.version}); } finally {installing=false;paintBanner();}
      return;
    }
    await poll(true);
  }

  async function start() {
    await listen<[number,number]>('ndm://update-progress',({payload})=>{updateProgress=payload;paintBanner();});
    void checkUpdate();
    void poll();
    setInterval(()=>{void poll();paintBanner();},2000);
    setInterval(()=>void checkUpdate(),60*60*1000);
  }
  return { settings, paintAfterShell, handle, start };
}
