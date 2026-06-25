// Renders one documentation page: markdown -> highlighted HTML, with `.md`
// links rewritten to `.html`, heading anchors, note/tip callouts, per-block
// copy buttons, a live component preview, breadcrumb, and prev/next paging.

import { Marked } from "marked";
import { markedHighlight } from "marked-highlight";
import hljs from "highlight.js";

import { shell } from "./shell";
import { sidebar, prevNext, groupOf } from "./nav";
import { previewsFor } from "./previews";

const REPO = "https://github.com/wess/guise";

const marked = new Marked(
  markedHighlight({
    langPrefix: "hljs language-",
    highlight(code: string, lang: string) {
      const language = hljs.getLanguage(lang) ? lang : "plaintext";
      return hljs.highlight(code, { language }).value;
    },
  }),
  { gfm: true },
);

function slugify(text: string): string {
  return text
    .toLowerCase()
    .replace(/<[^>]+>/g, "")
    .replace(/&[a-z]+;/g, "")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function rewriteLinks(html: string): string {
  return html.replace(
    /href="([a-z0-9]+)\.md(#[^"]*)?"/gi,
    (_m, slug: string, anchor = "") => {
      const out = slug.toLowerCase() === "readme" ? "docs" : slug;
      return `href="${out}.html${anchor ?? ""}"`;
    },
  );
}

type Heading = { level: number; id: string; text: string };

function anchorHeadings(html: string): { html: string; toc: Heading[] } {
  const toc: Heading[] = [];
  const out = html.replace(
    /<h([23])>([\s\S]*?)<\/h\1>/g,
    (_m, level: string, inner: string) => {
      const id = slugify(inner);
      toc.push({ level: Number(level), id, text: inner.replace(/<[^>]+>/g, "") });
      return `<h${level} id="${id}"><a class="anchor" href="#${id}" aria-hidden="true">#</a>${inner}</h${level}>`;
    },
  );
  return { html: out, toc };
}

// Turn blockquotes that begin with a keyword into colored callouts.
function admonitions(html: string): string {
  return html.replace(/<blockquote>([\s\S]*?)<\/blockquote>/g, (m, inner: string) => {
    const kw = inner.match(/<strong>\s*([A-Za-z][A-Za-z ]*?)\s*[:.]?\s*<\/strong>/);
    if (!kw) return m;
    const w = kw[1].toLowerCase();
    let kind = "note";
    if (/(warn|caution|danger|avoid|don't|never)/.test(w)) kind = "warn";
    else if (/(tip|hint|note that)/.test(w)) kind = "tip";
    const label = kw[1];
    const body = inner.replace(/(<p>)\s*<strong>[^<]*?<\/strong>\s*[:.]?\s*/, "$1");
    return `<div class="admon admon-${kind}"><span class="admon-l">${label}</span><div class="admon-b">${body}</div></div>`;
  });
}

const decode = (s: string) =>
  s
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&amp;/g, "&");
const attr = (s: string) => s.replace(/&/g, "&amp;").replace(/'/g, "&#39;");

// Wrap each code block with a copy button carrying the raw source.
function codeCopy(html: string): string {
  return html.replace(/<pre>([\s\S]*?)<\/pre>/g, (_m, inner: string) => {
    const raw = decode(inner.replace(/<[^>]+>/g, ""));
    return `<div class="codeblock"><button class="codecopy" data-copy='${attr(raw)}' aria-label="Copy code">Copy</button><pre>${inner}</pre></div>`;
  });
}

function tocHtml(toc: Heading[]): string {
  if (toc.length < 2) return "";
  const items = toc
    .map((h) => `<li class="lvl${h.level}"><a href="#${h.id}">${h.text}</a></li>`)
    .join("");
  return `<aside class="toc"><span class="toc-title">On this page</span><ul>${items}</ul></aside>`;
}

function pageNav(out: string): string {
  const { prev, next } = prevNext(out);
  const p = prev
    ? `<a class="pagenav-link prev" href="${prev.out}"><span>&#8592; Previous</span><b>${prev.title}</b></a>`
    : `<span></span>`;
  const n = next
    ? `<a class="pagenav-link next" href="${next.out}"><span>Next &#8594;</span><b>${next.title}</b></a>`
    : `<span></span>`;
  return `<nav class="pagenav">${p}${n}</nav>`;
}

const DOC_SCRIPT = `<script>
(function () {
  var links = [].slice.call(document.querySelectorAll('.toc a'));
  var hs = links.map(function (a) { return document.getElementById(a.getAttribute('href').slice(1)); }).filter(Boolean);
  function spy() {
    var top = window.scrollY + 96, cur = null;
    for (var i = 0; i < hs.length; i++) { if (hs[i].offsetTop <= top) cur = hs[i]; }
    links.forEach(function (a) { a.parentElement.classList.toggle('active', !!cur && a.getAttribute('href') === '#' + cur.id); });
  }
  document.addEventListener('scroll', spy, { passive: true }); spy();
  var t = document.querySelector('.docs-toggle');
  if (t) t.addEventListener('click', function () { document.body.classList.toggle('docs-open'); });
})();
</script>`;

export function renderDoc(opts: { md: string; out: string; title: string }): string {
  const raw = marked.parse(opts.md) as string;
  let html = rewriteLinks(raw);
  const anchored = anchorHeadings(html);
  html = admonitions(anchored.html);
  html = codeCopy(html);

  const slug = opts.out === "docs.html" ? "docs" : opts.out.replace(/\.html$/, "");
  const src = opts.out === "docs.html" ? "readme.md" : `${slug}.md`;
  const preview = previewsFor(slug);
  const group = groupOf(opts.out);

  const body = `<div class="docshell">
  ${sidebar(opts.out)}
  <main class="docmain">
    <div class="doc-head">
      <div class="crumbs"><a href="docs.html">Docs</a> <span>/</span> <span>${group}</span></div>
      <button class="docs-toggle" aria-label="Toggle sidebar">Menu &#9776;</button>
    </div>
    ${preview}
    <article class="doc">${html}</article>
    ${pageNav(opts.out)}
    <div class="doc-foot"><a href="${REPO}/blob/main/docs/${src}" rel="noreferrer">Edit this page on GitHub &#8599;</a></div>
  </main>
  ${tocHtml(anchored.toc)}
</div>`;

  return shell({
    title: `${opts.title} — guise`,
    description: `${opts.title} — documentation for guise, a Mantine-inspired component library for gpui.`,
    body,
    active: "docs",
    tail: DOC_SCRIPT,
  });
}
