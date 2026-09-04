// The documentation home (docs.html): a styled landing with a card per page,
// grouped by section — a comprehensive, scannable entry point to the docs.

import { shell } from "./shell";
import { groups, outFor } from "./nav";

const blurb: Record<string, string> = {
  gettingstarted: "Add the crate, install a theme, and render your first window.",
  tutorial: "Build a SQL data workbench end to end — eleven chapters, one app.",
  appguide: "A project tracker wired the way a real app fits together: forms, overlays, reordering, motion.",
  theming: "The palette, scales, color scheme, and semantic colors.",
  components: "RenderOnce builders vs. stateful entities, variants, and events.",
  buttons: "Button, ActionIcon, CloseButton, ThemeIcon, and CopyButton.",
  icons: "The Icon component and the embedded Lucide icon set.",
  inputs: "TextInput, Select, Combobox, NumberInput, Slider, Rating, and more.",
  dates: "Calendar, DatePicker and TimePicker, over pure Date / Time models.",
  files: "FileInput's native dialog and Dropzone's OS drag-and-drop.",
  ai: "A chat kit: transcript, composer, streaming text, tool calls, citations, cost — transport-agnostic.",
  markdowneditor: "MarkdownEditor: an Obsidian-style live-preview editor, and the read-only Markdown renderer.",
  editor: "A multiline code editor: gutter, syntax highlighting, styling, highlights, and undo/redo.",
  typography: "Text, Title, Anchor, Code, Kbd, Mark, and Spoiler.",
  layout: "Stack, Group, SimpleGrid, Card, Paper, ScrollArea, and AppShell.",
  panels: "Panel header/footer framing, and SplitPanel's draggable divider.",
  feedback: "Alert, Loader, Progress, RingProgress, ToastStack, and Skeleton.",
  data: "Avatar, Badge, Table, TableView, DataView, TreeView, Tabs, and TabBar.",
  charts: "Sparkline, BarChart, LineChart, and PieChart, painted on canvas.",
  overlays: "Modal, ConfirmModal, Menu, ContextMenu, Popover, Spotlight, and Drawer.",
  navigation: "Breadcrumbs, NavLink, Stepper, Pagination, and StatusBar.",
  webview: "A native OS web view embedded via wry.",
  flex: "Flutter-style Row, Column, Expanded, Container, and Wrap.",
  macros: "row! / col! / zstack! for containers, style! and color!, and motion! / sequence! for animation.",
  transitions: "Keyframed Motion, sequences, stagger, a playhead you can scrub, springs, and Presence exit animations.",
  motiontutorial: "Build one animated panel, nine chapters — entrances, stagger, keyframes, a playhead, and exits.",
  dnd: "Draggable, DropTarget and SortableList, with typed drag payloads.",
  update: "Check a release feed, verify the signature, install in place, and restart.",
  devtools: "An in-app Safari-shaped inspector: Elements, Styles, Logs, Network, Timelines, Audit.",
  settings: "SettingsView, SettingsSection and SettingsRow — the settings screen, without a schema to adopt.",
  reactive: "Signal, Binding, context / provider, hooks, and FormState validation.",
  windowmenu: "Wiring the native application menu.",
  architecture: "Workspace layout, the gpui dependency, and adding a component.",
  release: "Cutting a version, signing and notarizing Tailor, publishing the crate.",
  performance: "What the crate costs to compile, link and render, and where the money goes.",

  // Tailor
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
    <p class="lead">Everything from installing the crate to embedding a native web view — and the whole of Tailor, the interface builder that draws with it. Press <kbd class="kbd-inline">&#8984; K</kbd> to search, or start with the essentials.</p>
    <div class="hero-cta">
      <a class="btn btn-primary" href="gettingstarted.html">Installation</a>
      <a class="btn btn-ghost" href="tutorial.html">Read the tutorial</a>
      <a class="btn btn-ghost" href="components.html">Component model</a>
      <a class="btn btn-ghost" href="https://github.com/wess/tailor" rel="noreferrer">Tailor</a>
    </div>
    ${sections}
  </div>
</main>`;

  return shell({
    title: "Documentation — guise",
    description:
      "guise documentation: installation, theming, the component model, every component family, layout, reactive state, and Tailor, the visual interface builder.",
    body,
    active: "docs",
  });
}
