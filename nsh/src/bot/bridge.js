(function () {
  'use strict';
  const context = __MIXIN_CONTEXT__;
  const pending = Object.create(null);
  let sequence = 0;
  const documentId = Math.random().toString(36).slice(2) + Date.now().toString(36);
  function post(name, data) {
    __MIXIN_POST__(JSON.stringify({ name, data }));
  }
  function reply(callback, args) {
    if (typeof callback !== 'string' || !/^[A-Za-z_$][\w$]*(\.[A-Za-z_$][\w$]*)*$/.test(callback)) return;
    setTimeout(function () {
      let target = window, owner = window;
      for (const key of callback.split('.')) { owner = target; target = target && target[key]; }
      if (typeof target === 'function') target.apply(owner, args);
    }, 0);
  }
  Object.defineProperty(window, '__mixinBotReply', { value: function (id, args) {
    const callback = pending[id];
    if (callback === undefined) return;
    delete pending[id];
    reply(callback, args);
  }});
  Object.defineProperty(window, '__mixinBotSetLocale', { value: function (locale) {
    if (typeof locale === 'string') context.locale = locale;
  }});
  window.MixinContext = {
    getContext: function () {
      context.appearance = matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
      return JSON.stringify(context);
    },
    reloadTheme: function () { publishTheme(); },
    close: function () { post('close'); },
    openInBrowser: function (url) { post('openInBrowser', String(url)); return true; },
    showToast: function () {},
    playlist: function () {},
    getAssets: function (_, callback) { reply(callback, ['[]']); },
    getTipAddress: function (_, callback) { reply(callback, ['']); },
    tipSign: function (_, __, callback) { reply(callback, ['']); },
    verifyPin: function (callback) { reply(callback, [false]); },
    signBotSignature: function (body) {
      if (!Array.isArray(body) || body.length < 6) return;
      if (Object.keys(pending).length >= 32) { reply(body[5], ['[]']); return; }
      const id = documentId + '-' + (++sequence);
      pending[id] = body[5];
      post('signBotSignature', { id, body: body.slice(0, 5) });
    }
  };
  let lastTheme;
  let sawTheme = false;
  function publishTheme() {
    let color = null;
    const metas = document.querySelectorAll('meta[name="theme-color"][content]');
    if (!metas.length && !sawTheme && document.readyState === 'loading') return;
    sawTheme = sawTheme || metas.length > 0;
    for (const meta of metas) {
      const media = meta.getAttribute('media');
      if (media && !matchMedia(media).matches) continue;
      const canvas = document.createElement('canvas');
      canvas.width = canvas.height = 1;
      const ctx = canvas.getContext('2d');
      if (!ctx) break;
      const probe = document.createElement('div');
      probe.style.color = meta.getAttribute('content');
      if (!probe.style.color) break;
      document.documentElement.appendChild(probe);
      ctx.fillStyle = getComputedStyle(probe).color;
      probe.remove();
      ctx.fillRect(0, 0, 1, 1);
      const rgba = ctx.getImageData(0, 0, 1, 1).data;
      if (rgba[3]) color = [rgba[0], rgba[1], rgba[2]];
      break;
    }
    const serialized = JSON.stringify(color);
    if (serialized !== lastTheme) { lastTheme = serialized; post('theme', { color }); }
  }
  const colors = new MutationObserver(publishTheme);
  function watchHead() {
    if (!document.head) return false;
    colors.observe(document.head, { childList: true, subtree: true, attributes: true, attributeFilter: ['name', 'content', 'media'] });
    publishTheme();
    return true;
  }
  if (!watchHead()) {
    const parser = new MutationObserver(function () { if (watchHead()) parser.disconnect(); });
    parser.observe(document, { childList: true, subtree: true });
  }
  document.addEventListener('DOMContentLoaded', publishTheme, { once: true });
  matchMedia('(prefers-color-scheme: dark)').addEventListener('change', publishTheme);
})();
