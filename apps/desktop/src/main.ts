import '@fontsource-variable/inter';
import '@fontsource-variable/vazirmatn';
import './style.css';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { open } from '@tauri-apps/plugin-dialog';
import { createElement, ArrowDownToLine, Plus, Layers, Play, Pause, Square, Search, Settings as Gear, FolderOpen, Check, CircleAlert, Clock, ChevronDown, X, MoreHorizontal, ArrowUp, ArrowDown, File, Link, Menu, RotateCcw, Trash2, Copy, ListOrdered, Minus, Maximize2, Minimize2 } from 'lucide';
import { bytes, percent, eta, escapeHtml as e, parseLinks } from './format.mjs';
import { translator, type Key } from './i18n';
import type { Download, Settings, Snapshot, Status } from './types';
import { companionUI } from './companion-ui';

const native = isTauri();
const icons: Record<string, any> = { download:ArrowDownToLine, plus:Plus, layers:Layers, play:Play, pause:Pause, stop:Square, search:Search, settings:Gear, folder:FolderOpen, check:Check, error:CircleAlert, clock:Clock, down:ChevronDown, close:X, more:MoreHorizontal, up:ArrowUp, arrowDown:ArrowDown, file:File, link:Link, menu:Menu, retry:RotateCcw, trash:Trash2, copy:Copy, list:ListOrdered };
const icon = (name: string) => createElement(icons[name] || File, { width:18, height:18, 'stroke-width':1.65, 'aria-hidden':'true' }).outerHTML;
const mark = '<img src="/brand/mark-blue.png" alt="" draggable="false">';
const appWindow = native ? getCurrentWindow() : null;
let maximized = false;
let focused = true;
const root = document.querySelector<HTMLDivElement>('#app')!;
let state: Snapshot = { downloads:[], running:false, draining:false, settings:{ directory:'', language:'en', theme:'dark', compact:false, concurrent_downloads:1, connections_per_file:1, global_connections:16, host_connections:8, speed_limit:0, resume_on_launch:false, stop_on_error:false, schedule_enabled:false, schedule_start:0, schedule_end:0, close_to_tray:false, confirmless_add:false } };
let t = translator('en');
const features=companionUI(native,key=>t(key),toast);
let filter = 'all';
let search = '';
let selected = new Set<string>();
let detailId: string | null = null;
let settingsView = false;
let detailOpen = true;
let pending = false;
let browserConnection = { seen:false, connected:false };
let checkingBrowser = false;
let toastTimer: ReturnType<typeof setTimeout>;
const history = new Map<string, { time:number; speed:number }[]>();
const filters: [string,string][] = [['all','download'],['downloading','play'],['queued','list'],['paused','pause'],['completed','check'],['failed','error']];

function button(action:string, label:Key, glyph?:string, attrs='') {
  return `<button type="button" data-action="${action}" ${attrs}>${glyph?icon(glyph):''}<span>${e(t(label))}</span></button>`;
}
function countFor(key:string) { return state.downloads.filter(d => key === 'all' || (key === 'downloading' ? active(d) : key === 'paused' ? ['paused','stopped'].includes(d.status) : d.status === key)).length; }
function active(d:Download) { return ['connecting','downloading'].includes(d.status); }
function displayName(d:Download) { return d.name || t('newDownload'); }
function queueLabel():Key { return state.draining ? 'queueDraining' : state.running ? 'queueRunning' : 'queuePaused'; }
function applyAppearance() {
  t = translator(state.settings.language);
  document.documentElement.lang = state.settings.language;
  document.documentElement.dir = state.settings.language === 'fa' ? 'rtl' : 'ltr';
  document.documentElement.dataset.theme = state.settings.theme;
  document.documentElement.dataset.compact = String(state.settings.compact);
}

