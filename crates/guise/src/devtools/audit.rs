//! The Audit panel: checks that run against the recorded tree.
//!
//! Safari's Audit tab runs test suites over the page and reports what failed.
//! The interesting difference here is that the rules can be *about `guise`* —
//! contrast, hit-target size, layout that has quietly gone wrong — and they run
//! against what actually rendered this frame rather than against source.
//!
//! Every rule is a pure function over the tree so it can be tested without a
//! window, and every finding points at a node the Elements panel can select.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, Context, Hsla, Pixels, SharedString, Window};

use super::probe::{ProbeNode, ProbeTree};
use super::shell::{empty_state, filter_pill, glyph, section_header, Ink, LABEL_SIZE, MONO_SIZE};
use super::state::SourceRef;
use super::DevTools;
use crate::icon::IconName;
use crate::style::MONO_FAMILY;

/// How much a finding matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
  /// Worth knowing, not worth fixing on its own.
  Info,
  /// Likely wrong.
  Warning,
  /// Wrong, and users will hit it.
  Error,
}

impl Severity {
  fn label(self) -> &'static str {
    match self {
      Severity::Info => "Info",
      Severity::Warning => "Warning",
      Severity::Error => "Error",
    }
  }
}

/// One rule violation, tied to the node that broke it.
#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
  pub severity: Severity,
  /// The rule's stable name, shown as the group heading.
  pub rule: &'static str,
  pub message: SharedString,
  /// The node's key, so clicking the row selects it in Elements.
  pub node: SharedString,
  pub source: Option<SourceRef>,
}

/// WCAG relative luminance.
pub fn luminance(color: Hsla) -> f32 {
  let rgba = gpui::Rgba::from(color);
  let channel = |value: f32| {
    if value <= 0.03928 {
      value / 12.92
    } else {
      ((value + 0.055) / 1.055).powf(2.4)
    }
  };
  0.2126 * channel(rgba.r) + 0.7152 * channel(rgba.g) + 0.0722 * channel(rgba.b)
}

/// WCAG contrast ratio, between 1.0 and 21.0.
pub fn contrast(a: Hsla, b: Hsla) -> f32 {
  let (first, second) = (luminance(a), luminance(b));
  let (lighter, darker) = if first > second {
    (first, second)
  } else {
    (second, first)
  };
  (lighter + 0.05) / (darker + 0.05)
}

/// The smallest square a pointer target should be (WCAG 2.5.8).
const MIN_TARGET: f32 = 24.0;
/// A target this long in its other dimension is easy to hit regardless.
///
/// The bare 24×24 rule fails a full-width list row that happens to be 23px
/// tall, which is not the problem the rule is aimed at — nobody misses a
/// 600px-wide row. Reporting those buries the 16×16 close button that is the
/// actual defect, so a target is only flagged when it is small in *both*
/// directions.
const EASY_TARGET: f32 = 44.0;
/// Text below this is unreadable at a normal viewing distance.
const MIN_TEXT: f32 = 10.0;
/// Nesting past this is a smell rather than a defect, so it reports as info.
const MAX_DEPTH: usize = 24;
/// Slack allowed before a child counts as escaping its parent. Sub-pixel
/// rounding routinely produces a fraction of a pixel of overhang.
const OVERFLOW_SLACK: f32 = 1.0;

/// Run every rule over the tree.
pub fn run(tree: &ProbeTree, rem_size: Pixels) -> Vec<Finding> {
  let mut findings = Vec::new();

  for (index, node) in tree.nodes.iter().enumerate() {
    contrast_rule(node, &mut findings);
    target_rule(node, &mut findings);
    text_rule(node, rem_size, &mut findings);
    collapsed_rule(node, &mut findings);
    overflow_rule(tree, index, node, &mut findings);
    depth_rule(node, &mut findings);
  }

  // Worst first, so the panel opens on what matters.
  findings.sort_by(|a, b| b.severity.cmp(&a.severity).then(a.rule.cmp(b.rule)));
  findings
}

fn finding(severity: Severity, rule: &'static str, message: String, node: &ProbeNode) -> Finding {
  Finding {
    severity,
    rule,
    message: SharedString::from(message),
    node: node.key.clone(),
    source: node.source.clone(),
  }
}

