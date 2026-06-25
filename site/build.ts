// Build the static site into ./dist:
//   index.html      landing page
//   gallery.html    component gallery
//   docs.html       documentation home (styled index)
//   <slug>.html     one page per ../docs/<slug>.md
//   searchindex.json, theme/style.css, assets/*, .nojekyll
//
// Run with `bun run build.ts` (or `bun run build`).

import { renderLanding } from "./render/landing";
import { renderGallery } from "./render/gallery";
import { renderDocsIndex } from "./render/docsindex";
import { renderDoc } from "./render/doc";
import { docPages, groupOf } from "./render/nav";
import { favicon, ogImage } from "./assets/svg";

const root = import.meta.dir;
const docsDir = `${root}/../docs`;
const out = `${root}/dist`;

async function write(rel: string, content: string) {
  await Bun.write(`${out}/${rel}`, content);
}

function slugify(text: string): string {
  return text
    .replace(/`([^`]*)`/g, "$1")
    .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1")
    .replace(/[*_]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function plain(md: string): string {
  return md
    .replace(/```[\s\S]*?```/g, " ")
    .replace(/^#{1,6}\s+.*$/gm, " ")
    .replace(/[`*_>#|]/g, " ")
    .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1")
    .replace(/\s+/g, " ")
    .trim();
}

type IndexEntry = {
  title: string;
  out: string;
  group: string;
  headings: { text: string; id: string }[];
  text: string;
};

console.log("building guise site …");

await write("index.html", renderLanding());
await write("gallery.html", renderGallery());
await write("docs.html", renderDocsIndex());

const searchIndex: IndexEntry[] = [];

for (const page of docPages()) {
  const md = await Bun.file(`${docsDir}/${page.src}`).text();

  if (page.out !== "docs.html") {
    await write(page.out, renderDoc({ md, out: page.out, title: page.title }));
    console.log(`  doc  ${page.src} -> ${page.out}`);
  }

  const headings = [...md.matchAll(/^#{2,3}\s+(.+?)\s*$/gm)].map((m) => {
    const text = m[1].replace(/`([^`]*)`/g, "$1").replace(/[*_]/g, "").trim();
    return { text, id: slugify(m[1]) };
  });
  searchIndex.push({
    title: page.title,
    out: page.out,
    group: groupOf(page.out),
    headings,
    text: plain(md).slice(0, 240),
  });
}

await write("searchindex.json", JSON.stringify(searchIndex));

// Stylesheet + assets.
await write("theme/style.css", await Bun.file(`${root}/theme/style.css`).text());
await write("assets/favicon.svg", favicon());
await write("assets/og.svg", ogImage());
await write(".nojekyll", "");

console.log(`done -> ${out}`);