function shell() {
  applyAppearance();
  root.innerHTML = `
    <header class="titlebar" aria-label="NDM" data-tauri-drag-region>
      <div class="window-identity" data-tauri-drag-region>${mark}<span data-tauri-drag-region>NDM</span><span class="window-product" data-tauri-drag-region>Next Download Manager</span></div>
      <div class="window-drag-space" data-tauri-drag-region></div>
      <div class="window-controls" role="group" aria-label="${e(t('windowControls'))}">
        <button data-window="minimize" aria-label="${e(t('minimize'))}" title="${e(t('minimize'))}" ${!native?'disabled':''}>${createElement(Minus,{'aria-hidden':'true'}).outerHTML}</button>
        <button data-window="maximize" aria-label="${e(t(maximized?'restore':'maximize'))}" title="${e(t(maximized?'restore':'maximize'))}" ${!native?'disabled':''}>${createElement(maximized?Minimize2:Maximize2,{'aria-hidden':'true'}).outerHTML}</button>
        <button data-window="close" class="window-close" aria-label="${e(t('close'))}" title="${e(t('close'))}" ${!native?'disabled':''}>${icon('close')}</button>
      </div>
    </header>
    <aside class="sidebar" aria-label="${e(t('menu'))}">
      <a class="brand" href="#downloads" aria-label="NDM — Next Download Manager"><span class="app-badge"><img class="logo-dark" src="/brand/app-dark.png" alt="" draggable="false"><img class="logo-light" src="/brand/app-light.png" alt="" draggable="false"></span><span>NDM<small>Next Download Manager</small></span></a>
      <div class="nav-caption">${e(t('local'))}</div>
      <nav>${filters.map(([key,glyph]) => `<button data-filter="${key}" class="nav-item ${!settingsView&&filter===key?'current':''}" ${!settingsView&&filter===key?'aria-current="page"':''}>${icon(glyph)}<span>${e(t(key as Key))}</span><span class="nav-count">${countFor(key)}</span></button>`).join('')}</nav>
      <div class="sidebar-bottom">
        <section class="extension-note" id="browser-connection" aria-label="${e(t('extension'))}" ${browserConnection.connected?'hidden':''}>${extensionMarkup()}</section>
        <button class="nav-item ${settingsView?'current':''}" data-action="settings">${icon('settings')}<span>${e(t('settings'))}</span><i id="update-marker" aria-label="${e(t('updateLabel'))}" title="${e(t('updateLabel'))}" hidden></i></button>
        <div class="build-label"><span>${e(t('version'))}</span><span class="local-indicator">${icon('folder')}</span></div>
      </div>
    </aside>
    <div class="workspace">
      <header class="topbar"><div class="breadcrumb"><button class="icon-button mobile-nav" data-action="nav" aria-label="${e(t('menu'))}">${icon('menu')}</button><span>NDM</span><span class="slash">/</span><span>${e(t(settingsView?'settings':'downloads'))}</span></div><span class="queue-state"><span class="state-dot ${state.running?'running':''}"></span><span data-queue-label>${e(t(queueLabel()))}</span></span></header>
      <main>${settingsView ? settingsMarkup() : downloadsMarkup()}</main>
      <aside id="update-banner" class="update-banner" aria-live="polite" hidden></aside>
      <footer class="statusbar"><span class="total-speed">${icon('download')} <span data-total-speed dir="ltr">${bytes(totalSpeed())}/s</span></span><span data-summary>${summary()}</span><span class="status-privacy">${e(t('privacy'))}</span></footer>
    </div>
    <div id="toast" role="status" aria-live="polite" hidden></div>
    <dialog id="add-dialog" aria-labelledby="add-title"></dialog>
    <dialog id="confirm-dialog" aria-labelledby="confirm-title"></dialog>
    <dialog id="release-dialog" aria-labelledby="release-title"></dialog>
    <div id="context-menu" class="context-menu" role="menu" hidden></div>
  `;
  if (!settingsView) { updateList(); updateDetails(); }
  bindShell();
  features.paintAfterShell();
}

async function syncWindowState() {
  if (!appWindow) return;
  maximized = await appWindow.isMaximized();
  document.documentElement.dataset.maximized = String(maximized);
  const control = document.querySelector<HTMLButtonElement>('[data-window="maximize"]');
  if (control) {
    control.innerHTML = createElement(maximized?Minimize2:Maximize2,{'aria-hidden':'true'}).outerHTML;
    control.title = t(maximized?'restore':'maximize');
    control.setAttribute('aria-label',control.title);
  }
}

document.addEventListener('click',async event=>{
  const control = (event.target as Element).closest<HTMLElement>('[data-window]');
  if (!control || !appWindow) return;
  try {
    if (control.dataset.window === 'minimize') await appWindow.minimize();
    else if (control.dataset.window === 'maximize') { await appWindow.toggleMaximize(); await syncWindowState(); }
    // close() preserves the existing close-to-tray and graceful queue shutdown handler.
    else if (control.dataset.window === 'close') await appWindow.close();
  } catch (error) { toast(String(error)); }
});

function downloadsMarkup() {
  return `<div class="title-row"><div><h1>${e(t(filter as Key))}</h1><p>${e(t('subtitle'))}</p></div><div class="title-actions">${button('multiple','multiple','layers')}${button('add','add','plus','class="primary"')}</div></div>
    ${!native?`<div class="preview-notice">${icon('error')}${e(t('webPreview'))}</div>`:''}
    <div class="toolbar"><div class="queue-controls">${button('toggle-queue',state.running?'pauseQueue':'start',state.running?'pause':'play',!native?'disabled':'')}
      <button data-action="queue-menu" class="icon-button" aria-label="${e(t('tools'))}" aria-haspopup="menu">${icon('down')}</button>
      <span class="control-divider"></span><button data-action="stop-queue" class="icon-button" aria-label="${e(t('stopQueue'))}" title="${e(t('stopQueue'))}" ${!native?'disabled':''}>${icon('stop')}</button>
    </div><label class="search">${icon('search')}<input id="search" type="search" placeholder="${e(t('search'))}" aria-label="${e(t('search'))}" value="${e(search)}"><kbd>Ctrl K</kbd></label></div>
    <div id="bulk" class="bulk-toolbar" hidden></div>
    <div class="download-layout"><section class="download-list" aria-label="${e(t('downloads'))}">
      <div class="list-meta"><span id="list-count"></span><button class="text-button" data-action="details">${e(t('details'))}${icon('down')}</button></div>
      <div class="table-scroll"><table><thead><tr><th class="select-column"><input id="select-all" type="checkbox" aria-label="${e(t('allSelected'))}"></th><th>${e(t('name'))}</th><th class="number">${e(t('size'))}</th><th class="number speed-col">${e(t('speed'))}</th><th class="number eta-col">${e(t('eta'))}</th><th>${e(t('status'))}</th><th class="row-action-column"></th></tr></thead><tbody id="download-rows"></tbody></table><div id="empty"></div></div>
    </section><aside id="details" class="details-panel" aria-label="${e(t('details'))}"></aside></div>`;
}

