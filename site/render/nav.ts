// The documentation map: sidebar groups in reading order. Each entry's `slug`
// is the source markdown filename (without `.md`) under ../docs, and the output
// page is `<slug>.html`. `readme.md` is special-cased to `docs.html`.

export type DocEntry = { slug: string; title: string };
export type DocGroup = { title: string; items: DocEntry[] };

export const groups: DocGroup[] = [
  {
    title: "Getting started",
    items: [
      { slug: "docs", title: "Overview" },
      { slug: "gettingstarted", title: "Installation" },
    ],
  },
  {
    title: "Foundations",
    items: [
      { slug: "theming", title: "Theming" },
      { slug: "components", title: "Component model" },
    ],
  },
  {
    title: "Components",
    items: [
      { slug: "buttons", title: "Buttons" },
      { slug: "icons", title: "Icons" },
      { slug: "inputs", title: "Inputs" },
      { slug: "typography", title: "Typography" },
      { slug: "layout", title: "Layout" },
      { slug: "feedback", title: "Feedback" },
      { slug: "data", title: "Data display" },
      { slug: "overlays", title: "Overlays" },
      { slug: "navigation", title: "Navigation" },
      { slug: "webview", title: "WebView" },
    ],
  },
  {
    title: "Systems",
    items: [
      { slug: "flex", title: "Flex layout" },
      { slug: "macros", title: "Layout macros" },
      { slug: "transitions", title: "Transitions" },
      { slug: "reactive", title: "Reactive state" },
      { slug: "windowmenu", title: "Window menu" },
    ],
  },
  {
    title: "Reference",
    items: [{ slug: "architecture", title: "Architecture" }],
  },
];

// Source markdown filename -> output html filename. `readme` is the docs home.
export function outFor(slug: string): string {
  return slug === "readme" ? "docs.html" : `${slug}.html`;
}

// Flat list of every doc page to build: { src markdown, out html, title }.
export function docPages(): { src: string; out: string; title: string }[] {
  const pages: { src: string; out: string; title: string }[] = [
    { src: "readme.md", out: "docs.html", title: "Overview" },
  ];
  for (const group of groups) {
    for (const item of group.items) {
      if (item.slug === "docs") continue; // readme.md, already added
      pages.push({
        src: `${item.slug}.md`,
        out: `${item.slug}.html`,
        title: item.title,
      });
    }
  }
  return pages;
}

// Sidebar HTML, with the current page's link marked active.
export function sidebar(currentOut: string): string {
  const sections = groups
    .map((group) => {
      const links = group.items
        .map((item) => {
          const href = outFor(item.slug === "docs" ? "readme" : item.slug);
          const active = href === currentOut ? ' class="active"' : "";
          return `<li><a href="${href}"${active}>${item.title}</a></li>`;
        })
        .join("");
      return `<div class="navgroup"><span class="navgroup-title">${group.title}</span><ul>${links}</ul></div>`;
    })
    .join("");
  return `<nav class="sidebar">${sections}</nav>`;
}
