// The HTML document shell shared by every page: <head> metadata, the top
// header bar, and the footer. Editorial "specimen sheet" chrome on a light
// paper canvas. Pages pass their own <body> contents in.

const SITE = "https://wess.github.io/guise";
const REPO = "https://github.com/wess/guise";

export type ShellOpts = {
  title: string;
  description: string;
  body: string;
  active?: "docs" | "gallery" | "home";
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
    <nav class="topnav">
      ${link("docs.html", "Docs", "docs")}
      ${link("gallery.html", "Gallery", "gallery")}
      <a href="${REPO}" class="ext" rel="noreferrer">GitHub &#8599;</a>
    </nav>
    <button class="navtoggle" aria-label="Toggle navigation">&#9776;</button>
  </div>
</header>`;
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
      <a href="https://github.com/zed-industries/zed" rel="noreferrer">gpui</a>
      <a href="https://mantine.dev" rel="noreferrer">Mantine</a>
      <a href="https://crates.io/crates/wry" rel="noreferrer">wry</a>
    </div>
  </div>
  <div class="container foot-base"><span>&copy; 2026 Wess Cope</span><span>guise &mdash; a design specimen</span></div>
</footer>`;
}

// Tiny progressive-enhancement script: copy buttons + mobile nav toggle.
const SCRIPT = `<script>
document.addEventListener('click', function (e) {
  var b = e.target.closest('[data-copy]');
  if (b) {
    navigator.clipboard && navigator.clipboard.writeText(b.getAttribute('data-copy'));
    var t = b.textContent; b.textContent = 'Copied'; b.classList.add('ok');
    setTimeout(function () { b.textContent = t; b.classList.remove('ok'); }, 1200);
  }
  if (e.target.closest('.navtoggle')) document.body.classList.toggle('nav-open');
});
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
${SCRIPT}
${opts.tail ?? ""}
</body>
</html>`;
}
