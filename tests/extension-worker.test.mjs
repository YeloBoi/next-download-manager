import test from 'node:test';
import assert from 'node:assert/strict';

for (const namespace of ['chrome','browser']) test(`${namespace} background serializes selections, preserves failed sends and survives suspension`,async()=>{
  let stored={};let fail=true;const sent=[];let events={};
  const boot=async(index)=>{
    events={};
    const event=name=>({addListener:listener=>{events[name]=listener;}});
    globalThis[namespace]={
      runtime:{id:'ndm-test',getURL:path=>'chrome-extension://ndm-test/'+path,
        onInstalled:event('installed'),onStartup:event('startup'),onMessage:event('message'),
        sendNativeMessage:async(_host,message)=>{if(fail)throw new Error('offline');sent.push(message);return {version:1,request_id:message.request_id,accepted:true};}},
      storage:{local:{get:async key=>typeof key==='string'?{[key]:structuredClone(stored[key])}:structuredClone(stored),set:async value=>{stored={...stored,...structuredClone(value)};}},onChanged:event('storage')},
      contextMenus:{removeAll:async()=>{},create:()=>{},onClicked:event('context')},
      action:{setBadgeText:async()=>{},setBadgeBackgroundColor:async()=>{}},
      alarms:{create:async()=>{},onAlarm:event('alarm')},
    };
    await import(`../extensions/chromium/background.js?test=${namespace}-${index}`);
  };
  const call=(operation,values={})=>new Promise(resolve=>events.message({type:'selection',operation,...values},{id:'ndm-test',url:'chrome-extension://ndm-test/popup.html'},resolve));
  await boot(1);
  await Promise.all([call('add',{urls:['https://a.test/']}),call('add',{urls:['https://b.test/']})]);
  assert.deepEqual((await call('get')).urls,['https://a.test/','https://b.test/']);
  assert.equal((await call('send',{start:true})).ok,false);
  assert.deepEqual(stored.selection,['https://a.test/','https://b.test/']);
  await boot(2); // New service-worker module, same persistent storage.
  assert.deepEqual((await call('get')).urls,['https://a.test/','https://b.test/']);
  fail=false;
  assert.equal((await call('send',{start:false})).ok,true);
  assert.deepEqual(sent[0].urls,['https://a.test/','https://b.test/']);
  assert.equal(sent[0].start,false);
  assert.deepEqual(stored.selection,[]);
  events.context({menuItemId:'ndm-select',linkUrl:'https://c.test/'});
  assert.deepEqual((await call('get')).urls,['https://c.test/']);
  assert.equal(sent.length,1,'selection alone must not start a download');
  events.context({menuItemId:'ndm-link',linkUrl:'https://d.test/'});
  await call('get');
  assert.deepEqual(sent[1].urls,['https://d.test/']);
  assert.equal(sent[1].start,true);
  assert.deepEqual(stored.selection,['https://c.test/'],'only acknowledged URLs are removed');
  let replied=false;
  events.message({type:'selection',operation:'clear'},{id:'ndm-test',url:'https://untrusted.test/'},()=>{replied=true;});
  assert.equal(replied,false);
  delete globalThis[namespace];
});
