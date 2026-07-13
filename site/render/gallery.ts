// The gallery: every component family shown as a captioned specimen plate,
// grouped into numbered bands. Mocks reuse the `.g-*` classes from theme/style.css.

import { shell } from "./shell";
import { lucide, plate, ribbon } from "./bits";

type Plate = { name: string; note: string; href: string; preview: string; wide?: boolean };
type Band = { idx: string; kicker: string; plates: Plate[] };

const bands: Band[] = [
  {
    idx: "01",
    kicker: "buttons & actions",
    plates: [
      {
        name: "Button",
        note: "filled · light · outline · subtle",
        href: "buttons.html",
        preview: `<div class="g-group">
          <button class="g-btn g-btn--filled">Filled</button>
          <button class="g-btn g-btn--light">Light</button>
          <button class="g-btn g-btn--outline">Outline</button>
          <button class="g-btn g-btn--subtle">Subtle</button>
        </div>`,
      },
      {
        name: "ActionIcon · ThemeIcon",
        note: "compact icon buttons — Lucide built in",
        href: "buttons.html",
        preview: `<div class="g-group">
          <span class="g-aicon is-blue">${lucide("settings")}</span>
          <span class="g-aicon is-grape">${lucide("star")}</span>
          <span class="g-aicon is-teal">${lucide("check")}</span>
          <span class="g-aicon is-red">${lucide("x")}</span>
          <span class="g-themeicon is-violet">${lucide("play")}</span>
        </div>`,
      },
      {
        name: "Badge · Chip",
        note: "status & selection",
        href: "data.html",
        preview: `<div class="g-col g-col--gap">
          <div class="g-group g-badges">
            <span class="g-badge is-blue">Blue</span>
            <span class="g-badge is-teal">Teal</span>
            <span class="g-badge is-grape">Grape</span>
            <span class="g-badge is-orange">Orange</span>
            <span class="g-badge is-red">Red</span>
          </div>
          <div class="g-group">
            <span class="g-chip is-on">&#10003; React</span>
            <span class="g-chip">Vue</span>
            <span class="g-chip">Svelte</span>
          </div>
        </div>`,
      },
    ],
  },
  {
    idx: "02",
    kicker: "inputs & forms",
    plates: [
      {
        name: "TextInput · Select",
        note: "labelled fields",
        href: "inputs.html",
        preview: `<div class="g-col">
          <div class="g-field"><span class="g-label">Email</span><div class="g-input"><span>you@studio.dev</span><i class="g-caret"></i></div></div>
          <div class="g-field"><span class="g-label">Framework</span><div class="g-select"><span>gpui</span><i class="g-chev"></i></div></div>
        </div>`,
      },
      {
        name: "Switch · Checkbox · Radio",
        note: "controlled toggles",
        href: "inputs.html",
        preview: `<div class="g-col g-col--gap">
          <div class="g-switchrow"><span class="g-switch is-on"><i></i></span><span class="g-dimmed">Notifications</span></div>
          <div class="g-checkrow"><span class="g-check is-on">&#10003;</span><span class="g-dimmed">I agree to terms</span></div>
          <div class="g-radiorow"><span class="g-radio is-on"></span><span class="g-dimmed">Monthly billing</span></div>
        </div>`,
      },
      {
        name: "Slider · SegmentedControl",
        note: "range & exclusive choice",
        href: "inputs.html",
        preview: `<div class="g-col g-col--gap">
          <div class="g-slider"><span class="g-slider-fill" style="width:62%"></span><span class="g-slider-knob" style="left:62%"></span></div>
          <div class="g-segmented"><span class="is-on">Day</span><span>Week</span><span>Month</span></div>
        </div>`,
      },
    ],
  },
  {
    idx: "03",
    kicker: "feedback",
    plates: [
      {
        name: "Alert",
        note: "inline messages",
        href: "feedback.html",
        preview: `<div class="g-col g-col--gap">
          <div class="g-alert"><span class="g-alert-bar is-teal"></span><div><strong>Saved</strong><span class="g-dimmed">All changes synced.</span></div></div>
          <div class="g-alert"><span class="g-alert-bar is-red"></span><div><strong>Build failed</strong><span class="g-dimmed">2 errors in theme.rs</span></div></div>
        </div>`,
      },
      {
        name: "Progress · RingProgress",
        note: "determinate progress",
        href: "feedback.html",
        preview: `<div class="g-col g-col--gap">
          <div class="g-progress"><span style="width:42%"></span></div>
          <div class="g-progress"><span class="is-teal" style="width:78%"></span></div>
          <div class="g-group"><span class="g-ring"></span><span class="g-ring is-grape"></span></div>
        </div>`,
      },
      {
        name: "Loader · Skeleton",
        note: "indeterminate & placeholder",
        href: "feedback.html",
        preview: `<div class="g-col g-col--gap">
          <div class="g-group"><span class="g-loader"></span><span class="g-loader is-grape"></span><span class="g-loader is-teal"></span></div>
          <div class="g-skel" style="width:90%"></div>
          <div class="g-skel" style="width:60%"></div>
        </div>`,
      },
    ],
  },
  {
    idx: "04",
    kicker: "data display",
    plates: [
      {
        name: "Tabs · AvatarGroup",
        note: "panels & people",
        href: "data.html",
        preview: `<div class="g-col">
          <div class="g-tabs"><span class="is-on">Overview</span><span>Activity</span><span>Members</span></div>
          <div class="g-avatars"><i class="is-blue">WC</i><i class="is-grape">AD</i><i class="is-teal">JR</i><i class="more">+3</i></div>
        </div>`,
      },
      {
        name: "Table",
        note: "tabular data",
        href: "data.html",
        wide: true,
        preview: `<div class="g-table">
          <div class="g-tr g-th"><span>Component</span><span>Kind</span><span>Status</span></div>
          <div class="g-tr"><span>Button</span><span>builder</span><span class="g-badge is-teal">stable</span></div>
          <div class="g-tr"><span>TextInput</span><span>entity</span><span class="g-badge is-teal">stable</span></div>
          <div class="g-tr"><span>WebView</span><span>entity</span><span class="g-badge is-blue">new</span></div>
        </div>`,
      },
      {
        name: "Timeline",
        note: "ordered events",
        href: "data.html",
        preview: `<div class="g-timeline">
          <div class="g-tl-item"><i></i><div><strong>Committed</strong><span class="g-dimmed">add WebView</span></div></div>
          <div class="g-tl-item"><i></i><div><strong>Reviewed</strong><span class="g-dimmed">2 approvals</span></div></div>
          <div class="g-tl-item is-last"><i></i><div><strong>Merged</strong><span class="g-dimmed">to main</span></div></div>
        </div>`,
      },
    ],
  },
  {
    idx: "05",
    kicker: "overlays",
    plates: [
      {
        name: "Menu",
        note: "contextual actions",
        href: "overlays.html",
        preview: `<div class="g-menu">
          <span class="g-menu-h">Actions</span>
          <span class="g-menu-i">Duplicate</span>
          <span class="g-menu-i is-on">Rename</span>
          <span class="g-menu-i">Move to…</span>
          <span class="g-menu-i is-danger">Delete</span>
        </div>`,
      },
      {
        name: "Modal · Drawer",
        note: "focused layers",
        href: "overlays.html",
        preview: `<div class="g-modalmock">
          <div class="g-modal">
            <span class="g-title">Delete project?</span>
            <span class="g-dimmed">This action cannot be undone.</span>
            <div class="g-group"><button class="g-btn g-btn--subtle">Cancel</button><button class="g-btn g-btn--filled is-red">Delete</button></div>
          </div>
        </div>`,
      },
      {
        name: "Tooltip · Popover",
        note: "hover & click surfaces",
        href: "overlays.html",
        preview: `<div class="g-col g-col--center">
          <span class="g-tooltip">Copy link</span>
          <button class="g-btn g-btn--light">Share</button>
        </div>`,
      },
    ],
  },
  {
    idx: "06",
    kicker: "navigation",
    plates: [
      {
        name: "Stepper",
        note: "multi-step flows",
        href: "navigation.html",
        wide: true,
        preview: `<div class="g-stepper">
          <div class="g-step is-done"><i>&#10003;</i><span>Account</span></div>
          <div class="g-step is-on"><i>2</i><span>Profile</span></div>
          <div class="g-step"><i>3</i><span>Review</span></div>
        </div>`,
      },
      {
        name: "Pagination",
        note: "paged collections",
        href: "navigation.html",
        preview: `<div class="g-pager"><span>&#8249;</span><span>1</span><span class="is-on">2</span><span>3</span><span>&#8230;</span><span>9</span><span>&#8250;</span></div>`,
      },
      {
        name: "Breadcrumbs · NavLink",
        note: "wayfinding",
        href: "navigation.html",
        preview: `<div class="g-col g-col--gap">
          <div class="g-crumbs"><a>Home</a><i>/</i><a>Projects</a><i>/</i><span>guise</span></div>
          <div class="g-navlinks"><span class="g-navlink is-on">&#9679; Dashboard</span><span class="g-navlink">&#9679; Settings</span></div>
        </div>`,
      },
    ],
  },
  {
    idx: "07",
    kicker: "layout & web",
    plates: [
      {
        name: "Card · Paper",
        note: "surfaces",
        href: "layout.html",
        preview: `<div class="g-cardmock">
          <span class="g-title">Pro plan</span>
          <span class="g-dimmed">Everything, unlimited.</span>
          <div class="g-row"><span class="g-price">$12<em>/mo</em></span><button class="g-btn g-btn--filled">Upgrade</button></div>
        </div>`,
      },
      {
        name: "WebView",
        note: "WKWebView · WebView2 · WebKitGTK",
        href: "webview.html",
        wide: true,
        preview: `<div class="g-webview">
          <div class="g-webbar"><i></i><i></i><i></i><span>https://example.com</span></div>
          <div class="g-webbody"><strong>Native WebView</strong><span>Embedded via wry, positioned in guise layout.</span></div>
        </div>`,
      },
    ],
  },
];