/// Text on a background it cannot be read against.
fn contrast_rule(node: &ProbeNode, out: &mut Vec<Finding>) {
  let Some(style) = node.style.as_ref() else {
    return;
  };
  let Some(text) = style.text.color else {
    return;
  };
  let Some(background) = style
    .background
    .as_ref()
    .and_then(super::styles::fill_color)
  else {
    return;
  };
  // A transparent background is not what the text is actually drawn on, so
  // there is nothing to compare against.
  if background.a < 0.95 {
    return;
  }

  let ratio = contrast(text, background);
  if ratio < 3.0 {
    out.push(finding(
      Severity::Error,
      "Text contrast",
      format!("Contrast {ratio:.1}:1 is below the 4.5:1 minimum"),
      node,
    ));
  } else if ratio < 4.5 {
    out.push(finding(
      Severity::Warning,
      "Text contrast",
      format!("Contrast {ratio:.1}:1 is below the 4.5:1 minimum"),
      node,
    ));
  }
}

/// A control too small to hit reliably.
fn target_rule(node: &ProbeNode, out: &mut Vec<Finding>) {
  // Only elements gpui gave an id are interactive; everything else is layout.
  if node.element_id.is_none() {
    return;
  }
  let width = f32::from(node.bounds.size.width);
  let height = f32::from(node.bounds.size.height);
  if width <= 0.0 || height <= 0.0 {
    return;
  }
  let smaller = width.min(height);
  let larger = width.max(height);
  if smaller < MIN_TARGET && larger < EASY_TARGET {
    out.push(finding(
      Severity::Warning,
      "Hit target size",
      format!("{width:.0}×{height:.0} is under the {MIN_TARGET:.0}×{MIN_TARGET:.0} minimum"),
      node,
    ));
  }
}

/// Text set too small to read.
fn text_rule(node: &ProbeNode, rem_size: Pixels, out: &mut Vec<Finding>) {
  let Some(size) = node.style.as_ref().and_then(|style| style.text.font_size) else {
    return;
  };
  let pixels = f32::from(size.to_pixels(rem_size));
  if pixels < MIN_TEXT {
    out.push(finding(
      Severity::Warning,
      "Text size",
      format!("{pixels:.0}px text is below the {MIN_TEXT:.0}px minimum"),
      node,
    ));
  }
}

/// A container with children but no size — almost always a missing `flex_1`,
/// `w_full`, or `min_h(0)`.
fn collapsed_rule(node: &ProbeNode, out: &mut Vec<Finding>) {
  if node.is_leaf() {
    return;
  }
  let width = f32::from(node.bounds.size.width);
  let height = f32::from(node.bounds.size.height);
  if width <= 0.0 || height <= 0.0 {
    out.push(finding(
      Severity::Error,
      "Collapsed container",
      format!(
        "{} has {} children but lays out at {width:.0}×{height:.0}",
        node.name,
        node.children.len()
      ),
      node,
    ));
  }
}

/// A child painting outside its parent.
fn overflow_rule(tree: &ProbeTree, index: usize, node: &ProbeNode, out: &mut Vec<Finding>) {
  let Some(parent) = node.parent.and_then(|parent| tree.get(parent)) else {
    return;
  };
  // A parent that has not been laid out yet cannot contain anything.
  if f32::from(parent.bounds.size.width) <= 0.0 {
    return;
  }
  let child_right = f32::from(node.bounds.origin.x) + f32::from(node.bounds.size.width);
  let parent_right = f32::from(parent.bounds.origin.x) + f32::from(parent.bounds.size.width);
  let child_bottom = f32::from(node.bounds.origin.y) + f32::from(node.bounds.size.height);
  let parent_bottom = f32::from(parent.bounds.origin.y) + f32::from(parent.bounds.size.height);

  let overhang = (child_right - parent_right)
    .max(child_bottom - parent_bottom)
    .max(f32::from(parent.bounds.origin.x) - f32::from(node.bounds.origin.x))
    .max(f32::from(parent.bounds.origin.y) - f32::from(node.bounds.origin.y));

  if overhang > OVERFLOW_SLACK {
    let _ = index;
    out.push(finding(
      Severity::Warning,
      "Overflow",
      format!(
        "{} extends {overhang:.0}px outside {}",
        node.name, parent.name
      ),
      node,
    ));
  }
}

