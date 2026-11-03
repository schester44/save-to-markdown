'use strict';
const HOST = 'local.save_to_markdown';
const MENU = 'save-to-markdown';
browser.menus.create({ id: MENU, title: 'Save for later', contexts: ['link', 'page', 'tab'] });
// Serialize clicks within this background process; the host also locks across processes.
let pending = Promise.resolve();
browser.menus.onClicked.addListener((info, tab) => {
  if (info.menuItemId !== MENU) return;
  pending = pending.then(async () => {
    try {
      const url = info.linkUrl || info.pageUrl || (tab && tab.url);
      const title = info.linkUrl ? (info.linkText || url) : ((tab && tab.title) || url);
      const response = await browser.runtime.sendNativeMessage(HOST, { action: 'save', title, url });
      if (!response || response.ok !== true) throw new Error((response && response.error) || 'No response from native host');
    } catch (error) {
      console.error('Save to Markdown:', error);
      await browser.notifications.create({ type: 'basic', iconUrl: browser.runtime.getURL('icon.svg'),
        title: 'Could not save for later', message: String(error.message || error) });
    }
  }).catch(console.error);
});