function visible() { return state.downloads.filter(d => (filter==='all' || (filter==='downloading'?active(d):filter==='paused'?['paused','stopped'].includes(d.status):d.status===filter)) && `${d.name} ${d.url}`.toLowerCase().includes(search.toLowerCase())); }
function totalSpeed() { return state.downloads.reduce((sum,d)=>sum+(active(d)?d.speed:0),0); }
function summary() { return `${state.downloads.filter(active).length} ${t('active')}<span class="summary-dot">·</span>${countFor('queued')} ${t('waiting')}`; }

function updateList() {
  if (settingsView) return;
  const items = visible();
  const current = new Set(state.downloads.map(d=>d.id)); selected = new Set([...selected].filter(id=>current.has(id)));
  document.querySelector('#list-count')!.textContent = `${items.length} ${t('count')}`;
  document.querySelector('#download-rows')!.innerHTML = items.map(d => {
    const progress = percent(d);
    return `<tr data-id="${d.id}" draggable="true" tabindex="0" class="${detailId===d.id?'focused-row':''}" aria-selected="${selected.has(d.id)}">
      <td class="select-column"><input type="checkbox" data-select="${d.id}" aria-label="${e(t('select'))} ${e(displayName(d))}" ${selected.has(d.id)?'checked':''}></td>
      <td class="identity"><div class="file-identity">${icon('file')}<div><strong title="${e(displayName(d))}">${e(displayName(d))}</strong><small dir="ltr">${e(d.url)}</small></div></div></td>
      <td class="number progress-cell"><span dir="ltr">${bytes(d.downloaded)} <span class="muted">/ ${bytes(d.total)}</span></span><div class="progress-track" ${progress===null?'aria-label="'+e(t('unknown'))+'"':`role="progressbar" aria-valuenow="${Math.round(progress)}" aria-valuemin="0" aria-valuemax="100" aria-label="${e(displayName(d))}"`}><span style="width:${progress??0}%"></span></div></td>
      <td class="number speed-col" dir="ltr">${active(d)?bytes(d.speed)+'/s':'—'}</td><td class="number eta-col" dir="ltr">${eta(d)}</td>
      <td><span class="status status-${d.status}">${icon(statusIcon(d.status))}<span>${e(t(d.status))}</span></span></td>
      <td><button data-row-menu="${d.id}" class="icon-button" aria-label="${e(t('actions'))} ${e(displayName(d))}" aria-haspopup="menu">${icon('more')}</button></td></tr>`;
  }).join('');
  document.querySelector('#empty')!.innerHTML = items.length ? '' : state.downloads.length ? `<div class="filtered-empty">${e(t('noResults'))}</div>` : `<div class="empty-state"><div class="empty-symbol">${mark}</div><h2>${e(t('empty'))}</h2><p>${e(t('emptyText'))}</p>${button('add','emptyAction','plus','class="primary"')}<div class="empty-key"><kbd>Ctrl</kbd><span>+</span><kbd>N</kbd></div></div>`;
  const bulk = document.querySelector<HTMLDivElement>('#bulk')!;
  bulk.hidden = selected.size===0;
  bulk.innerHTML = `<span>${selected.size} ${e(t('selected'))}</span>${button('bulk-resume','resume','play')}${button('bulk-pause','pause','pause')}${button('bulk-stop','stop','stop')}${button('bulk-remove','remove','trash')}<button data-action="clear-selection" class="icon-button" aria-label="${e(t('close'))}">${icon('close')}</button>`;
  const all = document.querySelector<HTMLInputElement>('#select-all')!;
  all.checked = items.length>0&&items.every(d=>selected.has(d.id)); all.indeterminate = items.some(d=>selected.has(d.id))&&!all.checked;
  document.querySelectorAll<HTMLElement>('[data-filter]').forEach(n=>{ const key=n.dataset.filter!; n.querySelector('.nav-count')!.textContent=String(countFor(key)); });
}
function statusIcon(status:Status) { return ({ queued:'clock', connecting:'clock', downloading:'download', paused:'pause', stopped:'stop', completed:'check', failed:'error' })[status]; }

function updateDetails() {
  const panel = document.querySelector<HTMLElement>('#details'); if (!panel) return;
  panel.hidden = !detailOpen;
  document.querySelector('.download-layout')?.classList.toggle('without-details',!detailOpen);
  const d=state.downloads.find(d=>d.id===detailId);
  if (!d) { panel.innerHTML = `<div class="detail-heading"><h2>${e(t('details'))}</h2></div><div class="detail-placeholder">${icon('file')}<p>${e(t('noSelection'))}</p></div>`; return; }
  const progress=percent(d);
  const values=history.get(d.id)??[];
  const max=Math.max(1024,...values.map(p=>p.speed));
  const points=values.map((p,i)=>`${42+i*210/Math.max(1,values.length-1)},${108-p.speed/max*74}`).join(' ');
  panel.innerHTML = `<div class="detail-heading"><h2>${e(t('details'))}</h2><button class="icon-button" data-action="details" aria-label="${e(t('close'))}">${icon('close')}</button></div>
    <div class="detail-file">${icon('file')}<h3>${e(displayName(d))}</h3><span class="status status-${d.status}">${icon(statusIcon(d.status))}${e(t(d.status))}</span></div>
    <div class="detail-progress"><div><span>${e(t('transferred'))}</span><strong dir="ltr">${progress===null?'—':progress.toFixed(1)+'%'}</strong></div><div class="progress-track"><span style="width:${progress??0}%"></span></div><small dir="ltr">${bytes(d.downloaded)} / ${bytes(d.total)}</small></div>
    <div class="detail-actions">${button(active(d)?'item-pause':'item-resume',active(d)?'pause':'resume',active(d)?'pause':'play',d.status==='completed'?'disabled':'')}${button('item-reveal','reveal','folder',d.path?'':'disabled')}</div>
    <section class="speed-chart"><h3>${e(t('chart'))}</h3><svg viewBox="0 0 264 132" role="img" aria-label="${e(t('chart'))}"><path class="chart-grid" d="M42 34H252M42 71H252M42 108H252"/><text x="0" y="38">${bytes(max)}</text><text x="0" y="112">0 B/s</text><polyline class="chart-line" points="${points}"/><text x="42" y="128">−60s</text><text x="222" y="128">0s</text></svg></section>
    <dl class="metadata"><dt>${e(t('source'))}</dt><dd dir="ltr">${e(d.url)}</dd><dt>${e(t('path'))}</dt><dd dir="ltr">${e(d.path||d.directory)}</dd><dt>${e(t('connections'))}</dt><dd dir="ltr">${d.connections||'—'}</dd><dt>${e(t('resumable'))}</dt><dd>${e(t(d.connections?d.resumable?'yes':'no':'unknown'))}</dd><dt>${e(t('sha256'))}</dt><dd dir="ltr" class="hash">${e(d.sha256||'—')}</dd></dl>
    ${d.error?`<div class="error-panel" role="status"><strong>${icon('error')}${e(t('error'))}</strong><p>${e(d.error)}</p>${button('item-resume','resume','retry')}</div>`:''}
    ${button('item-copy','copy','copy',d.path?'class="text-button"':'class="text-button" disabled')}`;
}