fn depth_rule(node: &ProbeNode, out: &mut Vec<Finding>) {
  if node.depth > MAX_DEPTH {
    out.push(finding(
      Severity::Info,
      "Nesting depth",
      format!("Nested {} levels deep", node.depth),
      node,
    ));
  }
}

#[derive(Default)]
pub struct AuditPanel {
  /// `None` shows everything; otherwise only this severity.
  severity: Option<Severity>,
}

impl AuditPanel {
  fn color(severity: Severity, ink: &Ink) -> Hsla {
    match severity {
      Severity::Error => ink.danger,
      Severity::Warning => ink.warning,
      Severity::Info => ink.info,
    }
  }

  fn icon(severity: Severity) -> IconName {
    match severity {
      Severity::Error => IconName::CircleX,
      Severity::Warning => IconName::TriangleAlert,
      Severity::Info => IconName::Info,
    }
  }

  pub fn render(
    &self,
    tree: &ProbeTree,
    window: &mut Window,
    cx: &mut Context<DevTools>,
  ) -> AnyElement {
    let ink = Ink::read(cx);
    let all = run(tree, window.rem_size());
    let findings: Vec<&Finding> = all
      .iter()
      .filter(|finding| {
        self
          .severity
          .is_none_or(|wanted| finding.severity == wanted)
      })
      .collect();

    let counts = |severity: Severity| {
      all
        .iter()
        .filter(|finding| finding.severity == severity)
        .count()
    };

    let mut bar = div()
      .flex()
      .flex_none()
      .items_center()
      .gap(px(4.0))
      .h(px(26.0))
      .px(px(8.0))
      .w_full()
      .bg(ink.chrome)
      .border_b_1()
      .border_color(ink.border)
      .child(
        filter_pill("devtools-audit-all", "All", self.severity.is_none(), &ink).on_click(
          cx.listener(|this: &mut DevTools, _event, _window, cx| {
            this.audit.severity = None;
            cx.notify();
          }),
        ),
      );
    for severity in [Severity::Error, Severity::Warning, Severity::Info] {
      bar = bar.child(
        filter_pill(
          ("devtools-audit-severity", severity as usize),
          format!("{} {}", severity.label(), counts(severity)),
          self.severity == Some(severity),
          &ink,
        )
        .on_click(
          cx.listener(move |this: &mut DevTools, _event, _window, cx| {
            this.audit.severity = Some(severity);
            cx.notify();
          }),
        ),
      );
    }
    bar = bar.child(div().flex_1()).child(
      div()
        .text_size(px(LABEL_SIZE))
        .text_color(ink.dim)
        .child(SharedString::from(format!("{} nodes audited", tree.len()))),
    );

    if findings.is_empty() {
      return div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h(px(0.0))
        .w_full()
        .child(bar)
        .child(empty_state(
          if tree.is_empty() {
            "Nothing to audit yet"
          } else {
            "No issues found"
          },
          &ink,
        ))
        .into_any_element();
    }

    let mut list = div()
      .id("devtools-audit-list")
      .flex()
      .flex_col()
      .flex_1()
      .min_h(px(0.0))
      .w_full()
      .overflow_scroll()
      .bg(ink.content);

    let mut rule: Option<&'static str> = None;
    for (position, found) in findings.iter().enumerate() {
      if rule != Some(found.rule) {
        rule = Some(found.rule);
        list = list.child(section_header(found.rule, &ink));
      }

      let color = Self::color(found.severity, &ink);
      let key = found.node.clone();
      let hover_bg = ink.hover;

      list = list.child(
        div()
          .id(("devtools-audit-row", position))
          .flex()
          .items_start()
          .gap(px(6.0))
          .w_full()
          .px(px(8.0))
          .py(px(3.0))
          .border_b_1()
          .border_color(ink.border.opacity(0.4))
          .font_family(MONO_FAMILY)
          .text_size(px(MONO_SIZE))
          .hover(move |st| st.bg(hover_bg))
          .child(glyph(Self::icon(found.severity), 11.0, color, cx))
          .child(
            div()
              .flex_1()
              .min_w(px(0.0))
              .text_color(ink.text)
              .child(found.message.clone()),
          )
          .child(
            div()
              .flex_none()
              .text_color(ink.tag)
              .child(found.node.clone()),
          )
          .on_click(
            cx.listener(move |this: &mut DevTools, _event, _window, cx| {
              let tree = this.tree.clone();
              this.elements.reveal(&tree, &key);
              this.set_tab(super::DevToolsTab::Elements, cx);
            }),
          ),
      );
    }

    div()
      .flex()
      .flex_col()
      .flex_1()
      .min_h(px(0.0))
      .w_full()
      .child(bar)
      .child(list)
      .into_any_element()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::devtools::probe;
  use gpui::{hsla, point, size, Bounds};

  fn tree_with(build: impl FnOnce(&mut ProbeTree)) -> ProbeTree {
    probe::set_enabled(false);
    probe::set_enabled(true);
    probe::test_record("Root", || {});
    probe::begin_frame_unclaimed();
    let mut tree = probe::tree();
    build(&mut tree);
    tree
  }

  fn styled(node: &mut ProbeNode, build: impl FnOnce(gpui::Div) -> gpui::Div) {
    let mut div = build(gpui::div());
    node.style = Some(Box::new(gpui::Styled::style(&mut div).clone()));
  }

  #[test]
  fn black_on_white_is_the_maximum_contrast() {
    let ratio = contrast(hsla(0.0, 0.0, 0.0, 1.0), hsla(0.0, 0.0, 1.0, 1.0));
    assert!((ratio - 21.0).abs() < 0.1, "expected 21:1, got {ratio}");
  }

  #[test]
  fn a_color_against_itself_is_one_to_one() {
    let color = hsla(0.5, 0.5, 0.5, 1.0);
    assert!((contrast(color, color) - 1.0).abs() < 0.001);
  }

  #[test]
  fn contrast_is_symmetric() {
    let a = hsla(0.1, 0.8, 0.3, 1.0);
    let b = hsla(0.6, 0.2, 0.9, 1.0);
    assert!((contrast(a, b) - contrast(b, a)).abs() < 0.001);
  }

  #[test]
  fn low_contrast_text_is_reported() {
    let tree = tree_with(|tree| {
      styled(&mut tree.nodes[0], |d| {
        d.text_color(hsla(0.0, 0.0, 0.75, 1.0))
          .bg(hsla(0.0, 0.0, 1.0, 1.0))
      });
    });

    let findings = run(&tree, px(16.0));
    assert!(findings
      .iter()
      .any(|finding| finding.rule == "Text contrast"));
  }

  #[test]
  fn readable_text_is_not_reported() {
    let tree = tree_with(|tree| {
      styled(&mut tree.nodes[0], |d| {
        d.text_color(hsla(0.0, 0.0, 0.1, 1.0))
          .bg(hsla(0.0, 0.0, 1.0, 1.0))
      });
    });

    assert!(!run(&tree, px(16.0))
      .iter()
      .any(|finding| finding.rule == "Text contrast"));
  }

  #[test]
  fn contrast_is_skipped_over_a_translucent_background() {
    let tree = tree_with(|tree| {
      styled(&mut tree.nodes[0], |d| {
        d.text_color(hsla(0.0, 0.0, 0.75, 1.0))
          .bg(hsla(0.0, 0.0, 1.0, 0.2))
      });
    });

    assert!(!run(&tree, px(16.0))
      .iter()
      .any(|finding| finding.rule == "Text contrast"));
  }

  #[test]
  fn a_small_interactive_element_is_reported() {
    let tree = tree_with(|tree| {
      tree.nodes[0].element_id = Some("close".into());
      tree.nodes[0].bounds = Bounds {
        origin: point(px(0.0), px(0.0)),
        size: size(px(16.0), px(16.0)),
      };
    });

    assert!(run(&tree, px(16.0))
      .iter()
      .any(|finding| finding.rule == "Hit target size"));
  }

  #[test]
  fn a_wide_row_is_hittable_even_when_it_is_short() {
    let tree = tree_with(|tree| {
      tree.nodes[0].element_id = Some("row".into());
      tree.nodes[0].bounds = Bounds {
        origin: point(px(0.0), px(0.0)),
        size: size(px(640.0), px(22.0)),
      };
    });

    assert!(!run(&tree, px(16.0))
      .iter()
      .any(|finding| finding.rule == "Hit target size"));
  }

  #[test]
  fn a_small_element_with_no_id_is_layout_not_a_target() {
    let tree = tree_with(|tree| {
      tree.nodes[0].bounds = Bounds {
        origin: point(px(0.0), px(0.0)),
        size: size(px(4.0), px(4.0)),
      };
    });

    assert!(!run(&tree, px(16.0))
      .iter()
      .any(|finding| finding.rule == "Hit target size"));
  }

  #[test]
  fn a_container_that_collapsed_is_an_error() {
    let tree = tree_with(|tree| {
      let mut child = tree.nodes[0].clone();
      child.key = "Root[0]/Child[0]".into();
      child.parent = Some(0);
      child.depth = 1;
      tree.nodes.push(child);
      tree.nodes[0].children = vec![1];
      tree.nodes[0].bounds = Bounds {
        origin: point(px(0.0), px(0.0)),
        size: size(px(0.0), px(0.0)),
      };
    });

    let findings = run(&tree, px(16.0));
    let collapsed = findings
      .iter()
      .find(|finding| finding.rule == "Collapsed container")
      .expect("a sized-zero parent should report");
    assert_eq!(collapsed.severity, Severity::Error);
  }

  #[test]
  fn a_child_escaping_its_parent_is_reported() {
    let tree = tree_with(|tree| {
      tree.nodes[0].bounds = Bounds {
        origin: point(px(0.0), px(0.0)),
        size: size(px(100.0), px(100.0)),
      };
      let mut child = tree.nodes[0].clone();
      child.key = "Root[0]/Child[0]".into();
      child.parent = Some(0);
      child.depth = 1;
      child.bounds = Bounds {
        origin: point(px(50.0), px(0.0)),
        size: size(px(120.0), px(20.0)),
      };
      tree.nodes.push(child);
      tree.nodes[0].children = vec![1];
    });

    assert!(run(&tree, px(16.0))
      .iter()
      .any(|finding| finding.rule == "Overflow"));
  }

  #[test]
  fn sub_pixel_overhang_is_tolerated() {
    let tree = tree_with(|tree| {
      tree.nodes[0].bounds = Bounds {
        origin: point(px(0.0), px(0.0)),
        size: size(px(100.0), px(100.0)),
      };
      let mut child = tree.nodes[0].clone();
      child.key = "Root[0]/Child[0]".into();
      child.parent = Some(0);
      child.depth = 1;
      child.bounds = Bounds {
        origin: point(px(0.0), px(0.0)),
        size: size(px(100.4), px(100.0)),
      };
      tree.nodes.push(child);
      tree.nodes[0].children = vec![1];
    });

    assert!(!run(&tree, px(16.0))
      .iter()
      .any(|finding| finding.rule == "Overflow"));
  }

  #[test]
  fn findings_come_back_worst_first() {
    let tree = tree_with(|tree| {
      tree.nodes[0].element_id = Some("x".into());
      tree.nodes[0].bounds = Bounds {
        origin: point(px(0.0), px(0.0)),
        size: size(px(10.0), px(10.0)),
      };
      styled(&mut tree.nodes[0], |d| {
        d.text_color(hsla(0.0, 0.0, 0.8, 1.0))
          .bg(hsla(0.0, 0.0, 0.85, 1.0))
      });
    });

    let findings = run(&tree, px(16.0));
    assert!(findings.len() >= 2);
    assert!(findings
      .windows(2)
      .all(|pair| pair[0].severity >= pair[1].severity));
  }

  #[test]
  fn an_empty_tree_produces_no_findings() {
    assert!(run(&ProbeTree::default(), px(16.0)).is_empty());
  }
}
