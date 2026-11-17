// Browser smoke checks run only on GitHub; the Tauri IPC fixture exercises UI state.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir } from 'node:fs/promises';
import { chromium } from 'playwright';
const server=spawn(process.execPath,['node_modules/vite/bin/vite.js','preview','--config','apps/desktop/vite.config.ts','--host','127.0.0.1','--port','4173'],{stdio:'inherit'});
let browser;
try {
  let ready=false;
  for(let attempt=0;attempt<60;attempt++) {
    try { if((await fetch('http://127.0.0.1:4173')).ok){ready=true;break;} } catch {}
    await new Promise(resolve=>setTimeout(resolve,500));
  }
  assert.ok(ready,'Built UI preview server must start');
  browser=await chromium.launch({channel:'msedge',headless:true});
  const page=await browser.newPage({viewport:{width:1220,height:820},deviceScaleFactor:1});
  const errors=[];
  page.on('pageerror',error=>errors.push(error.message));
  await page.goto('http://127.0.0.1:4173');
  await page.getByRole('heading',{name:'All downloads',exact:true}).waitFor();
  await page.evaluate(()=>document.fonts.ready);
  await mkdir('.artifacts',{recursive:true});
  const check=async()=>{
    const measurements=await page.evaluate(()=>({
      images:[...document.querySelectorAll('img')].every(img=>img.complete&&img.naturalWidth>0),
      controls:document.querySelectorAll('.window-controls button').length,
      overflow:document.documentElement.scrollWidth>innerWidth,
      radius:getComputedStyle(document.querySelector('#app')).borderTopLeftRadius,
      bar:getComputedStyle(document.querySelector('.titlebar')).direction,
    }));
    assert.equal(measurements.images,true,'Every crop must load');
    assert.equal(measurements.controls,3);
    assert.equal(measurements.overflow,false);
    assert.equal(measurements.radius,'16px');
    assert.equal(measurements.bar,'ltr','Window controls keep Windows ordering in RTL');
  };
  await check();
  await page.getByText('Connect your browser',{exact:true}).waitFor();
  assert.equal(await page.getByRole('button',{name:'Get the extension',exact:true}).isDisabled(),true,'Web preview must not invent a desktop connection');
  await page.screenshot({path:'.artifacts/preview-en-dark.png'});
  await page.locator('[data-action="settings"]').click();
  await page.locator('[name="language"]').selectOption('fa');
  await page.locator('[name="theme"]').selectOption('light');
  await page.locator('[name="directory"]').fill('C:\\NDM-UI-test');
  await page.locator('[data-action="save-settings"]').click();
  assert.equal(await page.locator('html').getAttribute('dir'),'rtl');
  await page.locator('[data-filter="all"]').click();
  await page.getByText('مرورگر را متصل کنید',{exact:true}).waitFor();
  await check();
  await page.screenshot({path:'.artifacts/preview-fa-light.png'});
  await page.getByRole('button',{name:'افزودن نخستین لینک',exact:true}).click();
  await page.locator('dialog[open]').waitFor();
  assert.equal(await page.locator('textarea[name="urls"]').getAttribute('dir'),'ltr');
  await page.getByRole('dialog').getByRole('button',{name:'بستن',exact:true}).click();
  for(const width of [1440,768,390]) {
    await page.setViewportSize({width,height:900});
    await check();
  }
  const desktop=await browser.newPage();
  desktop.on('pageerror',error=>errors.push(error.message));
  await desktop.addInitScript(()=>{
    window.isTauri=true;
    window.ndmTestConnection={seen:false,connected:false};
    window.__TAURI_INTERNALS__={
      metadata:{currentWindow:{label:'main'}},
      transformCallback:()=>1,
      invoke:async(command)=>{
        if(command==='browser_connection')return {...window.ndmTestConnection};
        if(command==='snapshot')return {downloads:[],running:false,draining:false,settings:{directory:'C:\\Downloads',language:'en',theme:'dark',compact:false,concurrent_downloads:1,connections_per_file:1,global_connections:16,host_connections:8,speed_limit:0,resume_on_launch:false,stop_on_error:false,schedule_enabled:false,schedule_start:0,schedule_end:0,close_to_tray:false}};
        if(command==='plugin:event|listen')return 1;
        if(command==='plugin:window|is_maximized')return false;
        throw new Error(`Unexpected desktop fixture command: ${command}`);
      },
    };
  });
  await desktop.goto('http://127.0.0.1:4173');
  await desktop.locator('[data-action="check-browser"]:enabled').waitFor();
  await desktop.evaluate(()=>{window.ndmTestConnection={seen:true,connected:true};});
  await desktop.locator('[data-action="check-browser"]').click();
  await desktop.locator('#browser-connection').waitFor({state:'hidden'});
  assert.equal(await desktop.locator('#browser-connection').innerText(),'','Connected card content is removed');
  await desktop.locator('[data-action="settings"]').click();
  await desktop.locator('#browser-connection').waitFor({state:'hidden'});
  await desktop.evaluate(()=>{window.ndmTestConnection={seen:true,connected:false};});
  await desktop.locator('#browser-connection').waitFor({state:'visible'});
  await desktop.locator('[data-action="check-browser"]:enabled').waitFor();
  await desktop.evaluate(()=>{window.ndmTestConnection={seen:true,connected:true};});
  await desktop.locator('#browser-connection').waitFor({state:'hidden'});
  await desktop.close();
  assert.deepEqual(errors,[],'UI must not raise runtime errors');
  console.log('UI smoke checks passed: assets, rounded frame, English/Persian, theme, dialog, 3 viewport widths, connection card hides on connect and returns on disconnect.');
} finally {
  await browser?.close();
  server.kill();
}