function field(name:string,label:Key,type:string,value:string|number,attrs='') { return `<label class="field"><span>${e(t(label))}</span><input type="${type}" name="${name}" value="${e(value)}" ${attrs}></label>`; }
function toggle(name:string,label:Key,value:boolean) { return `<label class="toggle-row"><span>${e(t(label))}</span><input type="checkbox" name="${name}" role="switch" ${value?'checked':''}></label>`; }
function timeValue(minute:number) { return `${String(Math.floor(minute/60)).padStart(2,'0')}:${String(minute%60).padStart(2,'0')}`; }
function section(title:Key,hint:Key,content:string) { return `<section class="settings-section"><div><h2>${e(t(title))}</h2><p>${e(t(hint))}</p></div><div class="settings-fields">${content}</div></section>`; }
function settingsMarkup() {
  const s=state.settings;
  return `<div class="title-row"><div><h1>${e(t('settings'))}</h1><p>${e(t('privacy'))}</p></div>${button('save-settings','save',undefined,'class="primary"')}</div><form id="settings-form">
    ${section('general','generalHint',`<div class="field-grid"><label class="field"><span>${e(t('language'))}</span><select name="language"><option value="en" ${s.language==='en'?'selected':''}>English</option><option value="fa" ${s.language==='fa'?'selected':''}>فارسی</option></select></label><label class="field"><span>${e(t('theme'))}</span><select name="theme">${['dark','light','system'].map(theme=>`<option value="${theme}" ${s.theme===theme?'selected':''}>${e(t(theme as Key))}</option>`).join('')}</select></label></div>${toggle('compact','compact',s.compact)}`)}
    ${section('destination','destinationHint',`<label class="field"><span>${e(t('folder'))}</span><div class="joined"><input name="directory" value="${e(s.directory)}" dir="ltr" required>${button('browse-settings','browse','folder')}</div></label>${toggle('confirmless_add','confirmLess',s.confirmless_add)}`)}
    ${section('network','networkHint',`<div class="field-grid">${field('concurrent_downloads','concurrent','number',s.concurrent_downloads,'min="1" max="8" required')}${field('connections_per_file','perFile','number',s.connections_per_file,'min="1" max="8" required')}${field('global_connections','global','number',s.global_connections,'min="1" max="64" required')}${field('host_connections','perHost','number',s.host_connections,'min="1" max="32" required')}</div>${field('speed_limit','limit','number',s.speed_limit/1024,'min="0" step="1" required')}<label class="field"><span>${e(t('profile'))}</span><select name="profile"><option value="-1">—</option><option value="0">${e(t('unlimited'))}</option><option value="1024">${e(t('work'))}</option><option value="256">${e(t('gaming'))}</option></select></label>`)}
    ${section('recovery','recoveryHint',`${toggle('resume_on_launch','autoResume',s.resume_on_launch)}${toggle('stop_on_error','stopError',s.stop_on_error)}${toggle('close_to_tray','tray',s.close_to_tray)}`)}
    ${section('schedule','scheduleHint',`${toggle('schedule_enabled','scheduleEnabled',s.schedule_enabled)}<div class="field-grid">${field('schedule_start','from','time',timeValue(s.schedule_start),'required')}${field('schedule_end','to','time',timeValue(s.schedule_end),'required')}</div>`)}
    </form>${features.settings()}<div class="settings-tools"><h2>${e(t('tools'))}</h2>${button('import','import','link',!native?'disabled':'')}${button('export','export','list',!native?'disabled':'')}</div>`;
}

function bindShell() {
  document.querySelector<HTMLInputElement>('#search')?.addEventListener('input',event=>{search=(event.target as HTMLInputElement).value;updateList();});
  document.querySelector('#select-all')?.addEventListener('change', event=>{for(const d of visible()){if((event.target as HTMLInputElement).checked)selected.add(d.id);else selected.delete(d.id);}updateList();});
  document.querySelector('#settings-form')?.addEventListener('submit',event=>{event.preventDefault();void saveSettings();});
  document.querySelector<HTMLSelectElement>('select[name="profile"]')?.addEventListener('change',event=>{const value=(event.target as HTMLSelectElement).value;if(value!=='-1')document.querySelector<HTMLInputElement>('[name="speed_limit"]')!.value=value;});
}

