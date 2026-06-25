// The landing page. Dark, gradient-washed, component-forward — the centerpiece
// is a guise "app" inside a window frame with annotation callouts pointing at
// the components, in the spirit of the React Aria landing page.

import { shell } from "./shell";
import { ribbon, plate } from "./bits";

const REPO = "https://github.com/wess/guise";
const DEP = 'guise-ui = { git = "https://github.com/wess/guise" }';
const DEP_HTML = DEP.replace(/&/g, "&amp;").replace(/</g, "&lt;");

// The annotated hero exhibit: a guise interface with callout labels.
function exhibit(): string {
  const app = `<div class="g-app">
  <div class="g-stack">
    <div class="g-row"><span class="g-title">Create project</span><span class="g-badge is-violet">v0.1</span></div>
    <span class="g-dimmed">A Mantine-inspired component layer for gpui.</span>
    <div class="g-field"><span class="g-label">Name</span><div class="g-input"><span>guise-app</span><i class="g-caret"></i></div></div>
    <div class="g-group">
      <button class="g-btn g-btn--filled">Filled</button>
      <button class="g-btn g-btn--light">Light</button>
      <button class="g-btn g-btn--outline">Outline</button>
    </div>
    <div class="g-group g-badges">
      <span class="g-badge is-blue">Blue</span>
      <span class="g-badge is-teal">Teal</span>
      <span class="g-badge is-grape">Grape</span>
      <span class="g-badge is-orange">Orange</span>
    </div>
    <div class="g-switchrow"><span class="g-switch is-on"><i></i></span><span class="g-dimmed">Enable notifications</span></div>
    <div class="g-alert"><span class="g-alert-bar"></span><div><strong>Heads up</strong><span class="g-dimmed">Theme switches light / dark at runtime.</span></div></div>
    <div class="g-progress"><span style="width:68%"></span></div>
  </div>
</div>`;
  const cal = (cls: string, top: string, label: string) =>
    `<span class="cal ${cls}" style="top:${top}"><span>${label}</span></span>`;
  return `<div class="exhibit">
  <div class="exhibit-wash" aria-hidden="true"></div>
  <div class="winwrap">
    <div class="win" role="img" aria-label="A guise interface: a title, an input, buttons, badges, a switch, an alert and a progress bar.">
      <div class="win-bar"><span class="win-dots"><i></i><i></i><i></i></span><span class="win-title">guise · gallery</span></div>
      <div class="win-body">${app}</div>
    </div>
    ${cal("cal-l", "31%", "TextInput")}
    ${cal("cal-r", "46%", "Button · variant")}
    ${cal("cal-l", "60%", "Badge")}
    ${cal("cal-r", "73%", "Switch")}
    ${cal("cal-l", "85%", "Alert")}
  </div>
</div>`;
}

const SAMPLE_CODE = `<span class="t-c">// a stateful entity: owns its buffer, emits events</span>
<span class="t-k">let</span> name = cx.new(|cx| {
    <span class="t-t">TextInput</span>::new(cx)
        .label(<span class="t-s">"Name"</span>)
        .placeholder(<span class="t-s">"guise-app"</span>)
});

cx.subscribe(&name, |this, _input, e: &<span class="t-t">TextInputEvent</span>, cx| {
    <span class="t-k">if let</span> <span class="t-t">TextInputEvent</span>::Change(value) = e {
        this.name = value.clone();
        cx.notify();
    }
})
.detach();`;

type Feature = { icon: string; title: string; body: string };
const features: Feature[] = [
  { icon: "&#9632;", title: "Mantine palette, themed", body: "A 14-hue open-color palette, sizing tokens, and semantic colors. Read every value from the theme — light / dark switching is free." },
  { icon: "&#8943;", title: "Builder API", body: "Chainable builders that read like Mantine: <code>.variant()</code>, <code>.color()</code>, <code>.size()</code>, <code>.radius()</code>." },
  { icon: "&#9651;", title: "Stateful entities", body: "Inputs, overlays and data views are gpui entities that own their state and emit events you take with <code>cx.subscribe</code>." },
  { icon: "&#9707;", title: "Flex + macros", body: "A Flutter-style flexbox layer plus terse <code>row!</code> / <code>col!</code> / <code>zstack!</code> macros for dense layout." },
  { icon: "&#10022;", title: "Reactive state", body: "A lightweight React-style layer: <code>Signal</code>, <code>use_state</code>, <code>provide</code> / <code>use_context</code>, and <code>use_form</code> validation." },
  { icon: "&#9673;", title: "Native WebView", body: "Embed a real OS web view (WKWebView / WebView2 / WebKitGTK) via <code>wry</code>, positioned inside normal guise layout." },
];