const ACCENTS = ["band-violet", "band-blue", "band-grape", "band-teal", "band-pink", "band-blue", "band-grape"];

export function renderGallery(): string {
  const sections = bands
    .map((b, i) => {
      const plates = b.plates
        .map((p) => plate({ index: b.idx, name: p.name, note: p.note, href: p.href, preview: p.preview, wide: p.wide }))
        .join("");
      return `<section class="band ${ACCENTS[i % ACCENTS.length]}">
  <div class="container">
    <div class="band-head"><h2><span class="gw">${b.idx}</span> &nbsp;${b.kicker}</h2></div>
    <div class="plate-grid">${plates}</div>
  </div>
</section>`;
    })
    .join("");

  const body = `
<section class="hero">
  <div class="container">
    <span class="eyebrow">Gallery · ~60 components · one palette</span>
    <h1 class="display">The component <span class="grad">gallery</span>.</h1>
    <p class="lead">Every guise component family, grouped and labelled. The styling here is mocked for the web; the real thing renders natively through gpui. Follow any plate to its documentation.</p>
    ${ribbon(false)}
  </div>
</section>
${sections}`;

  return shell({
    title: "Gallery — guise components",
    description:
      "A specimen wall of guise components: buttons, inputs, feedback, data display, overlays, navigation, layout, and a native WebView — every family on one Mantine palette.",
    body,
    active: "gallery",
  });
}