function toast(message:string) {
  const target=document.querySelector<HTMLElement>('#toast')!; target.textContent=message;target.hidden=false;
  clearTimeout(toastTimer);toastTimer=setTimeout(()=>target.hidden=true,5000);
}
function extensionMarkup() {
  if(browserConnection.connected)return '';
  const title:Key=browserConnection.seen?'browserDisconnected':'connectBrowser';
  const hint:Key=browserConnection.seen?'browserReconnect':'browserIntro';
  return `${icon('link')}<div><strong>${e(t(title))}</strong><small>${e(t(hint))}</small><div class="extension-actions">${button('extension-guide',browserConnection.seen?'setupGuide':'getExtension',undefined,!native?'disabled':'')}${button('check-browser',checkingBrowser?'checkingBrowser':'checkBrowser',undefined,!native||checkingBrowser?'disabled':'')}</div></div>`;
}
async function checkBrowser(manual=false) {
  if(!native||checkingBrowser)return;
  const previous={...browserConnection};
  checkingBrowser=true;
  const setButton=()=>{const node=document.querySelector<HTMLButtonElement>('[data-action="check-browser"]');if(node){node.disabled=checkingBrowser;node.textContent=t(checkingBrowser?'checkingBrowser':'checkBrowser');}};
  if(manual)setButton();
  try {
    browserConnection=await invoke<typeof browserConnection>('browser_connection',{repair:manual});
    if(manual)toast(t(browserConnection.connected?'browserReady':'browserCheckHint'));
  } catch(error) { browserConnection.connected=false;if(manual)toast(String(error)); }
  finally {
    checkingBrowser=false;
    if(previous.seen!==browserConnection.seen||previous.connected!==browserConnection.connected){
      const card=document.querySelector<HTMLElement>('#browser-connection');
      const focusedAction=(document.activeElement as HTMLElement)?.dataset.action;
      if(card){card.hidden=browserConnection.connected;card.innerHTML=extensionMarkup();}
      if(!browserConnection.connected&&(focusedAction==='check-browser'||focusedAction==='extension-guide'))document.querySelector<HTMLButtonElement>(`[data-action="${focusedAction}"]`)?.focus();
    }
    setButton();
  }
}
async function execute(action:unknown) {
  if(!native){toast(t('webPreview'));return false;}
  try {state=await invoke<Snapshot>('command',{action});refresh();return true;}catch(error){toast(String(error));return false;}
}
function refresh() {
  updateList();updateDetails();
  const label=document.querySelector('[data-queue-label]');if(label)label.textContent=t(queueLabel());
  document.querySelector('.state-dot')?.classList.toggle('running',state.running);
  const speed=document.querySelector('[data-total-speed]');if(speed)speed.textContent=bytes(totalSpeed())+'/s';
  const summaryNode=document.querySelector('[data-summary]');if(summaryNode)summaryNode.innerHTML=summary();
  const queueButton=document.querySelector<HTMLButtonElement>('[data-action="toggle-queue"]');if(queueButton)queueButton.innerHTML=icon(state.running?'pause':'play')+`<span>${e(t(state.running?'pauseQueue':'start'))}</span>`;
}

