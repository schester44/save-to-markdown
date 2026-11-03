'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
(async () => {
  let listener;
  const messages = [], notices = [];
  const browser = {
    menus: { create: options => { assert.equal(options.title, 'Save for later'); assert.deepEqual(Array.from(options.contexts), ['link', 'page', 'tab']); },
      onClicked: { addListener: fn => { listener = fn; } } },
    runtime: { getURL: p => p, sendNativeMessage: async (host, data) => {
      assert.equal(host, 'local.save_to_markdown');
      messages.push(JSON.parse(JSON.stringify(data)));
      return data.title === 'Failure' ? { ok: false, error: 'test failure' } : { ok: true };
    } },
    notifications: { create: async n => { notices.push(n); } }
  };
  vm.runInNewContext(fs.readFileSync(path.join(__dirname, '../extension/background.js'), 'utf8'), { browser, console: { error() {} } });
  listener({ menuItemId: 'other' }, {});
  listener({ menuItemId: 'save-to-markdown', linkUrl: 'https://link.test', linkText: 'Link title' }, { title: 'Parent title' });
  listener({ menuItemId: 'save-to-markdown', pageUrl: 'https://page.test' }, { title: 'Page title' });
  listener({ menuItemId: 'save-to-markdown', linkUrl: 'https://image.test' }, {});
  listener({ menuItemId: 'save-to-markdown', pageUrl: 'https://error.test' }, { title: 'Failure' });
  listener({ menuItemId: 'save-to-markdown', pageUrl: 'https://later.test' }, { title: 'Later' });
  listener({ menuItemId: 'save-to-markdown' }, { title: 'Background tab', url: 'https://tab.test', active: false });
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(messages.length, 6);
  assert.deepEqual(messages[0], { action: 'save', title: 'Link title', url: 'https://link.test' });
  assert.equal(messages[1].title, 'Page title');
  assert.equal(messages[2].title, 'https://image.test');
  assert.equal(notices.length, 1);
  assert.equal(messages[4].title, 'Later');
  assert.deepEqual(messages[5], { action: 'save', title: 'Background tab', url: 'https://tab.test' });
  console.log('PASS: link/page capture, fallback, failure notification, queue recovery');
})().catch(e => { console.error(e); process.exitCode = 1; });