function bandHead(h: string, p: string, learnHref?: string, learnLabel?: string): string {
  const learn = learnHref ? `<a class="learn" href="${learnHref}">${learnLabel} &#8594;</a>` : "";
  return `<div class="band-head"><h2>${h}</h2><p>${p}</p>${learn}</div>`;
}

export function renderLanding(): string {
  const featureCards = features
    .map((f) => `<article class="feature"><span class="ic">${f.icon}</span><h3>${f.title}</h3><p>${f.body}</p></article>`)
    .join("");

  const teaser = [
    plate({ index: "01", name: "Button", note: "variant · color · size", href: "buttons.html", preview: `<div class="g-group"><button class="g-btn g-btn--filled">Filled</button><button class="g-btn g-btn--light">Light</button><button class="g-btn g-btn--outline">Outline</button></div>` }),
    plate({ index: "02", name: "Inputs", note: "TextInput · Select", href: "inputs.html", preview: `<div class="g-col"><div class="g-field"><span class="g-label">Email</span><div class="g-input"><span>you@studio.dev</span></div></div><div class="g-field"><span class="g-label">Framework</span><div class="g-select"><span>gpui</span><i class="g-chev"></i></div></div></div>` }),
    plate({ index: "03", name: "Controls", note: "Switch · Checkbox · Slider", href: "inputs.html", preview: `<div class="g-col g-col--gap"><div class="g-switchrow"><span class="g-switch is-on"><i></i></span><span class="g-dimmed">Sync</span></div><div class="g-checkrow"><span class="g-check is-on">&#10003;</span><span class="g-dimmed">I agree</span></div><div class="g-slider"><span class="g-slider-fill" style="width:55%"></span><span class="g-slider-knob" style="left:55%"></span></div></div>` }),
    plate({ index: "04", name: "Feedback", note: "Alert · Progress · Loader", href: "feedback.html", preview: `<div class="g-col g-col--gap"><div class="g-alert"><span class="g-alert-bar is-teal"></span><div><strong>Saved</strong><span class="g-dimmed">All changes synced.</span></div></div><div class="g-progress"><span style="width:42%"></span></div><div class="g-group"><span class="g-ring"></span><span class="g-loader"></span></div></div>` }),
    plate({ index: "05", name: "Data", note: "Tabs · AvatarGroup", href: "data.html", preview: `<div class="g-col"><div class="g-tabs"><span class="is-on">Overview</span><span>Activity</span><span>Members</span></div><div class="g-avatars"><i class="is-blue">WC</i><i class="is-grape">AD</i><i class="is-teal">JR</i><i class="more">+3</i></div></div>` }),
    plate({ index: "06", name: "Overlays", note: "Menu · Popover · Modal", href: "overlays.html", preview: `<div class="g-menu"><span class="g-menu-h">Actions</span><span class="g-menu-i">Duplicate</span><span class="g-menu-i is-on">Rename</span><span class="g-menu-i is-danger">Delete</span></div>` }),
  ].join("");

  const systems = [
    ["Flex layout", "Row · Column · Expanded · Wrap", "flex.html"],
    ["Layout macros", "row! · col! · zstack! · style!", "macros.html"],
    ["Reactive state", "Signal · use_state · use_form", "reactive.html"],
    ["Transitions", "Transition · Collapse", "transitions.html"],
    ["Theming", "14 hues × 10 shades · light / dark", "theming.html"],
    ["Native WebView", "WKWebView · WebView2 · WebKitGTK", "webview.html"],
  ]
    .map(
      ([t, n, h], i) =>
        `<a class="sysrow" href="${h}"><span class="sys-n">${String(i + 1).padStart(2, "0")}</span><span class="sys-t">${t}</span><span class="sys-d">${n}</span><span class="sys-x">&#8594;</span></a>`,
    )
    .join("");

  const endCards = [
    ["Install &amp; setup", "Add the dependency, install a theme, render your first window.", "gettingstarted.html"],
    ["Browse components", "Every component family on one palette, in the gallery.", "gallery.html"],
    ["Read the docs", "Theming, the component model, layout, reactive state and more.", "docs.html"],
  ]
    .map(([t, d, h]) => `<a class="endcard" href="${h}"><h3>${t} &#8594;</h3><p>${d}</p></a>`)
    .join("");

  const body = `
<section class="hero">
  <div class="container">
    <span class="eyebrow">Native UI · built on Zed's gpui</span>
    <h1 class="display">Native UI components<br />for <span class="grad">Rust</span>.</h1>
    <p class="lead">
      guise brings <a href="https://mantine.dev" rel="noreferrer">Mantine's</a> ergonomics to
      <a href="https://github.com/zed-industries/zed" rel="noreferrer">gpui</a> — a themed palette,
      sizing tokens, and roughly sixty composable components, GPU-rendered at native speed.
    </p>
    <div class="hero-cta">
      <a class="btn btn-primary" href="gettingstarted.html">Get started</a>
      <a class="btn btn-ghost" href="gallery.html">Explore components</a>
    </div>
    <div class="cmd" style="margin-left:auto;margin-right:auto;">
      <code>${DEP_HTML}</code>
      <button class="copybtn" data-copy='${DEP}' aria-label="Copy dependency line">Copy</button>
    </div>
    ${exhibit()}
    ${ribbon(true)}
    <p class="ribbon-cap">The palette — 14 open-color hues, 10 shades each, read straight from the theme.</p>
  </div>
</section>

<section class="band band-violet">
  <div class="container">
    ${bandHead(`Everything you need to <span class="gw">build a desktop app</span>.`, "A complete component layer, not a widget grab-bag — themed, composable, and native from the first line.")}
    <div class="feature-grid">${featureCards}</div>
  </div>
</section>

<section class="band band-blue">
  <div class="container">
    ${bandHead(`If you know Mantine, you already know <span class="gw">guise</span>.`, "Controlled widgets take a value and a handler. Stateful ones are entities you create with cx.new and subscribe to. Same mental model — native rendering underneath.", "components.html", "Read the component model")}
    <div class="split">
      <div class="split-copy">
        <ul class="ticks">
          <li>Variant + color + size on every component</li>
          <li>Events via <code>cx.subscribe</code>, redraws via <code>cx.notify</code></li>
          <li>No hardcoded colors or spacing — only theme tokens</li>
          <li>Drop to the flex layer or macros when you want them</li>
        </ul>
      </div>
      <div class="codepanel">
        <div class="codepanel-bar"><i></i><i></i><i></i><span>text.rs</span></div>
        <pre class="code"><code>${SAMPLE_CODE}</code></pre>
      </div>
    </div>
  </div>
</section>

<section class="band band-grape">
  <div class="container">
    ${bandHead(`Roughly sixty components, <span class="gw">one palette</span>.`, "From buttons to a native web view — a selection of the families below.")}
    <div class="plate-grid">${teaser}</div>
    <a class="more-link" href="gallery.html">See every component in the gallery &#8594;</a>
  </div>
</section>

<section class="band band-teal">
  <div class="container">
    ${bandHead(`Systems, not just <span class="gw">widgets</span>.`, "Layout, motion and state — the parts of an app a component grab-bag leaves out.")}
    <div class="syslist">${systems}</div>
  </div>
</section>

<section class="band band-pink">
  <div class="container">
    ${bandHead(`Ready to <span class="gw">get started</span>?`, "Add the dependency and open your first window.")}
    <div class="cmd"><code>${DEP_HTML}</code><button class="copybtn" data-copy='${DEP}' aria-label="Copy dependency line">Copy</button></div>
    <div class="endgrid">${endCards}</div>
  </div>
</section>`;

  return shell({
    title: "guise — native UI components for Rust, on gpui",
    description:
      "A Mantine-inspired component library for gpui (Zed's GPU UI framework): themed palette, sizing tokens, and ~60 composable, GPU-rendered components for native Rust desktop apps.",
    body,
    active: "home",
  });
}
