// Renders one documentation page: markdown -> highlighted HTML, with `.md`
// links rewritten to `.html`, heading anchors injected, and a sidebar + table
// of contents wrapped around the article via the shared shell.

import { Marked } from "marked";
import { markedHighlight } from "marked-highlight";
import hljs from "highlight.js";

import { shell } from "./shell";
import { sidebar } from "./nav";

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

// `foo.md` / `foo.md#bar` -> `foo.html` / `foo.html#bar`; `readme.md` -> docs.
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

// Inject `id` on h2/h3 and collect them for the table of contents.
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

function tocHtml(toc: Heading[]): string {
  if (toc.length < 2) return "";
  const items = toc
    .map(
      (h) =>
        `<li class="lvl${h.level}"><a href="#${h.id}">${h.text}</a></li>`,
    )
    .join("");
  return `<aside class="toc"><span class="toc-title">On this page</span><ul>${items}</ul></aside>`;
}

export function renderDoc(opts: {
  md: string;
  out: string;
  title: string;
}): string {
  const raw = marked.parse(opts.md) as string;
  const linked = rewriteLinks(raw);
  const { html, toc } = anchorHeadings(linked);

  const body = `<div class="docshell">
  ${sidebar(opts.out)}
  <main class="docmain">
    <article class="doc">${html}</article>
  </main>
  ${tocHtml(toc)}
</div>`;

  return shell({
    title: `${opts.title} — guise`,
    description: `${opts.title} — documentation for guise, a Mantine-inspired component library for gpui.`,
    body,
    active: "docs",
  });
}
