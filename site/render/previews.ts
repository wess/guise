// Live component previews shown at the top of component doc pages. The markup
// reuses the `.g-*` mock classes (styled in theme/style.css with guise's real
// palette), so each doc opens with a rendered look at what it documents.

import { lucide } from "./bits";

const stages: Record<string, string> = {
  buttons: `<div class="pv-wrap">
    <div class="g-group"><button class="g-btn g-btn--filled">Filled</button><button class="g-btn g-btn--light">Light</button><button class="g-btn g-btn--outline">Outline</button><button class="g-btn g-btn--subtle">Subtle</button></div>
    <div class="g-group"><span class="g-aicon is-blue">${lucide("settings")}</span><span class="g-aicon is-grape">${lucide("star")}</span><span class="g-aicon is-teal">${lucide("check")}</span><span class="g-aicon is-red">${lucide("x")}</span><span class="g-themeicon is-violet">${lucide("play")}</span></div>
  </div>`,

  components: `<div class="pv-wrap">
    <div class="g-group"><button class="g-btn g-btn--filled">Filled</button><button class="g-btn g-btn--light">Light</button><button class="g-btn g-btn--outline">Outline</button><button class="g-btn g-btn--subtle">Subtle</button></div>
    <div class="g-group g-badges"><span class="g-badge is-blue">Blue</span><span class="g-badge is-violet">Violet</span><span class="g-badge is-teal">Teal</span><span class="g-badge is-grape">Grape</span><span class="g-badge is-orange">Orange</span></div>
  </div>`,

  inputs: `<div class="pv-grid">
    <div class="g-field"><span class="g-label">Email</span><div class="g-input"><span>you@studio.dev</span><i class="g-caret"></i></div></div>
    <div class="g-field"><span class="g-label">Framework</span><div class="g-select"><span>gpui</span><i class="g-chev"></i></div></div>
    <div class="g-col g-col--gap"><div class="g-switchrow"><span class="g-switch is-on"><i></i></span><span class="g-dimmed">Notifications</span></div><div class="g-checkrow"><span class="g-check is-on">&#10003;</span><span class="g-dimmed">I agree</span></div></div>
    <div class="g-col g-col--gap"><div class="g-slider"><span class="g-slider-fill" style="width:60%"></span><span class="g-slider-knob" style="left:60%"></span></div><div class="g-segmented"><span class="is-on">Day</span><span>Week</span><span>Month</span></div></div>
  </div>`,

  typography: `<div class="pv-wrap pv-type">
    <span class="g-title" style="font-size:24px">The quick brown fox</span>
    <span class="g-dimmed">Body text reads from the theme — dimmed, sized, and colored by token.</span>
    <div class="g-group"><a class="g-anchor">Anchor link</a><code class="g-codeinline">Code::inline</code><span class="g-kbd">⌘</span><span class="g-kbd">K</span></div>
  </div>`,

  icons: `<div class="g-group">
    <span class="g-aicon is-blue">${lucide("sun")}</span><span class="g-aicon is-grape">${lucide("star")}</span><span class="g-aicon is-teal">${lucide("check")}</span><span class="g-aicon is-red">${lucide("x")}</span><span class="g-aicon is-teal">${lucide("leaf")}</span><span class="g-aicon is-red">${lucide("heart")}</span><span class="g-aicon is-blue">${lucide("zap")}</span><span class="g-themeicon is-violet">${lucide("play")}</span>
  </div>`,

  layout: `<div class="pv-grid">
    <div class="g-cardmock"><span class="g-title">Pro plan</span><span class="g-dimmed">Everything, unlimited.</span><div class="g-row"><span class="g-price">$12<em>/mo</em></span><button class="g-btn g-btn--filled">Upgrade</button></div></div>
    <div class="g-col g-col--gap"><div class="g-alert"><span class="g-alert-bar"></span><div><strong>Stack &amp; Group</strong><span class="g-dimmed">Themed spacing tokens.</span></div></div><div class="g-group"><span class="g-badge is-blue">Paper</span><span class="g-badge is-teal">Card</span><span class="g-badge is-grape">Divider</span></div></div>
  </div>`,

  feedback: `<div class="pv-grid">
    <div class="g-col g-col--gap"><div class="g-alert"><span class="g-alert-bar is-teal"></span><div><strong>Saved</strong><span class="g-dimmed">All changes synced.</span></div></div><div class="g-alert"><span class="g-alert-bar is-red"></span><div><strong>Build failed</strong><span class="g-dimmed">2 errors</span></div></div></div>
    <div class="g-col g-col--gap"><div class="g-progress"><span style="width:42%"></span></div><div class="g-progress"><span class="is-teal" style="width:78%"></span></div><div class="g-group"><span class="g-ring"></span><span class="g-ring is-grape"></span><span class="g-loader"></span></div></div>
  </div>`,

  data: `<div class="pv-grid">
    <div class="g-col"><div class="g-tabs"><span class="is-on">Overview</span><span>Activity</span><span>Members</span></div><div class="g-avatars"><i class="is-blue">WC</i><i class="is-grape">AD</i><i class="is-teal">JR</i><i class="more">+3</i></div></div>
    <div class="g-timeline"><div class="g-tl-item"><i></i><div><strong>Committed</strong><span class="g-dimmed">add WebView</span></div></div><div class="g-tl-item is-last"><i></i><div><strong>Merged</strong><span class="g-dimmed">to main</span></div></div></div>
  </div>`,

  overlays: `<div class="pv-grid">
    <div class="g-menu"><span class="g-menu-h">Actions</span><span class="g-menu-i">Duplicate</span><span class="g-menu-i is-on">Rename</span><span class="g-menu-i is-danger">Delete</span></div>
    <div class="g-modal"><span class="g-title">Delete project?</span><span class="g-dimmed">This cannot be undone.</span><div class="g-group"><button class="g-btn g-btn--subtle">Cancel</button><button class="g-btn g-btn--filled is-red">Delete</button></div></div>
  </div>`,

  navigation: `<div class="pv-wrap">
    <div class="g-stepper"><div class="g-step is-done"><i>&#10003;</i><span>Account</span></div><div class="g-step is-on"><i>2</i><span>Profile</span></div><div class="g-step"><i>3</i><span>Review</span></div></div>
    <div class="g-group"><div class="g-pager"><span>&#8249;</span><span>1</span><span class="is-on">2</span><span>3</span><span>&#8250;</span></div><div class="g-crumbs"><a>Home</a><i>/</i><a>Projects</a><i>/</i><span>guise</span></div></div>
  </div>`,

  editor: `<div class="codepanel" style="width:100%;max-width:520px">
    <div class="codepanel-bar"><i></i><i></i><i></i><span>query.sql &middot; Editor</span></div>
    <pre class="code"><code><span class="t-c">-- &#8984;Enter emits EditorEvent::Run</span>
<span class="t-k">select</span> name, stars
<span class="t-k">from</span> repos
<span class="t-k">where</span> lang = <span class="t-s">'rust'</span>
<span class="t-k">order by</span> stars <span class="t-k">desc</span>;</code></pre>
  </div>`,

  panels: `<div class="pv-grid">
    <div class="g-cardmock"><div class="g-row"><span class="g-title">Results</span><span class="g-badge is-blue">128 rows</span></div><span class="g-dimmed">Card chrome, header actions, footer.</span><div class="g-progress"><span style="width:64%"></span></div></div>
    <div class="g-col g-col--gap"><div class="g-alert"><span class="g-alert-bar"></span><div><strong>Panel</strong><span class="g-dimmed">Controlled collapse, like Modal.</span></div></div><div class="g-alert"><span class="g-alert-bar is-teal"></span><div><strong>SplitPanel</strong><span class="g-dimmed">Draggable divider, Resized events.</span></div></div></div>
  </div>`,

  charts: `<div class="pv-grid">
    <div class="g-col g-col--gap"><svg viewBox="0 0 120 36" width="220" height="66" aria-hidden="true"><polyline fill="none" style="stroke:var(--blue)" stroke-width="2.5" stroke-linecap="round" points="2,28 16,20 30,24 44,11 58,17 72,7 86,13 100,5 118,9"/></svg><div class="g-progress"><span style="width:78%"></span></div><div class="g-progress"><span class="is-teal" style="width:52%"></span></div></div>
    <div class="g-group"><span class="g-ring"></span><span class="g-ring is-grape"></span></div>
  </div>`,

  webview: `<div class="g-webview" style="max-width:460px">
    <div class="g-webbar"><i></i><i></i><i></i><span>https://example.com</span></div>
    <div class="g-webbody"><strong>Native WebView</strong><span>Embedded via wry, positioned in guise layout.</span></div>
  </div>`,

  theming: `<div class="pv-wrap">
    <div class="ribbon ribbon--labelled" aria-hidden="true">
      ${["#2e2e2e","#868e96","#fa5252","#e64980","#be4bdb","#7950f2","#4c6ef5","#228be6","#15aabf","#12b886","#40c057","#82c91e","#fab005","#fd7e14"].map((c) => `<span class="swatch" style="--c:${c}"></span>`).join("")}
    </div>
    <div class="g-group g-badges"><span class="g-badge is-blue">Blue 6</span><span class="g-badge is-violet">Violet 6</span><span class="g-badge is-teal">Teal 6</span></div>
  </div>`,
};

// The styled preview showcase for a doc slug, or "" if none.
export function previewsFor(slug: string): string {
  const stage = stages[slug];
  if (!stage) return "";
  return `<section class="preview">
  <div class="preview-bar"><span class="preview-tag">Preview</span><span class="preview-note">rendered with guise's palette</span></div>
  <div class="preview-stage">${stage}</div>
</section>`;
}
