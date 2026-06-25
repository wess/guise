// The documentation home (docs.html): a styled landing with a card per page,
// grouped by section — a comprehensive, scannable entry point to the docs.

import { shell } from "./shell";
import { groups, outFor } from "./nav";

const blurb: Record<string, string> = {
  gettingstarted: "Add the crate, install a theme, and render your first window.",
  theming: "The palette, scales, color scheme, and semantic colors.",
  components: "RenderOnce builders vs. stateful entities, variants, and events.",
  buttons: "Button, ActionIcon, CloseButton, ThemeIcon, and CopyButton.",
  icons: "The Icon component and the IconName glyph set.",
  inputs: "TextInput, Select, Combobox, Slider, checkboxes, switches, and more.",
  typography: "Text, Title, Anchor, Code, and Kbd.",
  layout: "Stack, Group, Center, SimpleGrid, Card, Paper, and Divider.",
  feedback: "Alert, Loader, Progress, RingProgress, Notification, and toasts.",
  data: "Avatar, Badge, List, Table, Timeline, Tabs, and Accordion.",
  overlays: "Modal, Drawer, Menu, Popover, Spotlight, and Tooltip.",
  navigation: "Breadcrumbs, NavLink, Stepper, Pagination, and StatusBar.",
  webview: "A native OS web view embedded via wry.",
  flex: "Flutter-style Row, Column, Expanded, Container, and Wrap.",
  macros: "The row! / col! / zstack! layout macros and style! / color!.",
  transitions: "Transition and Collapse mount animations.",
  reactive: "Signal, context / provider, hooks, and FormState validation.",
  windowmenu: "Wiring the native application menu.",
  architecture: "Workspace layout, the gpui dependency, and adding a component.",
};

export function renderDocsIndex(): string {
  const sections = groups
    .map((group) => {
      const cards = group.items
        .filter((it) => it.slug !== "docs")
        .map((it) => {
          const href = outFor(it.slug);
          const desc = blurb[it.slug] ?? "";
          return `<a class="doccard" href="${href}"><h3>${it.title} <span class="arr">&#8594;</span></h3><p>${desc}</p></a>`;
        })
        .join("");
      return `<section class="docsec">
  <h2 class="docsec-h">${group.title}</h2>
  <div class="doccards">${cards}</div>
</section>`;
    })
    .join("");

  const body = `<main class="docindex">
  <div class="container">
    <span class="eyebrow">Documentation</span>
    <h1 class="display">Build with <span class="grad">guise</span>.</h1>
    <p class="lead">Everything from installing the crate to embedding a native web view. Press <kbd class="kbd-inline">&#8984; K</kbd> to search, or start with the essentials.</p>
    <div class="hero-cta">
      <a class="btn btn-primary" href="gettingstarted.html">Installation</a>
      <a class="btn btn-ghost" href="components.html">Component model</a>
    </div>
    ${sections}
  </div>
</main>`;

  return shell({
    title: "Documentation — guise",
    description:
      "guise documentation: installation, theming, the component model, every component family, layout, reactive state, and more.",
    body,
    active: "docs",
  });
}
