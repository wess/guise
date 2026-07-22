// The HTML document shell shared by every page: <head> metadata, the top
// header bar, and the footer. Editorial "specimen sheet" chrome on a light
// paper canvas. Pages pass their own <body> contents in.

const SITE = "https://wess.github.io/guise";
const REPO = "https://github.com/wess/guise";

export type ShellOpts = {
  title: string;
  description: string;
  body: string;
  active?: "docs" | "tutorial" | "gallery" | "home";
  tail?: string;
};

function header(active: string): string {
  const link = (href: string, label: string, key: string) =>
    `<a href="${href}"${active === key ? ' class="active"' : ""}>${label}</a>`;
  return `<header class="topbar">
  <div class="topbar-inner">
    <a class="brand" href="index.html" aria-label="guise home">
      <span class="brand-mark"></span><span class="brand-name">guise</span>
    </a>
    <button class="search-trigger" data-search-open aria-label="Search">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><circle cx="11" cy="11" r="7"/><path d="m21 21-4.3-4.3"/></svg>
      <span>Search</span><kbd>&#8984;K</kbd>
    </button>
    <nav class="topnav">
      ${link("docs.html", "Docs", "docs")}
      ${link("tutorial.html", "Tutorial", "tutorial")}
      ${link("gallery.html", "Gallery", "gallery")}
      <a href="${REPO}" class="ext" rel="noreferrer">GitHub &#8599;</a>
    </nav>
    <button class="navtoggle" aria-label="Toggle navigation">&#9776;</button>
  </div>
</header>`;
}

function searchModal(): string {
  return `<div class="search" id="search" hidden>
  <div class="search-mask" data-search-close></div>
  <div class="search-box" role="dialog" aria-label="Search documentation">
    <div class="search-top">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><circle cx="11" cy="11" r="7"/><path d="m21 21-4.3-4.3"/></svg>
      <input class="search-input" type="text" placeholder="Search the docs…" autocomplete="off" spellcheck="false" />
      <kbd>esc</kbd>
    </div>
    <div class="search-results"></div>
    <div class="search-foot"><span><kbd>&#8593;</kbd><kbd>&#8595;</kbd> navigate</span><span><kbd>&#8629;</kbd> open</span><span><kbd>esc</kbd> close</span></div>
  </div>
</div>`;
}

function footer(): string {
  return `<footer class="sitefooter">
  <div class="container foot-grid">
    <div class="foot-brand">
      <span class="foot-word">guise</span>
      <p>A Mantine-inspired component library for gpui — native UI for Rust.</p>
      <span class="foot-meta">MIT &middot; built on gpui</span>
    </div>
    <div class="foot-col">
      <span class="foot-h">Docs</span>
      <a href="gettingstarted.html">Installation</a>
      <a href="tutorial.html">Tutorial</a>
      <a href="theming.html">Theming</a>
      <a href="components.html">Component model</a>
      <a href="architecture.html">Architecture</a>
    </div>
    <div class="foot-col">
      <span class="foot-h">Components</span>
      <a href="buttons.html">Buttons</a>
      <a href="inputs.html">Inputs</a>
      <a href="overlays.html">Overlays</a>
      <a href="webview.html">WebView</a>
    </div>
    <div class="foot-col">
      <span class="foot-h">Project</span>
      <a href="${REPO}" rel="noreferrer">GitHub</a>
      <a href="https://github.com/sponsors/wess" rel="noreferrer">&hearts; Sponsor</a>
      <a href="https://github.com/zed-industries/zed" rel="noreferrer">gpui</a>
      <a href="https://mantine.dev" rel="noreferrer">Mantine</a>
      <a href="https://crates.io/crates/wry" rel="noreferrer">wry</a>
    </div>
  </div>
  <div class="container foot-base"><span>&copy; 2026 Wess Cope</span><span>guise &mdash; a design specimen</span></div>
</footer>`;
}