type Preview = { name:string; total:number|null; resumable:boolean|null; directory:string };
function validUrls(urls:string[]) {
  return urls.length>=1&&urls.length<=100&&urls.every(input=>{try{const url=new URL(input);return ['http:','https:'].includes(url.protocol)&&!url.username&&!url.password;}catch{return false;}});
}
function showAdd(multiple=false, initial:string[] = []) {
  const dialog=document.querySelector<HTMLDialogElement>('#add-dialog')!;
  dialog.innerHTML=`<form id="add-form"><div class="dialog-heading"><div><h2 id="add-title">${e(t(multiple?'multiple':'newDownload'))}</h2><p>${e(t('directHint'))}</p></div><button type="button" data-action="close-add" class="icon-button" aria-label="${e(t('close'))}">${icon('close')}</button></div><label class="field"><span>${e(t('url'))}</span><textarea name="urls" rows="${multiple?6:3}" placeholder="https://example.com/file.zip" dir="ltr" required autofocus></textarea></label><label class="field"><span>${e(t('folder'))}</span><div class="joined"><input name="directory" value="${e(state.settings.directory)}" dir="ltr" ${native?'required':''}>${button('browse-add','browse','folder',!native?'disabled':'')}</div></label>${field('name','optionalName','text','')}${field('sha256','hash','text','','pattern="[a-fA-F0-9]{64}" dir="ltr"')}<p id="add-error" class="form-error" role="alert"></p><div class="dialog-footer">${button('close-add','cancel')}<button type="submit" name="mode" value="queue">${e(t('addQueue'))}</button><button type="submit" name="mode" value="now" class="primary">${e(t('downloadNow'))}</button></div></form>`;
  dialog.showModal();
  const form=dialog.querySelector<HTMLFormElement>('form')!;
  const source=form.querySelector<HTMLTextAreaElement>('[name="urls"]')!;
  const destination=form.querySelector<HTMLInputElement>('[name="directory"]')!;
  const meta=document.createElement('section');
  meta.className='add-preview';meta.hidden=true;meta.setAttribute('aria-live','polite');
  source.closest('label')!.after(meta);
  const preference=document.createElement('label');
  preference.className='delete-checkbox';
  preference.innerHTML=`<input type="checkbox" name="skip" ${state.settings.confirmless_add?'checked':''}>${e(t('confirmLess'))}`;
  form.querySelector('.dialog-footer')!.before(preference);
  if(initial.length)source.value=initial.join('\n');
  if(multiple){form.querySelector('[name="name"]')?.closest('label')?.remove();form.querySelector('[name="sha256"]')?.closest('label')?.remove();}
  let manualFolder=false;
  destination.addEventListener('input',()=>{manualFolder=true;});
  let timer:ReturnType<typeof setTimeout>;
  let last='';
  let inspection:Promise<void>|undefined;
  function inspect() {
    const value=source.value;
    if(last===value&&inspection)return inspection;
    last=value;
    const urls:string[]=parseLinks(value);
    meta.hidden=!native||!validUrls(urls);
    if(meta.hidden){inspection=undefined;return;}
    meta.textContent=t('checkingFile');
    inspection=Promise.all(urls.slice(0,6).map(url=>invoke<Preview>('preview_download',{url}).catch(()=>null))).then(results=>{
      if(last!==value||!dialog.open)return;
      if(!manualFolder)destination.value=urls.length===1&&results[0]?.directory?results[0].directory:state.settings.directory;
      meta.innerHTML=`<h3>${e(t('previewFile'))}</h3>${results.map((file,i)=>{
        if(!file)return `<small>${e(t('sourcePreviewFailed'))} <bdi>#${i+1}</bdi></small>`;
        const index=file.name.lastIndexOf('.');
        return `<article><strong dir="auto">${e(file.name)}</strong><small>${e(t('exactSize'))}: <bdi>${file.total===null?e(t('unknown')):`${bytes(file.total)} · ${new Intl.NumberFormat(document.documentElement.lang).format(file.total)} B`}</bdi> · ${e(t('extensionType'))}: <bdi>${index>0?e(file.name.slice(index)):e(t('unknown'))}</bdi> · ${e(t('resumable'))}: ${e(t(file.resumable===null?'unknown':file.resumable?'yes':'no'))}</small></article>`;
      }).join('')}${urls.length>6?`<small>+${urls.length-6}</small>`:''}`;
    });
    return inspection;
  }
  source.addEventListener('input',()=>{
    clearTimeout(timer);
    last='';inspection=undefined;
    if(!validUrls(parseLinks(source.value)))meta.hidden=true;
    timer=setTimeout(()=>void inspect(),360);
  });
  source.addEventListener('paste',event=>{
    if(!native||!state.settings.confirmless_add||!preference.querySelector('input')?.checked||source.value.trim())return;
    const input=event.clipboardData?.getData('text/plain').trim()??'';
    const urls=parseLinks(input);
    if(validUrls(urls)&&(multiple||urls.length===1)){
      event.preventDefault();
      source.value=input;
      form.requestSubmit(form.querySelector<HTMLButtonElement>('button[value="now"]')!);
    }
  });
  source.addEventListener('keydown',event=>{
    if(event.key==='Enter'&&!multiple&&!event.shiftKey&&!event.isComposing){
      event.preventDefault();
      form.requestSubmit(form.querySelector<HTMLButtonElement>('button[value="now"]')!);
    }
  });
  dialog.addEventListener('close',()=>clearTimeout(timer),{once:true});
  form.addEventListener('submit',async event=>{
    event.preventDefault();if(pending)return;
    if(!validUrls(parseLinks(source.value))){dialog.querySelector('#add-error')!.textContent=t('invalidLinks');return;}
    if(!native){dialog.querySelector('#add-error')!.textContent=t('webPreview');return;}
    const start=(event as SubmitEvent).submitter?.getAttribute('value')!=='queue';
    pending=true;form.querySelectorAll<HTMLButtonElement>('button[type="submit"]').forEach(b=>b.disabled=true);
    source.readOnly=true;
    await inspect();
    const data=new FormData(form);
    const urls=parseLinks(source.value);
    const success=dialog.open&&await execute({type:'add',urls,directory:String(data.get('directory'))||null,name:data.get('name')||null,sha256:data.get('sha256')||null,start});
    if(success&&data.has('skip')!==!!state.settings.confirmless_add)await execute({type:'confirm_preference',skip:data.has('skip')});
    source.readOnly=false;
    pending=false;form.querySelectorAll<HTMLButtonElement>('button[type="submit"]').forEach(b=>b.disabled=false);
    if(success)dialog.close();
  });
  if(initial.length)void inspect();
}

function confirmAction(message:Key, callback:(deleteFiles:boolean)=>Promise<void>, allowDelete=false) {
  const dialog=document.querySelector<HTMLDialogElement>('#confirm-dialog')!;
  dialog.innerHTML=`<form method="dialog"><div class="dialog-heading"><h2 id="confirm-title">${e(t('confirm'))}</h2></div><p>${e(t(message))}</p>${allowDelete?`<label class="delete-checkbox"><input type="checkbox" id="delete-files">${e(t('deleteFiles'))}</label>`:''}<div class="dialog-footer"><button value="cancel">${e(t('cancel'))}</button><button value="confirm" class="primary">${e(t('confirm'))}</button></div></form>`;
  dialog.showModal();dialog.addEventListener('close',()=>{if(dialog.returnValue==='confirm')void callback(dialog.querySelector<HTMLInputElement>('#delete-files')?.checked??false);},{once:true});
}
async function batch(type:string,ids=[...selected],deleteFiles=false) {
  for(const id of ids){if(!await execute({type,id,...(type==='pause'?{continue_queue:false}:{}),...(type==='remove'?{delete_files:deleteFiles}:{})}))break;}
}

function rowMenu(id:string,x:number,y:number) {
  detailId=id;updateDetails();
  const d=state.downloads.find(d=>d.id===id);if(!d)return;
  const menu=document.querySelector<HTMLElement>('#context-menu')!;
  const item=(action:string,label:Key,glyph:string,disabled=false)=>`<button role="menuitem" data-menu-action="${action}" data-menu-id="${id}" ${disabled?'disabled':''}>${icon(glyph)}${e(t(label))}</button>`;
  menu.innerHTML=item('resume','resume','play',d.status==='completed')+item('pause','pause','pause',!active(d))+item('pause_continue','pauseContinue','pause',!active(d))+item('stop','stop','stop',d.status==='completed')+'<hr>'+item('top','top','up',active(d)||d.status==='completed')+item('up','up','up')+item('down','down','arrowDown')+item('skip','skip','clock',d.status==='completed')+'<hr>'+item('open','open','file',!d.path)+item('reveal','reveal','folder',!d.path)+item('copy','copy','copy',!d.path)+'<hr>'+item('restart','restart','retry')+item('remove','remove','trash');
  menu.hidden=false;
  menu.style.left=`${Math.max(8,Math.min(x,window.innerWidth-menu.offsetWidth-8))}px`;
  menu.style.top=`${Math.max(8,Math.min(y,window.innerHeight-menu.offsetHeight-8))}px`;
  menu.querySelector<HTMLButtonElement>('button:not(:disabled)')?.focus();
}
function queueMenu(x:number,y:number) {
  const menu=document.querySelector<HTMLElement>('#context-menu')!;
  menu.innerHTML=button('start-queue','start','play')+button('pause-queue','pauseQueue','pause')+button('stop-queue','stopQueue','stop')+button('drain','drain','clock')+'<hr>'+button('import','import','link')+button('export','export','list');
  menu.hidden=false;menu.style.left=`${Math.max(8,Math.min(x,window.innerWidth-280))}px`;menu.style.top=`${Math.min(y,window.innerHeight-menu.offsetHeight-8)}px`;
}
async function itemAction(action:string,id:string) {
  if(['resume','stop','skip'].includes(action)){await execute({type:action,id});return;}
  if(action==='pause'||action==='pause_continue'){await execute({type:'pause',id,continue_queue:action==='pause_continue'});return;}
  if(['top','up','down'].includes(action)){await execute({type:'move',id,position:action});return;}
  if(action==='restart'){confirmAction('restartConfirm',async()=>{await batch('restart',[id]);});return;}
  if(action==='remove'){confirmAction('removeConfirm',async deleteFiles=>{await batch('remove',[id],deleteFiles);},true);return;}
  if(action==='copy'){const path=state.downloads.find(d=>d.id===id)?.path;if(path){await navigator.clipboard.writeText(path);toast(t('copied'));}return;}
  if(action==='open'||action==='reveal'){try{await invoke('open_download',{id,folder:action==='reveal'});}catch(error){toast(String(error));}}
}

async function saveSettings() {
  const form=document.querySelector<HTMLFormElement>('#settings-form')!;if(!form.reportValidity())return;
  const data=new FormData(form);const s={...state.settings};
  for(const key of ['language','theme','directory'] as const)s[key]=String(data.get(key));
  for(const key of ['concurrent_downloads','connections_per_file','global_connections','host_connections'] as const)s[key]=Number(data.get(key));
  s.speed_limit=Number(data.get('speed_limit'))*1024;
  for(const key of ['compact','resume_on_launch','stop_on_error','schedule_enabled','close_to_tray','confirmless_add'] as const)s[key]=data.has(key);
  for(const key of ['schedule_start','schedule_end'] as const){const [h,m]=String(data.get(key)).split(':').map(Number);s[key]=h*60+m;}
  if(native){if(!await execute({type:'settings',settings:s}))return;}else{state.settings=s;}
  shell();toast(t('saved'));
}

document.addEventListener('click',async event=>{
  const target=event.target as HTMLElement;
  const menu=document.querySelector<HTMLElement>('#context-menu');
  const nav=target.closest<HTMLElement>('[data-filter]');
  if(nav){filter=nav.dataset.filter!;settingsView=false;root.classList.remove('nav-open');shell();return;}
  const checkbox=target.closest<HTMLInputElement>('[data-select]');
  if(checkbox){if(checkbox.checked)selected.add(checkbox.dataset.select!);else selected.delete(checkbox.dataset.select!);updateList();return;}
  const rowMenuButton=target.closest<HTMLElement>('[data-row-menu]');
  if(rowMenuButton){const bounds=rowMenuButton.getBoundingClientRect();rowMenu(rowMenuButton.dataset.rowMenu!,bounds.left,bounds.bottom+4);return;}
  const menuItem=target.closest<HTMLElement>('[data-menu-action]');
  if(menuItem){menu!.hidden=true;try{await itemAction(menuItem.dataset.menuAction!,menuItem.dataset.menuId!);}catch(error){toast(String(error));}return;}
  if(menu&&!target.closest('#context-menu'))menu.hidden=true;
  const action=target.closest<HTMLElement>('[data-action]')?.dataset.action;
  if(!action){const row=target.closest<HTMLElement>('tr[data-id]');if(row){detailId=row.dataset.id!;detailOpen=true;updateList();updateDetails();}return;}
  if(menu)menu.hidden=true;
  try {
    if(action.startsWith('feature-'))await features.handle(action,target.closest<HTMLElement>('[data-action]')!);
    else if(action==='add'||action==='multiple')showAdd(action==='multiple');
    else if(action==='extension-guide')await invoke('extension_guide');
    else if(action==='check-browser')await checkBrowser(true);
    else if(action==='close-add')document.querySelector<HTMLDialogElement>('#add-dialog')!.close();
    else if(action==='settings'){settingsView=true;root.classList.remove('nav-open');shell();}
    else if(action==='nav')root.classList.toggle('nav-open');
    else if(action==='details'){detailOpen=!detailOpen;updateDetails();}
    else if(action==='clear-selection'){selected.clear();updateList();}
    else if(action==='toggle-queue')await execute({type:state.running?'pause_queue':'start_queue'});
    else if(['start-queue','pause-queue','stop-queue','drain'].includes(action))await execute({type:action.replaceAll('-','_')});
    else if(action==='queue-menu'){const bounds=target.closest('button')!.getBoundingClientRect();queueMenu(bounds.left,bounds.bottom+4);}
    else if(action==='bulk-remove')confirmAction('removeConfirm',async remove=>{await batch('remove',undefined,remove);},true);
    else if(action.startsWith('bulk-'))await batch(action.slice(5));
    else if(action.startsWith('item-')&&detailId)await itemAction(action.slice(5),detailId);
    else if(action==='save-settings')await saveSettings();
    else if(action==='browse-add'||action==='browse-settings'){
      const directory=await open({directory:true,multiple:false});if(typeof directory==='string'){const input=document.querySelector<HTMLInputElement>(action==='browse-add'?'#add-form [name="directory"]':'#settings-form [name="directory"]');if(input){input.value=directory;input.dispatchEvent(new Event('input'));}}
    }
    else if(action==='import'){
      if(!native){toast(t('webPreview'));return;}
      const urls=await invoke<string[]>('import_links');
      if(state.settings.confirmless_add&&validUrls(urls))await execute({type:'add',urls,directory:null,name:null,sha256:null,start:true});
      else showAdd(urls.length>1,urls);
    }
    else if(action==='export')confirmAction('exportConfirm',async()=>{await invoke('export_links');});
  }catch(error){toast(String(error));}
});
document.addEventListener('contextmenu',event=>{const row=(event.target as HTMLElement).closest<HTMLElement>('tr[data-id]');if(row){event.preventDefault();rowMenu(row.dataset.id!,event.clientX,event.clientY);}});
let dragged:string|null=null;
document.addEventListener('dragstart',event=>{dragged=(event.target as HTMLElement).closest<HTMLElement>('tr[data-id]')?.dataset.id??null;if(dragged)event.dataTransfer?.setData('text/plain',dragged);});
document.addEventListener('dragover',event=>{if(dragged&&(event.target as HTMLElement).closest('tr[data-id]'))event.preventDefault();});
document.addEventListener('drop',event=>{
  const id=(event.target as HTMLElement).closest<HTMLElement>('tr[data-id]')?.dataset.id;
  if(!id||!dragged||id===dragged)return;event.preventDefault();
  const ids=state.downloads.map(d=>d.id);const from=ids.indexOf(dragged);const to=ids.indexOf(id);if(from<0||to<0)return;
  ids.splice(from,1);ids.splice(to,0,dragged);dragged=null;void execute({type:'reorder',ids});
});
document.addEventListener('keydown',event=>{
  if(event.key==='Escape'){root.classList.remove('nav-open');const menu=document.querySelector<HTMLElement>('#context-menu');if(menu)menu.hidden=true;}
  if(event.ctrlKey&&event.key.toLowerCase()==='n'){event.preventDefault();if(!document.querySelector('dialog[open]'))showAdd();}
  if(event.ctrlKey&&event.key.toLowerCase()==='k'){event.preventDefault();document.querySelector<HTMLInputElement>('#search')?.focus();}
  const row=(event.target as HTMLElement).closest<HTMLElement>('tr[data-id]');
  if(row&&event.key==='Enter'&&event.target===row){detailId=row.dataset.id!;detailOpen=true;updateDetails();}
  const menu=document.querySelector<HTMLElement>('#context-menu');
  if(menu&&!menu.hidden&&['ArrowDown','ArrowUp'].includes(event.key)){
    event.preventDefault();const items=[...menu.querySelectorAll<HTMLButtonElement>('button:not(:disabled)')];const i=items.indexOf(document.activeElement as HTMLButtonElement);items[(i+(event.key==='ArrowDown'?1:items.length-1))%items.length]?.focus();
  }
});

shell();
if(native){
  try {
    await syncWindowState();
    await appWindow!.onResized(()=>{ void syncWindowState().catch(error=>toast(String(error))); });
    await appWindow!.onFocusChanged(({payload})=>{
      focused = payload;
      if(focused)void checkBrowser();
      document.documentElement.dataset.focused = String(focused);
    });
    await listen<Snapshot>('ndm://snapshot',event=>{
      state=event.payload;
      const now=Date.now();
      for(const d of state.downloads){const values=history.get(d.id)??[];if(!values.length||now-values[values.length-1].time>=900){values.push({time:now,speed:active(d)?d.speed:0});history.set(d.id,values.filter(p=>p.time>=now-60000));}}
      refresh();
    });
    await listen<string>('ndm://error',event=>toast(event.payload));
    state=await invoke<Snapshot>('snapshot');shell();
    await features.start();
    await checkBrowser();
    setInterval(()=>{void checkBrowser();},2000);
  }catch(error){toast(`${t('backendError')} ${String(error)}`);}
}
