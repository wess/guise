// Build the static site into ./dist:
//   index.html      landing page
//   gallery.html    component gallery
//   <slug>.html     one page per ../docs/<slug>.md
//   theme/style.css, assets/*, .nojekyll
//
// Run with `bun run build.ts` (or `bun run build`).

import { renderLanding } from "./render/landing";
import { renderGallery } from "./render/gallery";
import { renderDoc } from "./render/doc";
import { docPages } from "./render/nav";
import { favicon, ogImage } from "./assets/svg";

const root = import.meta.dir;
const docsDir = `${root}/../docs`;
const out = `${root}/dist`;

async function write(rel: string, content: string) {
  await Bun.write(`${out}/${rel}`, content);
}

console.log("building guise site …");

// Top-level pages.
await write("index.html", renderLanding());
await write("gallery.html", renderGallery());

// One page per documentation markdown file.
for (const page of docPages()) {
  const md = await Bun.file(`${docsDir}/${page.src}`).text();
  await write(page.out, renderDoc({ md, out: page.out, title: page.title }));
  console.log(`  doc  ${page.src} -> ${page.out}`);
}

// Stylesheet + assets.
await write("theme/style.css", await Bun.file(`${root}/theme/style.css`).text());
await write("assets/favicon.svg", favicon());
await write("assets/og.svg", ogImage());
await write(".nojekyll", "");

console.log(`done -> ${out}`);