// Progressive-enhancement script: copy buttons, nav toggle, and Cmd+K search.
const SCRIPT = `<script>
(function () {
  // copy buttons + mobile nav toggle
  document.addEventListener('click', function (e) {
    var b = e.target.closest('[data-copy]');
    if (b) {
      navigator.clipboard && navigator.clipboard.writeText(b.getAttribute('data-copy'));
      var t = b.textContent; b.textContent = 'Copied'; b.classList.add('ok');
      setTimeout(function () { b.textContent = t; b.classList.remove('ok'); }, 1200);
    }
    if (e.target.closest('.navtoggle')) document.body.classList.toggle('nav-open');
  });

  // ---- search (Cmd/Ctrl+K) ----
  var modal = document.getElementById('search');
  if (!modal) return;
  var input = modal.querySelector('.search-input');
  var out = modal.querySelector('.search-results');
  var index = null, items = [], sel = 0;

  function open() {
    modal.hidden = false; document.body.classList.add('searching');
    input.value = ''; input.focus();
    if (!index) fetch('searchindex.json').then(function (r) { return r.json(); }).then(function (d) { index = d; render(''); });
    else render('');
  }
  function close() { modal.hidden = true; document.body.classList.remove('searching'); }

  function render(q) {
    q = q.trim().toLowerCase();
    var res = [];
    (index || []).forEach(function (page) {
      if (!q || page.title.toLowerCase().indexOf(q) > -1 || (page.text || '').toLowerCase().indexOf(q) > -1)
        res.push({ t: page.title, sub: page.group, url: page.out });
      (page.headings || []).forEach(function (h) {
        if (q && h.text.toLowerCase().indexOf(q) > -1) res.push({ t: h.text, sub: page.title, url: page.out + '#' + h.id });
      });
    });
    res = res.slice(0, 24); items = res; sel = 0;
    if (!res.length) { out.innerHTML = '<div class="search-empty">No matches.</div>'; return; }
    out.innerHTML = res.map(function (r, i) {
      return '<a class="search-item' + (i === 0 ? ' sel' : '') + '" href="' + r.url + '"><span class="si-t">' + r.t + '</span><span class="si-s">' + r.sub + '</span></a>';
    }).join('');
  }
  function move(d) {
    var els = out.querySelectorAll('.search-item'); if (!els.length) return;
    els[sel] && els[sel].classList.remove('sel');
    sel = (sel + d + els.length) % els.length;
    els[sel].classList.add('sel'); els[sel].scrollIntoView({ block: 'nearest' });
  }

  document.addEventListener('click', function (e) {
    if (e.target.closest('[data-search-open]')) open();
    if (e.target.closest('[data-search-close]')) close();
  });
  document.addEventListener('keydown', function (e) {
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') { e.preventDefault(); modal.hidden ? open() : close(); return; }
    if (modal.hidden) return;
    if (e.key === 'Escape') close();
    else if (e.key === 'ArrowDown') { e.preventDefault(); move(1); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); move(-1); }
    else if (e.key === 'Enter') { var el = out.querySelectorAll('.search-item')[sel]; if (el) location.href = el.getAttribute('href'); }
  });
  input.addEventListener('input', function () { render(input.value); });
})();
</script>`;

export function shell(opts: ShellOpts): string {
  const active = opts.active ?? "home";
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>${opts.title}</title>
<meta name="description" content="${opts.description}" />
<meta name="theme-color" content="#f4f2ec" />
<meta property="og:type" content="website" />
<meta property="og:title" content="${opts.title}" />
<meta property="og:description" content="${opts.description}" />
<meta property="og:url" content="${SITE}/" />
<meta property="og:image" content="${SITE}/assets/og.svg" />
<meta name="twitter:card" content="summary_large_image" />
<link rel="icon" type="image/svg+xml" href="assets/favicon.svg" />
<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Plus+Jakarta+Sans:wght@500;600;700;800&family=Inter:wght@400;500;600&family=JetBrains+Mono:wght@400;500&display=swap" />
<link rel="stylesheet" href="theme/style.css" />
</head>
<body>
${header(active)}
${opts.body}
${footer()}
${searchModal()}
${SCRIPT}
${opts.tail ?? ""}
</body>
</html>`;
}
