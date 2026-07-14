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
      { slug: "tutorial", title: "Tutorial" },
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
      { slug: "dates", title: "Dates & times" },
      { slug: "editor", title: "Editor" },
      { slug: "markdowneditor", title: "Markdown editor" },
      { slug: "typography", title: "Typography" },
      { slug: "layout", title: "Layout" },
      { slug: "panels", title: "Panels" },
      { slug: "feedback", title: "Feedback" },
      { slug: "data", title: "Data display" },
      { slug: "charts", title: "Charts" },
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

// Reading order across all groups, as { title, out, group }.
export function flatNav(): { title: string; out: string; group: string }[] {
  const flat: { title: string; out: string; group: string }[] = [];
  for (const group of groups) {
    for (const item of group.items) {
      const out = outFor(item.slug === "docs" ? "readme" : item.slug);
      flat.push({ title: item.title, out, group: group.title });
    }
  }
  return flat;
}

// The group (section) title a page belongs to — used for the breadcrumb.
export function groupOf(out: string): string {
  return flatNav().find((p) => p.out === out)?.group ?? "Docs";
}

// Previous / next page in reading order.
export function prevNext(out: string): {
  prev?: { title: string; out: string };
  next?: { title: string; out: string };
} {
  const flat = flatNav();
  const i = flat.findIndex((p) => p.out === out);
  return {
    prev: i > 0 ? flat[i - 1] : undefined,
    next: i >= 0 && i < flat.length - 1 ? flat[i + 1] : undefined,
  };
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
