//! Turning a gpui [`StyleRefinement`] into something that reads like CSS.
//!
//! Safari's Styles sidebar is a list of declarations and its Computed sidebar
//! is a box-model diagram; both want `property: value` pairs, and gpui stores
//! style as a refinement of typed `Option` fields. This module is the
//! translation, kept pure so it can be tested without a window.
//!
//! Only *set* fields are emitted. That is the useful behaviour and also the
//! honest one: a refinement records what a component actually asked for, and
//! inventing defaults for the rest would show style the element does not have.

use gpui::{
  AbsoluteLength, DefiniteLength, Edges, EdgesRefinement, Fill, Hsla, Length, Pixels, SharedString,
  StyleRefinement,
};

/// One `property: value` line in the Styles sidebar.
#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
  pub property: SharedString,
  pub value: SharedString,
  /// Set when the value names a color, so the row can paint a swatch.
  pub color: Option<Hsla>,
}

impl Declaration {
  fn new(property: &'static str, value: impl Into<SharedString>) -> Self {
    Declaration {
      property: SharedString::new_static(property),
      value: value.into(),
      color: None,
    }
  }

  fn colored(property: &'static str, color: Hsla) -> Self {
    Declaration {
      property: SharedString::new_static(property),
      value: hex(color).into(),
      color: Some(color),
    }
  }
}

/// `#rrggbb`, or `#rrggbbaa` when the color is not fully opaque — the notation
/// Safari's color swatches label themselves with.
pub fn hex(color: Hsla) -> String {
  let rgba = gpui::Rgba::from(color);
  let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
  if rgba.a >= 1.0 {
    format!(
      "#{:02x}{:02x}{:02x}",
      byte(rgba.r),
      byte(rgba.g),
      byte(rgba.b)
    )
  } else {
    format!(
      "#{:02x}{:02x}{:02x}{:02x}",
      byte(rgba.r),
      byte(rgba.g),
      byte(rgba.b),
      byte(rgba.a)
    )
  }
}

/// Recover the color behind a [`Fill`].
///
/// `Background::solid` is crate-private in gpui and `Fill::color` hands back
/// the same opaque `Background`, so its `Debug` output is the only public view
/// of the value. Parsing it is unlovely, but it is contained here, it is
/// covered by tests, and the failure mode is a missing swatch rather than a
/// wrong one.
pub fn fill_color(fill: &Fill) -> Option<Hsla> {
  let background = fill.color()?;
  let text = format!("{background:?}");
  if !text.starts_with("Solid(") {
    return None;
  }
  let floats: Vec<f32> = text
    .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
    .filter(|part| !part.is_empty())
    .filter_map(|part| part.parse::<f32>().ok())
    .collect();
  match floats.as_slice() {
    [h, s, l, a, ..] => Some(Hsla {
      h: *h,
      s: *s,
      l: *l,
      a: *a,
    }),
    _ => None,
  }
}

fn absolute(length: AbsoluteLength) -> String {
  match length {
    AbsoluteLength::Pixels(px) => format!("{}px", trim(f32::from(px))),
    AbsoluteLength::Rems(rems) => format!("{}rem", trim(rems.0)),
  }
}

fn definite(length: DefiniteLength) -> String {
  match length {
    DefiniteLength::Absolute(absolute_length) => absolute(absolute_length),
    DefiniteLength::Fraction(fraction) => format!("{}%", trim(fraction * 100.0)),
  }
}

fn length(length: Length) -> String {
  match length {
    Length::Definite(definite_length) => definite(definite_length),
    Length::Auto => "auto".to_string(),
  }
}

/// Drop the trailing `.0` so `12px` does not read as `12.0px`.
fn trim(value: f32) -> String {
  if (value - value.round()).abs() < f32::EPSILON {
    format!("{}", value.round() as i64)
  } else {
    format!("{value:.2}")
      .trim_end_matches('0')
      .trim_end_matches('.')
      .to_string()
  }
}

