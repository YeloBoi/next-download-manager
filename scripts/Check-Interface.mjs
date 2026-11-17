// Browser smoke checks run only on GitHub; native Windows interactions remain manual.
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
  await page.screenshot({path:'.artifacts/preview-en-dark.png'});
  await page.locator('[data-action="settings"]').click();
  await page.locator('[name="language"]').selectOption('fa');
  await page.locator('[name="theme"]').selectOption('light');
  await page.locator('[name="directory"]').fill('C:\\NDM-UI-test');
  await page.locator('[data-action="save-settings"]').click();
  assert.equal(await page.locator('html').getAttribute('dir'),'rtl');
  await page.locator('[data-filter="all"]').click();
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
  assert.deepEqual(errors,[],'UI must not raise runtime errors');
  console.log('UI smoke checks passed: real assets, rounded frame, window control layout, English/Persian, theme, dialog and 3 viewport widths.');
} finally {
  await browser?.close();
  server.kill();
}