/// Emit `prefix`, `prefix-top`, … for a set of edges, collapsing to the
/// shorthand when every side agrees — the same rule a CSS formatter uses.
fn edges<T: Copy + PartialEq>(
  out: &mut Vec<Declaration>,
  shorthand: &'static str,
  sides: [(&'static str, Option<T>); 4],
  format: impl Fn(T) -> String,
) {
  let values: Vec<T> = sides.iter().filter_map(|(_, value)| *value).collect();
  if values.len() == 4 && values.windows(2).all(|pair| pair[0] == pair[1]) {
    out.push(Declaration::new(shorthand, format(values[0])));
    return;
  }
  for (property, value) in sides {
    if let Some(value) = value {
      out.push(Declaration::new(property, format(value)));
    }
  }
}

/// Every declaration the refinement actually sets, in CSS authoring order:
/// layout, then box, then flex, then visual, then text.
pub fn declarations(style: &StyleRefinement) -> Vec<Declaration> {
  let mut out = Vec::new();

  if let Some(display) = style.display {
    out.push(Declaration::new(
      "display",
      format!("{display:?}").to_lowercase(),
    ));
  }
  if let Some(position) = style.position {
    out.push(Declaration::new(
      "position",
      format!("{position:?}").to_lowercase(),
    ));
  }
  if let Some(visibility) = style.visibility {
    out.push(Declaration::new(
      "visibility",
      format!("{visibility:?}").to_lowercase(),
    ));
  }
  if let Some(overflow) = style.overflow.x {
    out.push(Declaration::new(
      "overflow-x",
      format!("{overflow:?}").to_lowercase(),
    ));
  }
  if let Some(overflow) = style.overflow.y {
    out.push(Declaration::new(
      "overflow-y",
      format!("{overflow:?}").to_lowercase(),
    ));
  }

  edges(
    &mut out,
    "inset",
    [
      ("top", style.inset.top),
      ("right", style.inset.right),
      ("bottom", style.inset.bottom),
      ("left", style.inset.left),
    ],
    length,
  );

  if let Some(width) = style.size.width {
    out.push(Declaration::new("width", length(width)));
  }
  if let Some(height) = style.size.height {
    out.push(Declaration::new("height", length(height)));
  }
  if let Some(width) = style.min_size.width {
    out.push(Declaration::new("min-width", length(width)));
  }
  if let Some(height) = style.min_size.height {
    out.push(Declaration::new("min-height", length(height)));
  }
  if let Some(width) = style.max_size.width {
    out.push(Declaration::new("max-width", length(width)));
  }
  if let Some(height) = style.max_size.height {
    out.push(Declaration::new("max-height", length(height)));
  }
  if let Some(ratio) = style.aspect_ratio {
    out.push(Declaration::new("aspect-ratio", trim(ratio)));
  }

  edges(
    &mut out,
    "margin",
    [
      ("margin-top", style.margin.top),
      ("margin-right", style.margin.right),
      ("margin-bottom", style.margin.bottom),
      ("margin-left", style.margin.left),
    ],
    length,
  );
  edges(
    &mut out,
    "padding",
    [
      ("padding-top", style.padding.top),
      ("padding-right", style.padding.right),
      ("padding-bottom", style.padding.bottom),
      ("padding-left", style.padding.left),
    ],
    definite,
  );
  edges(
    &mut out,
    "border-width",
    [
      ("border-top-width", style.border_widths.top),
      ("border-right-width", style.border_widths.right),
      ("border-bottom-width", style.border_widths.bottom),
      ("border-left-width", style.border_widths.left),
    ],
    absolute,
  );

  if let Some(direction) = style.flex_direction {
    out.push(Declaration::new(
      "flex-direction",
      match format!("{direction:?}").as_str() {
        "Row" => "row".to_string(),
        "Column" => "column".to_string(),
        "RowReverse" => "row-reverse".to_string(),
        "ColumnReverse" => "column-reverse".to_string(),
        other => other.to_lowercase(),
      },
    ));
  }
  if let Some(wrap) = style.flex_wrap {
    out.push(Declaration::new("flex-wrap", kebab(&format!("{wrap:?}"))));
  }
  if let Some(align) = style.align_items {
    out.push(Declaration::new(
      "align-items",
      kebab(&format!("{align:?}")),
    ));
  }
  if let Some(align) = style.align_self {
    out.push(Declaration::new("align-self", kebab(&format!("{align:?}"))));
  }
  if let Some(align) = style.align_content {
    out.push(Declaration::new(
      "align-content",
      kebab(&format!("{align:?}")),
    ));
  }
  if let Some(justify) = style.justify_content {
    out.push(Declaration::new(
      "justify-content",
      kebab(&format!("{justify:?}")),
    ));
  }
  if let Some(basis) = style.flex_basis {
    out.push(Declaration::new("flex-basis", length(basis)));
  }
  if let Some(grow) = style.flex_grow {
    out.push(Declaration::new("flex-grow", trim(grow)));
  }
  if let Some(shrink) = style.flex_shrink {
    out.push(Declaration::new("flex-shrink", trim(shrink)));
  }
  if let Some(gap) = style.gap.width {
    out.push(Declaration::new("column-gap", definite(gap)));
  }
  if let Some(gap) = style.gap.height {
    out.push(Declaration::new("row-gap", definite(gap)));
  }

  if let Some(fill) = &style.background {
    match fill_color(fill) {
      Some(color) => out.push(Declaration::colored("background-color", color)),
      None => out.push(Declaration::new("background", format!("{fill:?}"))),
    }
  }
  if let Some(color) = style.border_color {
    out.push(Declaration::colored("border-color", color));
  }
  if let Some(style_) = style.border_style {
    out.push(Declaration::new(
      "border-style",
      format!("{style_:?}").to_lowercase(),
    ));
  }

  let radii = [
    ("border-top-left-radius", style.corner_radii.top_left),
    ("border-top-right-radius", style.corner_radii.top_right),
    (
      "border-bottom-right-radius",
      style.corner_radii.bottom_right,
    ),
    ("border-bottom-left-radius", style.corner_radii.bottom_left),
  ];
  edges(&mut out, "border-radius", radii, absolute);

  if let Some(opacity) = style.opacity {
    out.push(Declaration::new("opacity", trim(opacity)));
  }
  if !style
    .box_shadow
    .as_ref()
    .is_none_or(|shadows| shadows.is_empty())
  {
    let count = style.box_shadow.as_ref().map_or(0, |shadows| shadows.len());
    out.push(Declaration::new(
      "box-shadow",
      if count == 1 {
        "1 shadow".to_string()
      } else {
        format!("{count} shadows")
      },
    ));
  }

  {
    let text = &style.text;
    if let Some(color) = text.color {
      out.push(Declaration::colored("color", color));
    }
    if let Some(family) = &text.font_family {
      out.push(Declaration::new("font-family", family.clone()));
    }
    if let Some(size) = text.font_size {
      out.push(Declaration::new("font-size", absolute(size)));
    }
    if let Some(weight) = text.font_weight {
      out.push(Declaration::new("font-weight", trim(weight.0)));
    }
    if let Some(font_style) = text.font_style {
      out.push(Declaration::new(
        "font-style",
        format!("{font_style:?}").to_lowercase(),
      ));
    }
    if let Some(height) = text.line_height {
      out.push(Declaration::new("line-height", definite(height)));
    }
    if let Some(align) = text.text_align {
      out.push(Declaration::new("text-align", kebab(&format!("{align:?}"))));
    }
    if let Some(white_space) = text.white_space {
      out.push(Declaration::new(
        "white-space",
        kebab(&format!("{white_space:?}")),
      ));
    }
    if let Some(color) = text.background_color {
      out.push(Declaration::colored("background-color (text)", color));
    }
  }

  out
}

/// `SpaceBetween` -> `space-between`.
fn kebab(variant: &str) -> String {
  let mut out = String::with_capacity(variant.len() + 4);
  for (index, ch) in variant.chars().enumerate() {
    if ch.is_uppercase() {
      if index > 0 {
        out.push('-');
      }
      out.extend(ch.to_lowercase());
    } else {
      out.push(ch);
    }
  }
  out
}

/// The four nested boxes of the Computed sidebar's diagram. Every value is in
/// pixels, already resolved against the rem size, so the diagram can label
/// itself with numbers rather than units.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BoxModel {
  pub margin: Edges<f32>,
  pub border: Edges<f32>,
  pub padding: Edges<f32>,
  /// The laid-out size, which is the border box gpui measured.
  pub width: f32,
  pub height: f32,
}

impl BoxModel {
  /// The content box: the laid-out size less border and padding.
  pub fn content(&self) -> (f32, f32) {
    let width =
      self.width - self.border.left - self.border.right - self.padding.left - self.padding.right;
    let height =
      self.height - self.border.top - self.border.bottom - self.padding.top - self.padding.bottom;
    (width.max(0.0), height.max(0.0))
  }
}

fn edge_pixels<T: Copy + std::fmt::Debug + Default + PartialEq>(
  edges: &EdgesRefinement<T>,
  resolve: impl Fn(T) -> f32,
) -> Edges<f32> {
  Edges {
    top: edges.top.map(&resolve).unwrap_or(0.0),
    right: edges.right.map(&resolve).unwrap_or(0.0),
    bottom: edges.bottom.map(&resolve).unwrap_or(0.0),
    left: edges.left.map(&resolve).unwrap_or(0.0),
  }
}

/// Resolve the box model for an element, given the bounds it was laid out at.
///
/// `rem_size` is what turns rem-based style into the pixel numbers the diagram
/// prints; percentage values have no parent to resolve against here and read
/// as zero, exactly as Safari shows them when it cannot compute one.
pub fn box_model(style: &StyleRefinement, size: gpui::Size<Pixels>, rem_size: Pixels) -> BoxModel {
  let absolute_px = |value: AbsoluteLength| f32::from(value.to_pixels(rem_size));
  let definite_px = |value: DefiniteLength| match value {
    DefiniteLength::Absolute(absolute) => absolute_px(absolute),
    DefiniteLength::Fraction(_) => 0.0,
  };
  let length_px = |value: Length| match value {
    Length::Definite(definite) => definite_px(definite),
    Length::Auto => 0.0,
  };

  BoxModel {
    margin: edge_pixels(&style.margin, length_px),
    border: edge_pixels(&style.border_widths, absolute_px),
    padding: edge_pixels(&style.padding, definite_px),
    width: f32::from(size.width),
    height: f32::from(size.height),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use gpui::{px, rems, Styled};

  fn style_of(build: impl FnOnce(gpui::Div) -> gpui::Div) -> StyleRefinement {
    let mut div = build(gpui::div());
    div.style().clone()
  }

  fn find<'a>(declarations: &'a [Declaration], property: &str) -> Option<&'a Declaration> {
    declarations
      .iter()
      .find(|d| d.property.as_ref() == property)
  }

  #[test]
  fn only_set_fields_are_emitted() {
    let declarations = declarations(&style_of(|d| d.w(px(120.0))));
    assert_eq!(declarations.len(), 1);
    assert_eq!(declarations[0].property.as_ref(), "width");
    assert_eq!(declarations[0].value.as_ref(), "120px");
  }

  #[test]
  fn uniform_edges_collapse_to_the_shorthand() {
    let declarations = declarations(&style_of(|d| d.p(px(8.0))));
    assert_eq!(
      find(&declarations, "padding").unwrap().value.as_ref(),
      "8px"
    );
    assert!(find(&declarations, "padding-top").is_none());
  }

  #[test]
  fn mixed_edges_stay_long_hand() {
    let declarations = declarations(&style_of(|d| d.pt(px(4.0)).pb(px(10.0))));
    assert!(find(&declarations, "padding").is_none());
    assert_eq!(
      find(&declarations, "padding-top").unwrap().value.as_ref(),
      "4px"
    );
    assert_eq!(
      find(&declarations, "padding-bottom")
        .unwrap()
        .value
        .as_ref(),
      "10px"
    );
  }

  #[test]
  fn rems_keep_their_unit() {
    let declarations = declarations(&style_of(|d| d.w(rems(2.0))));
    assert_eq!(find(&declarations, "width").unwrap().value.as_ref(), "2rem");
  }

  #[test]
  fn fractions_render_as_percentages() {
    let declarations = declarations(&style_of(|d| d.w_1_2()));
    assert_eq!(find(&declarations, "width").unwrap().value.as_ref(), "50%");
  }

  #[test]
  fn variants_are_kebab_cased() {
    let declarations = declarations(&style_of(|d| d.flex().justify_between().items_center()));
    assert_eq!(
      find(&declarations, "justify-content")
        .unwrap()
        .value
        .as_ref(),
      "space-between"
    );
    assert_eq!(
      find(&declarations, "align-items").unwrap().value.as_ref(),
      "center"
    );
    assert_eq!(
      find(&declarations, "display").unwrap().value.as_ref(),
      "flex"
    );
  }

  #[test]
  fn a_background_yields_a_swatch_color() {
    let blue = gpui::hsla(0.6, 0.5, 0.5, 1.0);
    let declarations = declarations(&style_of(|d| d.bg(blue)));
    let background = find(&declarations, "background-color").unwrap();

    let color = background
      .color
      .expect("a solid fill should recover its color");
    assert!((color.h - blue.h).abs() < 0.01);
    assert!((color.s - blue.s).abs() < 0.01);
    assert!((color.l - blue.l).abs() < 0.01);
    assert!((color.a - blue.a).abs() < 0.01);
  }

  #[test]
  fn a_translucent_color_keeps_its_alpha_byte() {
    assert_eq!(hex(gpui::hsla(0.0, 0.0, 0.0, 1.0)), "#000000");
    assert_eq!(hex(gpui::hsla(0.0, 0.0, 1.0, 1.0)), "#ffffff");
    assert_eq!(hex(gpui::hsla(0.0, 0.0, 0.0, 0.5)), "#00000080");
  }

  #[test]
  fn the_box_model_resolves_rems_against_the_rem_size() {
    let style = style_of(|d| d.p(rems(1.0)).border_2().m(px(6.0)));
    let model = box_model(&style, gpui::size(px(100.0), px(50.0)), px(16.0));

    assert_eq!(model.padding.top, 16.0);
    assert_eq!(model.border.left, 2.0);
    assert_eq!(model.margin.bottom, 6.0);
    assert_eq!(model.content(), (100.0 - 32.0 - 4.0, 50.0 - 32.0 - 4.0));
  }

  #[test]
  fn a_content_box_never_goes_negative() {
    let style = style_of(|d| d.p(px(80.0)));
    let model = box_model(&style, gpui::size(px(20.0), px(20.0)), px(16.0));
    assert_eq!(model.content(), (0.0, 0.0));
  }

  #[test]
  fn pixel_values_lose_their_trailing_zero() {
    assert_eq!(trim(12.0), "12");
    assert_eq!(trim(12.5), "12.5");
    assert_eq!(trim(0.25), "0.25");
  }
}
