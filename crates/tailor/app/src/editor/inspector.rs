//! The inspectors.
//!
//! Five tabs, split the way Interface Builder splits them: what the component
//! *is* (Attributes), how big it is and where (Size), how it is painted
//! (Style), what it is wired to (Connections), and what it will be called in
//! the generated file (Identity).
//!
//! Every control in the Attributes tab is built from the catalog rather than
//! written out per component. That is the whole reason the catalog carries a
//! `PropType` — a hundred components would otherwise be a hundred inspectors.

use std::sync::Arc;

use gpui::prelude::*;
use gpui::{div, px, AnyElement, Context, ElementId, Entity, SharedString, Window};
use guise::prelude::*;
use tailor_model::catalog;
use tailor_model::motion::MotionProps;
use tailor_model::props::{PropSpec, PropType, PropValue};
use tailor_model::style::{Dimension, Direction, LayoutMode, Overflow, ShadowToken, TextAlign};
use tailor_model::tokens::{ColorSpec, ColorToken, EaseToken, EnterToken, LoopToken};
use tailor_model::{
  ActionDef, AlignToken, DocKind, Flavor, JustifyToken, NodeId, Scheme, SizeToken, StateVar,
  VarType, VariantToken,
};
use tailor_store::Panel;

use super::{icon, Inspector, Workbench};
use crate::theme;

impl Workbench {
  pub(super) fn render_inspector(
    &mut self,
    _window: &mut Window,
    cx: &mut Context<Self>,
  ) -> impl IntoElement {
    let chrome = theme::colors(cx);
    let width = self.settings.size(Panel::Inspector);
    let selected = self.selection.first().copied();
    let tab = self.inspector;

    let body = match selected {
      Some(id) => match tab {
        Inspector::Attributes => self.render_attributes(id, cx),
        Inspector::Size => self.render_size(id, cx),
        Inspector::Style => self.render_style(id, cx),
        Inspector::Motion => self.render_motion(id, cx),
        Inspector::Connections => self.render_connections(id, cx),
        Inspector::Identity => self.render_identity(id, cx),
      },
      None => self.render_document_inspector(cx),
    };

    div()
      .w(px(width))
      .flex_none()
      .h_full()
      .flex()
      .flex_col()
      .bg(chrome.surface)
      .child(
        div()
          .flex()
          .items_center()
          .gap(px(1.))
          .h(px(32.))
          .px(px(6.))
          .border_b(px(1.))
          .border_color(chrome.border)
          .children(Inspector::ALL.iter().map(|option| {
            let option = *option;
            let active = option == tab;
            div()
              .id(ElementId::Name(SharedString::from(format!(
                "insp-{}",
                option.label()
              ))))
              .flex()
              .items_center()
              .justify_center()
              .flex_grow()
              .py(px(5.))
              .min_w(px(30.))
              .rounded(px(5.))
              .when(active, |d| d.bg(chrome.raised).text_color(chrome.text))
              .when(!active, |d| d.text_color(chrome.dimmed))
              .child(icon(option.icon()))
              .tooltip(tooltip(option.label()))
              .on_click(cx.listener(move |this, _, _window, cx| {
                this.set_inspector(option, cx);
              }))
          }))
          .child(
            div()
              .id("fold-inspector")
              .flex()
              .flex_none()
              .items_center()
              .justify_center()
              .size(px(22.))
              .ml(px(2.))
              .rounded(px(5.))
              .text_color(chrome.dimmed)
              .hover(move |style| style.bg(chrome.raised).text_color(chrome.text))
              .child(icon("chevrons-right"))
              .tooltip(tooltip("Collapse"))
              .on_click(cx.listener(|this, _, _window, cx| {
                this.toggle_panel(Panel::Inspector, cx);
              })),
          ),
      )
      .child(
        div()
          .id("inspector-body")
          .flex()
          .flex_col()
          .flex_grow()
          .overflow_y_scroll()
          .p(px(10.))
          .gap(px(10.))
          .child(body),
      )
  }

  // --- attributes -------------------------------------------------------

  fn render_attributes(&mut self, id: NodeId, cx: &mut Context<Self>) -> AnyElement {
    let Some(node) = self.doc().and_then(|doc| doc.node(id)).cloned() else {
      return empty("This node is gone", cx);
    };
    if let Some(name) = node.component_ref() {
      return self.section(
        "component",
        "Component",
        vec![note(
          format!("{name} is one of your components. Open its tab to change it."),
          cx,
        )],
        cx,
      );
    }
    let Some(spec) = catalog::get(&node.kind) else {
      return empty("Tailor does not know this component", cx);
    };
    if spec.props.is_empty() {
      return self.section(
        "attributes",
        "Attributes",
        vec![note("Nothing to set here.", cx)],
        cx,
      );
    }

    let rows: Vec<AnyElement> = spec
      .props
      .iter()
      .map(|prop| self.prop_row(id, prop, cx))
      .collect();
    self.section("attributes", spec.title, rows, cx)
  }

  fn prop_row(
    &mut self,
    id: NodeId,
    prop: &'static PropSpec,
    cx: &mut Context<Self>,
  ) -> AnyElement {
    let chrome = theme::colors(cx);
    let value = self
      .doc()
      .and_then(|doc| doc.node(id))
      .and_then(|node| node.prop(prop.key).cloned())
      .unwrap_or_else(|| prop.default_value());
    let bound = value.as_binding().map(|name| name.to_string());

    let control = match bound.clone() {
      Some(name) => self.binding_pill(id, prop, name, cx),
      None => self.prop_control(id, prop, &value, cx),
    };

    div()
      .flex()
      .flex_col()
      .gap(px(3.))
      .child(
        div()
          .flex()
          .items_center()
          .justify_between()
          .child(
            div()
              .text_size(px(11.))
              .text_color(chrome.dimmed)
              .child(SharedString::from(prop.label)),
          )
          .when(bindable(prop), |d| {
            d.child(self.bind_menu(id, prop, bound.is_some(), cx))
          }),
      )
      .child(control)
      .when(!prop.hint.is_empty(), |d| {
        d.child(
          div()
            .text_size(px(10.))
            .text_color(chrome.dimmed)
            .child(SharedString::from(prop.hint)),
        )
      })
      .into_any_element()
  }

  fn prop_control(
    &mut self,
    id: NodeId,
    prop: &'static PropSpec,
    value: &PropValue,
    cx: &mut Context<Self>,
  ) -> AnyElement {
    let key = prop.key;
    match prop.ty {
      PropType::Bool => {
        let on = value.as_bool().unwrap_or(false);
        Switch::new(ElementId::Name(SharedString::from(format!("p-{id}-{key}"))))
          .checked(on)
          .size(Size::Sm)
          .on_change(cx.listener(move |this, _: &gpui::ClickEvent, _window, cx| {
            this.set_prop(id, key, PropValue::Bool(!on), cx);
          }))
          .into_any_element()
      }
      PropType::Text | PropType::MultilineText | PropType::Int | PropType::Float => {
        let initial = match value {
          PropValue::Int(v) => v.to_string(),
          PropValue::Float(v) => trim_float(*v),
          other => other.as_str().unwrap_or("").to_string(),
        };
        let ty = prop.ty;
        let field = self.field(format!("{id}/{key}"), initial, cx, move |this, text, cx| {
          let parsed = match ty {
            PropType::Int => PropValue::Int(text.trim().parse().unwrap_or_default()),
            PropType::Float => PropValue::Float(number(&text)),
            _ => PropValue::Text(text),
          };
          this.set_prop(id, key, parsed, cx);
        });
        field.into_any_element()
      }
      PropType::Choice => {
        let current = value.as_str().unwrap_or("").to_string();
        chip_row(
          prop
            .choices
            .iter()
            .map(|choice| (choice.to_string(), choice.to_string())),
          current,
          cx,
          move |this, choice, cx| {
            this.set_prop(id, key, PropValue::Choice(choice), cx);
          },
        )
      }
      PropType::Size => {
        let current = value.as_size().unwrap_or(SizeToken::Md);
        chip_row(
          SizeToken::ALL
            .iter()
            .map(|token| (token.label().to_string(), token.label().to_string())),
          current.label().to_string(),
          cx,
          move |this, label, cx| {
            if let Some(token) = SizeToken::parse(&label) {
              this.set_prop(id, key, PropValue::Size(token), cx);
            }
          },
        )
      }
      PropType::Variant => {
        let current = value.as_variant().unwrap_or(VariantToken::Filled);
        chip_row(
          VariantToken::ALL
            .iter()
            .map(|token| (token.label().to_string(), token.label().to_string())),
          current.label().to_string(),
          cx,
          move |this, label, cx| {
            if let Some(token) = VariantToken::parse(&label) {
              this.set_prop(id, key, PropValue::Variant(token), cx);
            }
          },
        )
      }
      PropType::Color => {
        let current = value.as_color().cloned().unwrap_or_default();
        let allow_custom = prop.rust_enum != "ColorName";
        self.color_control(id, key, current, allow_custom, cx)
      }
      PropType::Icon => {
        let current = value.as_str().unwrap_or("").to_string();
        self.icon_control(id, key, current, cx)
      }
      PropType::Items => {
        let items = value.as_items().map(|v| v.join("\n")).unwrap_or_default();
        let field = self.area(format!("{id}/{key}"), items, cx, move |this, text, cx| {
          let values: Vec<String> = text
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();
          this.set_prop(id, key, PropValue::Items(values), cx);
        });
        field.into_any_element()
      }
      PropType::Numbers => {
        let numbers = value
          .as_numbers()
          .map(|v| {
            v.iter()
              .map(|n| trim_float(*n))
              .collect::<Vec<_>>()
              .join(", ")
          })
          .unwrap_or_default();
        let field = self.field(format!("{id}/{key}"), numbers, cx, move |this, text, cx| {
          let values: Vec<f64> = text
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|part| !part.is_empty())
            .filter_map(|part| part.parse::<f64>().ok())
            .filter(|value| value.is_finite())
            .collect();
          this.set_prop(id, key, PropValue::Numbers(values), cx);
        });
        field.into_any_element()
      }
    }
  }

  fn color_control(
    &mut self,
    id: NodeId,
    key: &'static str,
    current: ColorSpec,
    allow_custom: bool,
    cx: &mut Context<Self>,
  ) -> AnyElement {
    let chrome = theme::colors(cx);
    let selected = match &current {
      ColorSpec::Named(token) => Some(*token),
      ColorSpec::Custom(_) => None,
    };
    let swatches = div()
      .flex()
      .flex_wrap()
      .gap(px(4.))
      .children(ColorToken::ALL.iter().map(|token| {
        let token = *token;
        let fill = theme(cx).color(theme::color_of(token), 6).hsla();
        let active = selected == Some(token);
        div()
          .id(ElementId::Name(SharedString::from(format!(
            "sw-{id}-{key}-{}",
            token.label()
          ))))
          .size(px(18.))
          .rounded(px(4.))
          .bg(fill)
          .when(active, |d| d.border(px(2.)).border_color(chrome.text))
          .tooltip(tooltip(token.label()))
          .on_click(cx.listener(move |this, _, _window, cx| {
            this.set_prop(id, key, PropValue::Color(ColorSpec::Named(token)), cx);
          }))
      }));

    if !allow_custom {
      return swatches.into_any_element();
    }
    let hex = match &current {
      ColorSpec::Custom(hex) => hex.clone(),
      ColorSpec::Named(_) => String::new(),
    };
    let field = self.field(format!("{id}/{key}/hex"), hex, cx, move |this, text, cx| {
      let text = text.trim().to_string();
      if text.is_empty() {
        return;
      }
      this.set_prop(id, key, PropValue::Color(ColorSpec::Custom(text)), cx);
    });
    div()
      .flex()
      .flex_col()
      .gap(px(5.))
      .child(swatches)
      .child(field)
      .into_any_element()
  }

  /// The icon picker: a search field over all 1991 Lucide names, showing the
  /// first few dozen matches. A full grid would be a scroll view of noise.
  fn icon_control(
    &mut self,
    id: NodeId,
    key: &'static str,
    current: String,
    cx: &mut Context<Self>,
  ) -> AnyElement {
    let chrome = theme::colors(cx);
    let field_key = format!("{id}/{key}");
    let query = self
      .fields
      .get(&field_key)
      .map(|field| field.read(cx).text())
      .unwrap_or_else(|| current.clone());
    let field = self.field(field_key, current.clone(), cx, move |this, text, cx| {
      let name = text.trim().to_string();
      this.set_prop(id, key, PropValue::Icon(name), cx);
    });

    let needle = query.trim().to_lowercase();
    let matches: Vec<IconName> = IconName::all()
      .iter()
      .copied()
      .filter(|name| needle.is_empty() || name.name().contains(&needle))
      .take(48)
      .collect();

    div()
      .flex()
      .flex_col()
      .gap(px(6.))
      .child(field)
      .child(
        div()
          .flex()
          .flex_wrap()
          .gap(px(2.))
          .children(matches.into_iter().map(|name| {
            let label = name.name();
            div()
              .id(ElementId::Name(SharedString::from(format!(
                "ic-{id}-{key}-{label}"
              ))))
              .flex()
              .items_center()
              .justify_center()
              .size(px(24.))
              .rounded(px(4.))
              .text_color(if label == current {
                chrome.accent
              } else {
                chrome.dimmed
              })
              .hover(move |style| style.bg(chrome.raised))
              .child(Icon::new(name).size(Size::Sm))
              .tooltip(tooltip(label))
              .on_click(cx.listener(move |this, _, _window, cx| {
                this.fields.remove(&format!("{id}/{key}"));
                this.set_prop(id, key, PropValue::Icon(label.to_string()), cx);
              }))
          })),
      )
      .into_any_element()
  }

  /// The pill shown in place of a control when a prop reads a variable.
  fn binding_pill(
    &mut self,
    id: NodeId,
    prop: &'static PropSpec,
    name: String,
    cx: &mut Context<Self>,
  ) -> AnyElement {
    let chrome = theme::colors(cx);
    let key = prop.key;
    let exists = self
      .doc()
      .map(|doc| doc.var(&name).is_some())
      .unwrap_or(false);
    div()
      .flex()
      .items_center()
      .gap(px(6.))
      .px(px(8.))
      .py(px(5.))
      .rounded(px(6.))
      .bg(chrome.raised)
      .border(px(1.))
      .border_color(if exists { chrome.accent } else { chrome.danger })
      .child(div().text_color(chrome.accent).child(icon("link")))
      .child(
        div()
          .flex_grow()
          .text_size(px(12.))
          .child(SharedString::from(name)),
      )
      .child(
        div()
          .id(ElementId::Name(SharedString::from(format!(
            "unbind-{id}-{key}"
          ))))
          .text_color(chrome.dimmed)
          .child(icon("unlink"))
          .on_click(cx.listener(move |this, _, _window, cx| {
            let spec = catalog::get(
              &this
                .doc()
                .and_then(|d| d.node(id))
                .map(|n| n.kind.clone())
                .unwrap_or_default(),
            );
            let fallback = spec
              .and_then(|spec| spec.default_prop(key))
              .unwrap_or(PropValue::Text(String::new()));
            this.set_prop(id, key, fallback, cx);
          })),
      )
      .into_any_element()
  }

  /// The little chain button that binds a prop to a state variable.
  fn bind_menu(
    &mut self,
    id: NodeId,
    prop: &'static PropSpec,
    bound: bool,
    cx: &mut Context<Self>,
  ) -> AnyElement {
    let chrome = theme::colors(cx);
    let key = prop.key;
    let vars: Vec<String> = self
      .doc()
      .map(|doc| doc.state.iter().map(|v| v.name.clone()).collect())
      .unwrap_or_default();
    if vars.is_empty() || bound {
      return div().into_any_element();
    }
    div()
      .flex()
      .items_center()
      .gap(px(3.))
      .children(vars.into_iter().map(move |name| {
        let for_click = name.clone();
        let tip = format!("Read this from {name}");
        div()
          .id(ElementId::Name(SharedString::from(format!(
            "bind-{id}-{key}-{name}"
          ))))
          .flex()
          .items_center()
          .gap(px(3.))
          .px(px(5.))
          .py(px(1.))
          .rounded(px(4.))
          .bg(chrome.raised)
          .text_size(px(10.))
          .text_color(chrome.dimmed)
          .hover(move |style| style.text_color(chrome.accent))
          .child(icon("link"))
          .child(SharedString::from(name.clone()))
          .tooltip(tooltip(tip))
          .on_click(cx.listener(move |this, _, _window, cx| {
            this.set_prop(id, key, PropValue::Binding(for_click.clone()), cx);
          }))
      }))
      .into_any_element()
  }

  // --- size -------------------------------------------------------------

  fn render_size(&mut self, id: NodeId, cx: &mut Context<Self>) -> AnyElement {
    let Some(node) = self.doc().and_then(|doc| doc.node(id)).cloned() else {
      return empty("This node is gone", cx);
    };
    let style = node.style.clone();
    let is_container = catalog::get(&node.kind)
      .map(|s| s.takes_children())
      .unwrap_or(false);
    let parent_absolute = self
      .doc()
      .and_then(|doc| doc.parent_of(id))
      .map(|(parent, _, _)| {
        self.doc().map(|doc| doc.layout_of(parent)) == Some(LayoutMode::Absolute)
      })
      .unwrap_or(false);

    let mut blocks = Vec::new();

    if is_container {
      blocks.push(labelled(
        "Children",
        chip_row(
          LayoutMode::ALL
            .iter()
            .map(|m| (m.title().to_string(), m.label().to_string())),
          style.layout.label().to_string(),
          cx,
          move |this, label, cx| {
            let mode = if label == "absolute" {
              LayoutMode::Absolute
            } else {
              LayoutMode::Flow
            };
            this.edit_style(id, "Layout", cx, move |style| style.layout = mode);
          },
        ),
        cx,
      ));
      if style.layout == LayoutMode::Flow {
        blocks.push(labelled(
          "Direction",
          chip_row(
            Direction::ALL
              .iter()
              .map(|d| (d.label().to_string(), d.label().to_string())),
            style.direction.label().to_string(),
            cx,
            move |this, label, cx| {
              let direction = if label == "row" {
                Direction::Row
              } else {
                Direction::Column
              };
              this.edit_style(id, "Direction", cx, move |style| {
                style.direction = direction
              });
            },
          ),
          cx,
        ));
        blocks.push(self.number_row(
          id,
          "Gap",
          "gap",
          style.gap.unwrap_or(0.0),
          cx,
          |style, value| style.gap = (value > 0.0).then_some(value),
        ));
        blocks.push(labelled(
          "Align",
          chip_row(
            AlignToken::ALL
              .iter()
              .map(|a| (a.label().to_string(), a.label().to_string())),
            style
              .align
              .map(|a| a.label().to_string())
              .unwrap_or_default(),
            cx,
            move |this, label, cx| {
              let align = AlignToken::parse(&label);
              this.edit_style(id, "Align", cx, move |style| style.align = align);
            },
          ),
          cx,
        ));
        blocks.push(labelled(
          "Justify",
          chip_row(
            JustifyToken::ALL
              .iter()
              .map(|j| (j.label().to_string(), j.label().to_string())),
            style
              .justify
              .map(|j| j.label().to_string())
              .unwrap_or_default(),
            cx,
            move |this, label, cx| {
              let justify = JustifyToken::parse(&label);
              this.edit_style(id, "Justify", cx, move |style| style.justify = justify);
            },
          ),
          cx,
        ));
        blocks.push(self.switch_row(id, "Wrap", style.wrap, cx, |style, on| style.wrap = on));
      }
    }

    if parent_absolute {
      blocks.push(self.number_row(id, "X", "x", style.x, cx, |style, value| style.x = value));
      blocks.push(self.number_row(id, "Y", "y", style.y, cx, |style, value| style.y = value));
    }

    blocks.push(self.dimension_row(id, "Width", true, style.width, cx));
    blocks.push(self.dimension_row(id, "Height", false, style.height, cx));
    blocks.push(self.optional_number_row(
      id,
      "Min width",
      "minw",
      style.min_width,
      cx,
      |style, v| style.min_width = v,
    ));
    blocks.push(self.optional_number_row(
      id,
      "Max width",
      "maxw",
      style.max_width,
      cx,
      |style, v| style.max_width = v,
    ));
    blocks.push(self.optional_number_row(
      id,
      "Min height",
      "minh",
      style.min_height,
      cx,
      |style, v| style.min_height = v,
    ));
    blocks.push(self.optional_number_row(
      id,
      "Max height",
      "maxh",
      style.max_height,
      cx,
      |style, v| style.max_height = v,
    ));

    let padding = style.padding;
    blocks.push(self.edges_row(id, "Padding", "pad", padding, cx, |style| {
      &mut style.padding
    }));
    let margin = style.margin;
    blocks.push(self.edges_row(id, "Margin", "mar", margin, cx, |style| &mut style.margin));

    blocks.push(labelled(
      "Overflow",
      chip_row(
        Overflow::ALL
          .iter()
          .map(|o| (o.label().to_string(), o.label().to_string())),
        style.overflow.label().to_string(),
        cx,
        move |this, label, cx| {
          let overflow = Overflow::ALL
            .iter()
            .copied()
            .find(|o| o.label() == label)
            .unwrap_or(Overflow::Visible);
          this.edit_style(id, "Overflow", cx, move |style| style.overflow = overflow);
        },
      ),
      cx,
    ));

    self.section("layout", "Layout", blocks, cx)
  }

  fn dimension_row(
    &mut self,
    id: NodeId,
    label: &'static str,
    horizontal: bool,
    current: Dimension,
    cx: &mut Context<Self>,
  ) -> AnyElement {
    let options = ["auto", "px", "full", "grow"];
    let selected = current.label().to_string();
    let chips = chip_row(
      options.iter().map(|o| (o.to_string(), o.to_string())),
      selected,
      cx,
      move |this, choice, cx| {
        let next = match choice.as_str() {
          "px" => Dimension::Px(120.0),
          "full" => Dimension::Full,
          "grow" => Dimension::Grow(1.0),
          _ => Dimension::Auto,
        };
        this.edit_style(id, label, cx, move |style| {
          if horizontal {
            style.width = next;
          } else {
            style.height = next;
          }
        });
      },
    );
    // Two fn pointers rather than one closure over `horizontal`: the
    // handler is copied into a `'static` subscription, and a fn pointer is
    // the only thing here that is `Copy`.
    fn set_width(style: &mut tailor_model::style::StyleProps, value: f32) {
      style.width = Dimension::Px(value);
    }
    fn set_height(style: &mut tailor_model::style::StyleProps, value: f32) {
      style.height = Dimension::Px(value);
    }
    let extra = match current {
      Dimension::Px(value) => Some(self.number_row(
        id,
        "",
        if horizontal { "wpx" } else { "hpx" },
        value,
        cx,
        if horizontal { set_width } else { set_height },
      )),
      _ => None,
    };
    div()
      .flex()
      .flex_col()
      .gap(px(4.))
      .child(labelled(label, chips, cx))
      .children(extra)
      .into_any_element()
  }

  fn number_row(
    &mut self,
    id: NodeId,
    label: &'static str,
    key: &'static str,
    value: f32,
    cx: &mut Context<Self>,
    apply: fn(&mut tailor_model::style::StyleProps, f32),
  ) -> AnyElement {
    let field = self.field(
      format!("{id}/style/{key}"),
      trim_float(value as f64),
      cx,
      move |this, text, cx| {
        let parsed = number(&text) as f32;
        this.edit_style(id, "Size", cx, move |style| apply(style, parsed));
      },
    );
    if label.is_empty() {
      field.into_any_element()
    } else {
      labelled(label, field.into_any_element(), cx)
    }
  }

  fn optional_number_row(
    &mut self,
    id: NodeId,
    label: &'static str,
    key: &'static str,
    value: Option<f32>,
    cx: &mut Context<Self>,
    apply: fn(&mut tailor_model::style::StyleProps, Option<f32>),
  ) -> AnyElement {
    let text = value.map(|v| trim_float(v as f64)).unwrap_or_default();
    let field = self.field(
      format!("{id}/style/{key}"),
      text,
      cx,
      move |this, text, cx| {
        let parsed = text
          .trim()
          .parse::<f32>()
          .ok()
          .filter(|v| v.is_finite() && *v > 0.0);
        this.edit_style(id, "Size", cx, move |style| apply(style, parsed));
      },
    );
    labelled(label, field.into_any_element(), cx)
  }

  fn switch_row(
    &mut self,
    id: NodeId,
    label: &'static str,
    on: bool,
    cx: &mut Context<Self>,
    apply: fn(&mut tailor_model::style::StyleProps, bool),
  ) -> AnyElement {
    let control = Switch::new(ElementId::Name(SharedString::from(format!(
      "sw-{id}-{label}"
    ))))
    .checked(on)
    .size(Size::Sm)
    .on_change(cx.listener(move |this, _: &gpui::ClickEvent, _window, cx| {
      this.edit_style(id, label, cx, move |style| apply(style, !on));
    }))
    .into_any_element();
    labelled(label, control, cx)
  }

  /// Four sides in one row, the way every design tool does it.
  fn edges_row(
    &mut self,
    id: NodeId,
    label: &'static str,
    key: &'static str,
    edges: tailor_model::Edges,
    cx: &mut Context<Self>,
    pick: fn(&mut tailor_model::style::StyleProps) -> &mut tailor_model::Edges,
  ) -> AnyElement {
    // `pick` stays a plain fn pointer: it is copied into four closures.
    let sides: [(&'static str, f32, usize); 4] = [
      ("T", edges.top, 0),
      ("R", edges.right, 1),
      ("B", edges.bottom, 2),
      ("L", edges.left, 3),
    ];
    let chrome = theme::colors(cx);
    let mut row = div().flex().gap(px(4.));
    for (side, value, index) in sides {
      let field = self.field(
        format!("{id}/style/{key}/{side}"),
        trim_float(value as f64),
        cx,
        move |this, text, cx| {
          let parsed = number(&text) as f32;
          this.edit_style(id, label, cx, move |style| {
            let edges = pick(style);
            match index {
              0 => edges.top = parsed,
              1 => edges.right = parsed,
              2 => edges.bottom = parsed,
              _ => edges.left = parsed,
            }
          });
        },
      );
      row = row.child(
        div()
          .flex()
          .flex_col()
          .gap(px(2.))
          .flex_grow()
          .child(
            div()
              .text_size(px(9.))
              .text_color(chrome.dimmed)
              .child(side),
          )
          .child(field),
      );
    }
    labelled(label, row.into_any_element(), cx)
  }

  // --- style -------------------------------------------------------------

  fn render_style(&mut self, id: NodeId, cx: &mut Context<Self>) -> AnyElement {
    let Some(node) = self.doc().and_then(|doc| doc.node(id)).cloned() else {
      return empty("This node is gone", cx);
    };
    let style = node.style.clone();
    let mut blocks = Vec::new();

    blocks.push(self.style_color_row(
      id,
      "Fill",
      "bg",
      style.background.clone(),
      cx,
      |style, color| style.background = color,
    ));
    blocks.push(self.number_row(
      id,
      "Border",
      "bw",
      style.border_width,
      cx,
      |style, value| style.border_width = value.max(0.0),
    ));
    blocks.push(self.style_color_row(
      id,
      "Border color",
      "bc",
      style.border_color.clone(),
      cx,
      |style, color| style.border_color = color,
    ));
    blocks.push(
      self.number_row(id, "Radius", "rad", style.radius, cx, |style, value| {
        style.radius = value.max(0.0)
      }),
    );
    blocks.push(labelled(
      "Shadow",
      chip_row(
        ShadowToken::ALL
          .iter()
          .map(|s| (s.label().to_string(), s.label().to_string())),
        style.shadow.label().to_string(),
        cx,
        move |this, label, cx| {
          let shadow = ShadowToken::ALL
            .iter()
            .copied()
            .find(|s| s.label() == label)
            .unwrap_or(ShadowToken::None);
          this.edit_style(id, "Shadow", cx, move |style| style.shadow = shadow);
        },
      ),
      cx,
    ));
    blocks.push(
      self.number_row(id, "Opacity", "op", style.opacity, cx, |style, value| {
        style.opacity = value.clamp(0.0, 1.0)
      }),
    );
    blocks.push(self.style_color_row(
      id,
      "Text color",
      "fg",
      style.text_color.clone(),
      cx,
      |style, color| style.text_color = color,
    ));
    blocks.push(self.optional_number_row(
      id,
      "Font size",
      "fs",
      style.font_size,
      cx,
      |style, v| style.font_size = v,
    ));
    blocks.push(labelled(
      "Text align",
      chip_row(
        TextAlign::ALL
          .iter()
          .map(|a| (a.label().to_string(), a.label().to_string())),
        style
          .text_align
          .map(|a| a.label().to_string())
          .unwrap_or_default(),
        cx,
        move |this, label, cx| {
          let align = TextAlign::ALL.iter().copied().find(|a| a.label() == label);
          this.edit_style(id, "Text align", cx, move |style| style.text_align = align);
        },
      ),
      cx,
    ));
    blocks.push(
      self.switch_row(id, "Italic", style.italic, cx, |style, on| {
        style.italic = on
      }),
    );

    self.section("style", "Style", blocks, cx)
  }

  // --- motion -----------------------------------------------------------

  fn render_motion(&mut self, id: NodeId, cx: &mut Context<Self>) -> AnyElement {
    let Some(node) = self.doc().and_then(|doc| doc.node(id)).cloned() else {
      return empty("This node is gone", cx);
    };
    let motion = node.motion;
    let has_children = !node.all_children().is_empty();
    let chrome = theme::colors(cx);

    // "None" is a chip like any other rather than a switch: it is the
    // default, and turning an animation off should be the same gesture as
    // choosing a different one.
    let mut options = vec![("None".to_string(), String::new())];
    options.extend(
      EnterToken::ALL
        .iter()
        .map(|kind| (kind.title().to_string(), kind.label().to_string())),
    );
    let entrance = labelled(
      "Entrance",
      chip_row(
        options,
        motion
          .enter
          .map(|kind| kind.label().to_string())
          .unwrap_or_default(),
        cx,
        move |this, label, cx| {
          let enter = EnterToken::parse(&label);
          this.edit_motion(id, "Entrance", cx, move |motion| motion.enter = enter);
        },
      ),
      cx,
    );

    let Some(kind) = motion.enter else {
      return self.section(
        "motion",
        "Motion",
        vec![
          entrance,
          note(
            "Pick an entrance and this node animates in — on the canvas, \
                         in the live window, and in the generated code.",
            cx,
          ),
        ],
        cx,
      );
    };

    let mut blocks = vec![entrance];

    blocks.push(labelled(
      "Easing",
      chip_row(
        EaseToken::ALL
          .iter()
          .map(|ease| (ease.title().to_string(), ease.label().to_string())),
        motion.ease.label().to_string(),
        cx,
        move |this, label, cx| {
          let ease = EaseToken::parse(&label).unwrap_or_default();
          this.edit_motion(id, "Easing", cx, move |motion| motion.ease = ease);
        },
      ),
      cx,
    ));

    blocks.push(self.motion_number(
      id,
      "Duration (ms)",
      "dur",
      motion.duration,
      cx,
      |motion, value| motion.duration = value.max(0.0),
    ));
    blocks.push(self.motion_number(
      id,
      "Delay (ms)",
      "delay",
      motion.delay,
      cx,
      |motion, value| motion.delay = value.max(0.0),
    ));

    if kind.travels() {
      blocks.push(self.motion_number(
        id,
        "Distance (px)",
        "dist",
        motion.distance,
        cx,
        |motion, value| motion.distance = value,
      ));
    }

    if has_children {
      blocks.push(self.motion_number(
        id,
        "Stagger children (ms)",
        "stagger",
        motion.stagger,
        cx,
        |motion, value| motion.stagger = value.max(0.0),
      ));
      if motion.staggers() {
        blocks.push(note(
          "This node hands its entrance to each child, one delay per \
                     index — it does not animate itself. A child with its own \
                     entrance keeps it.",
          cx,
        ));
      }
    }

    blocks.push(labelled(
      "Repeat",
      chip_row(
        LoopToken::ALL
          .iter()
          .map(|mode| (mode.title().to_string(), mode.label().to_string())),
        motion.repeat.label().to_string(),
        cx,
        move |this, label, cx| {
          let repeat = LoopToken::parse(&label).unwrap_or_default();
          this.edit_motion(id, "Repeat", cx, move |motion| motion.repeat = repeat);
        },
      ),
      cx,
    ));

    if motion.repeat == LoopToken::Forever {
      let alternate = motion.alternate;
      blocks.push(labelled(
        "Alternate",
        Switch::new(ElementId::Name(SharedString::from(format!(
          "motion-alt-{id}"
        ))))
        .checked(alternate)
        .size(Size::Sm)
        .on_change(cx.listener(move |this, _: &gpui::ClickEvent, _window, cx| {
          this.edit_motion(id, "Alternate", cx, move |motion| {
            motion.alternate = !alternate
          });
        }))
        .into_any_element(),
        cx,
      ));
      blocks.push(note(
        "A looping animation asks for a frame forever. It is honest \
                 about what it costs — use it for a hint, not for a screen.",
        cx,
      ));
    }

    blocks.push(
      div()
        .id("motion-replay")
        .flex()
        .items_center()
        .justify_center()
        .gap(px(5.))
        .py(px(5.))
        .rounded(px(5.))
        .bg(chrome.raised)
        .text_color(chrome.text)
        .text_size(px(11.))
        .hover(move |style| style.bg(chrome.accent_soft).text_color(chrome.accent))
        .child(icon("rotate-ccw"))
        .child("Play again")
        .on_click(cx.listener(|this, _, _window, cx| this.replay_motion(cx)))
        .into_any_element(),
    );

    self.section("motion", "Motion", blocks, cx)
  }

  fn motion_number(
    &mut self,
    id: NodeId,
    label: &'static str,
    key: &'static str,
    value: f32,
    cx: &mut Context<Self>,
    apply: fn(&mut MotionProps, f32),
  ) -> AnyElement {
    let field = self.field(
      format!("{id}/motion/{key}"),
      trim_float(value as f64),
      cx,
      move |this, text, cx| {
        let parsed = number(&text) as f32;
        this.edit_motion(id, label, cx, move |motion| apply(motion, parsed));
      },
    );
    labelled(label, field.into_any_element(), cx)
  }

  fn style_color_row(
    &mut self,
    id: NodeId,
    label: &'static str,
    key: &'static str,
    current: Option<ColorSpec>,
    cx: &mut Context<Self>,
    apply: fn(&mut tailor_model::style::StyleProps, Option<ColorSpec>),
  ) -> AnyElement {
    let chrome = theme::colors(cx);
    let selected = match &current {
      Some(ColorSpec::Named(token)) => Some(*token),
      _ => None,
    };
    let mut swatches = div().flex().flex_wrap().gap(px(4.)).child(
      div()
        .id(ElementId::Name(SharedString::from(format!(
          "none-{id}-{key}"
        ))))
        .flex()
        .items_center()
        .justify_center()
        .size(px(18.))
        .rounded(px(4.))
        .border(px(1.))
        .border_color(chrome.border)
        .when(current.is_none(), |d| d.border_color(chrome.text))
        .text_color(chrome.dimmed)
        .child(icon("slash"))
        .tooltip(tooltip("None"))
        .on_click(cx.listener(move |this, _, _window, cx| {
          this.edit_style(id, label, cx, move |style| apply(style, None));
        })),
    );
    for token in ColorToken::ALL {
      let token = *token;
      let fill = theme(cx).color(theme::color_of(token), 6).hsla();
      let active = selected == Some(token);
      swatches = swatches.child(
        div()
          .id(ElementId::Name(SharedString::from(format!(
            "sc-{id}-{key}-{}",
            token.label()
          ))))
          .size(px(18.))
          .rounded(px(4.))
          .bg(fill)
          .when(active, |d| d.border(px(2.)).border_color(chrome.text))
          .tooltip(tooltip(token.label()))
          .on_click(cx.listener(move |this, _, _window, cx| {
            this.edit_style(id, label, cx, move |style| {
              apply(style, Some(ColorSpec::Named(token)))
            });
          })),
      );
    }
    let hex = match &current {
      Some(ColorSpec::Custom(hex)) => hex.clone(),
      _ => String::new(),
    };
    let field = self.field(
      format!("{id}/style/{key}/hex"),
      hex,
      cx,
      move |this, text, cx| {
        let text = text.trim().to_string();
        if text.is_empty() {
          return;
        }
        this.edit_style(id, label, cx, move |style| {
          apply(style, Some(ColorSpec::Custom(text)))
        });
      },
    );
    labelled(
      label,
      div()
        .flex()
        .flex_col()
        .gap(px(5.))
        .child(swatches)
        .child(field)
        .into_any_element(),
      cx,
    )
  }

  // --- connections --------------------------------------------------------

  fn render_connections(&mut self, id: NodeId, cx: &mut Context<Self>) -> AnyElement {
    let chrome = theme::colors(cx);
    let Some(node) = self.doc().and_then(|doc| doc.node(id)).cloned() else {
      return empty("This node is gone", cx);
    };
    let events = catalog::get(&node.kind)
      .map(|spec| spec.events)
      .unwrap_or(&[]);
    let actions: Vec<String> = self
      .doc()
      .map(|doc| doc.actions.iter().map(|a| a.name.clone()).collect())
      .unwrap_or_default();

    let mut blocks = Vec::new();
    if events.is_empty() {
      blocks.push(note("This component raises no events.", cx));
    }
    for event in events {
      let key = event.key;
      let current = node.events.get(key).cloned().unwrap_or_default();
      let mut options: Vec<(String, String)> = vec![("—".to_string(), String::new())];
      options.extend(actions.iter().map(|name| (name.clone(), name.clone())));
      let control = chip_row(options, current.clone(), cx, move |this, action, cx| {
        this.edit_node(id, "Connect", cx, move |node| {
          if action.is_empty() {
            node.events.remove(key);
          } else {
            node.events.insert(key.to_string(), action);
          }
        });
      });
      blocks.push(labelled(event.label, control, cx));
    }
    blocks.push(
      div()
        .pt(px(4.))
        .child(
          Button::new("add-action", "New action")
            .variant(Variant::Default)
            .size(Size::Xs)
            .left_section(Icon::new(IconName::Plus).size(Size::Xs))
            .on_click(cx.listener(|this, _, _window, cx| this.add_action(cx))),
        )
        .into_any_element(),
    );

    let mut out = vec![self.section("events", "Events", blocks, cx)];
    out.push(self.render_state_editor(cx));
    div()
      .flex()
      .flex_col()
      .gap(px(12.))
      .children(out)
      .child(
        div()
          .text_size(px(10.))
          .text_color(chrome.dimmed)
          .child("Events become methods on the generated screen."),
      )
      .into_any_element()
  }

  /// The document's state variables and actions — Interface Builder's
  /// outlets and actions, in the only shape gpui has for them.
  fn render_state_editor(&mut self, cx: &mut Context<Self>) -> AnyElement {
    let chrome = theme::colors(cx);
    let vars: Vec<StateVar> = self.doc().map(|doc| doc.state.clone()).unwrap_or_default();
    let actions: Vec<ActionDef> = self
      .doc()
      .map(|doc| doc.actions.clone())
      .unwrap_or_default();

    let mut blocks: Vec<AnyElement> = Vec::new();
    for (index, var) in vars.iter().enumerate() {
      let name = var.name.clone();
      let ty = var.ty;
      let field = self.field(
        format!("var/{index}/initial"),
        var.initial.clone(),
        cx,
        move |this, text, cx| {
          this.edit_doc("State", cx, move |doc| {
            if let Some(var) = doc.state.get_mut(index) {
              var.initial = text;
            }
          });
        },
      );
      blocks.push(
        div()
          .flex()
          .flex_col()
          .gap(px(4.))
          .p(px(8.))
          .rounded(px(6.))
          .bg(chrome.raised)
          .child(
            div()
              .flex()
              .items_center()
              .justify_between()
              .child(
                div()
                  .text_size(px(12.))
                  .child(SharedString::from(name.clone())),
              )
              .child(
                div()
                  .id(ElementId::Name(SharedString::from(format!(
                    "rmvar-{index}"
                  ))))
                  .text_color(chrome.dimmed)
                  .child(icon("trash-2"))
                  .on_click(cx.listener(move |this, _, _window, cx| {
                    this.edit_doc("Remove variable", cx, move |doc| {
                      if index < doc.state.len() {
                        doc.state.remove(index);
                      }
                    });
                  })),
              ),
          )
          .child(chip_row(
            VarType::ALL
              .iter()
              .map(|t| (t.label().to_string(), t.label().to_string())),
            ty.label().to_string(),
            cx,
            move |this, label, cx| {
              let next = VarType::ALL
                .iter()
                .copied()
                .find(|t| t.label() == label)
                .unwrap_or(VarType::Text);
              this.edit_doc("Variable type", cx, move |doc| {
                if let Some(var) = doc.state.get_mut(index) {
                  var.ty = next;
                }
              });
            },
          ))
          .child(field)
          .into_any_element(),
      );
    }
    blocks.push(
      Button::new("add-var", "New variable")
        .variant(Variant::Default)
        .size(Size::Xs)
        .left_section(Icon::new(IconName::Plus).size(Size::Xs))
        .on_click(cx.listener(|this, _, _window, cx| this.add_variable(cx)))
        .into_any_element(),
    );

    for (index, action) in actions.iter().enumerate() {
      blocks.push(
        div()
          .flex()
          .items_center()
          .justify_between()
          .px(px(8.))
          .py(px(5.))
          .rounded(px(6.))
          .bg(chrome.raised)
          .child(
            div()
              .flex()
              .items_center()
              .gap(px(6.))
              .child(div().text_color(chrome.dimmed).child(icon("zap")))
              .child(
                div()
                  .text_size(px(12.))
                  .child(SharedString::from(action.name.clone())),
              ),
          )
          .child(
            div()
              .id(ElementId::Name(SharedString::from(format!(
                "rmact-{index}"
              ))))
              .text_color(chrome.dimmed)
              .child(icon("trash-2"))
              .on_click(cx.listener(move |this, _, _window, cx| {
                this.edit_doc("Remove action", cx, move |doc| {
                  if index < doc.actions.len() {
                    doc.actions.remove(index);
                  }
                });
              })),
          )
          .into_any_element(),
      );
    }

    self.section("state", "State and actions", blocks, cx)
  }

  pub fn add_variable(&mut self, cx: &mut Context<Self>) {
    let name = self
      .doc()
      .map(|doc| doc.unique_var_name("value"))
      .unwrap_or_else(|| "value".into());
    self.edit_doc("New variable", cx, move |doc| {
      doc.state.push(StateVar::new(name, VarType::Text));
    });
  }

  pub fn add_action(&mut self, cx: &mut Context<Self>) {
    let name = self
      .doc()
      .map(|doc| doc.unique_action_name("handle"))
      .unwrap_or_else(|| "handle".into());
    self.edit_doc("New action", cx, move |doc| {
      doc.actions.push(ActionDef::new(name));
    });
  }

  // --- identity -------------------------------------------------------------

  fn render_identity(&mut self, id: NodeId, cx: &mut Context<Self>) -> AnyElement {
    let chrome = theme::colors(cx);
    let Some(node) = self.doc().and_then(|doc| doc.node(id)).cloned() else {
      return empty("This node is gone", cx);
    };
    let spec = catalog::get(&node.kind);
    let name = node.name.clone().unwrap_or_default();
    let field = self.field(format!("{id}/name"), name, cx, move |this, text, cx| {
      let trimmed = text.trim().to_string();
      this.edit_node(id, "Rename", cx, move |node| {
        node.name = (!trimmed.is_empty()).then_some(trimmed);
      });
    });

    let generated = spec
      .filter(|spec| spec.ctor.is_entity())
      .map(|_| {
        format!(
          "A field on the generated screen: `{}`",
          tailor_model::snake_case(&tailor_render::nodes::label_of(&node))
        )
      })
      .unwrap_or_else(|| "Rendered inline; it has no field of its own.".to_string());

    self.section(
      "identity",
      "Identity",
      vec![
        labelled("Name", field.into_any_element(), cx),
        labelled(
          "Component",
          div()
            .text_size(px(12.))
            .child(SharedString::from(
              spec
                .map(|spec| spec.title.to_string())
                .unwrap_or_else(|| node.kind.clone()),
            ))
            .into_any_element(),
          cx,
        ),
        labelled(
          "Element id",
          div()
            .text_size(px(11.))
            .text_color(chrome.dimmed)
            .font_family(theme::MONO)
            .child(SharedString::from(id.element_id()))
            .into_any_element(),
          cx,
        ),
        note(generated, cx),
      ],
      cx,
    )
  }

  // --- the document, when nothing is selected --------------------------------

  fn render_document_inspector(&mut self, cx: &mut Context<Self>) -> AnyElement {
    let chrome = theme::colors(cx);
    let Some(doc) = self.doc().cloned() else {
      return empty("No document", cx);
    };
    let doc_id = doc.id.clone();

    let name_field = self.field(
      format!("doc/{doc_id}/name"),
      doc.name.clone(),
      cx,
      move |this, text, cx| {
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() {
          return;
        }
        this.rename_document(&trimmed, cx);
      },
    );
    let module_field = self.field(
      "project/module".to_string(),
      self.project.gen.module.clone(),
      cx,
      move |this, text, cx| {
        std::sync::Arc::make_mut(&mut this.project).gen.module = text.trim().to_string();
        this.dirty = true;
        this.refresh(cx);
      },
    );
    let project_field = self.field(
      "project/name".to_string(),
      self.project.name.clone(),
      cx,
      move |this, text, cx| {
        std::sync::Arc::make_mut(&mut this.project).name = text.trim().to_string();
        this.dirty = true;
        this.refresh(cx);
      },
    );

    let kind = doc.kind;
    let scheme = self.project.theme.scheme;
    let primary = self.project.theme.primary;
    let radius = self.project.theme.radius;
    let preset = self.project.theme.preset.clone();
    let overridden = self.project.theme.is_overridden();
    let has_json = !self.project.theme.json.trim().is_empty();
    let json_error = theme::json_error(&self.project.theme);
    let flavor = self.project.gen.flavor;

    let document = self.section(
      "document",
      "Document",
      vec![
        labelled("Name", name_field.into_any_element(), cx),
        labelled(
          "Kind",
          chip_row(
            DocKind::ALL
              .iter()
              .map(|k| (k.label().to_string(), k.label().to_string())),
            kind.label().to_string(),
            cx,
            move |this, label, cx| {
              let next = if label == "component" {
                DocKind::Component
              } else {
                DocKind::Screen
              };
              this.edit_doc("Document kind", cx, move |doc| doc.kind = next);
            },
          ),
          cx,
        ),
        note(
          format!("Generates as `{}`", tailor_model::pascal_case(&doc.name)),
          cx,
        ),
      ],
      cx,
    );

    // A preset or a loaded theme file decides the scheme and the colours, so
    // the two controls that would otherwise set them are not shown at all
    // rather than shown doing nothing.
    let mut theme_rows = vec![
      labelled(
        "Base",
        chip_row(
          // `chip_row` takes (label, value): the label is what shows, the
          // value is what comes back.
          std::iter::once(("none".to_string(), "none".to_string())).chain(
            tailor_model::THEME_PRESETS
              .iter()
              .map(|preset| (preset.label.to_string(), preset.id.to_string())),
          ),
          if has_json {
            "none".to_string()
          } else {
            preset.clone()
          },
          cx,
          move |this, id, cx| {
            let next = if id == "none" { String::new() } else { id };
            this.set_theme(cx, move |theme| {
              // The two are alternatives, not layers: picking one clears the
              // other, so what is showing is always what is set.
              theme.json = String::new();
              theme.preset = next;
            });
          },
        ),
        cx,
      ),
      labelled("Theme file", self.theme_file_control(has_json, cx), cx),
    ];
    if let Some(error) = json_error {
      theme_rows.push(note(format!("That theme file did not load — {error}"), cx));
    } else if has_json {
      theme_rows.push(note(
        "The theme file sets the scheme and the colours. It exports as `theme.json`.",
        cx,
      ));
    } else if overridden {
      theme_rows.push(note(
        "The preset sets the scheme and the colours. Radius and font still apply.",
        cx,
      ));
    }
    if !overridden {
      theme_rows.push(labelled(
        "Scheme",
        chip_row(
          Scheme::ALL
            .iter()
            .map(|s| (s.label().to_string(), s.label().to_string())),
          scheme.label().to_string(),
          cx,
          move |this, label, cx| {
            let next = if label == "light" {
              Scheme::Light
            } else {
              Scheme::Dark
            };
            this.set_theme(cx, move |theme| theme.scheme = next);
          },
        ),
        cx,
      ));
      theme_rows.push(labelled(
        "Primary",
        div()
          .flex()
          .flex_wrap()
          .gap(px(4.))
          .children(ColorToken::ALL.iter().map(|token| {
            let token = *token;
            let fill = theme(cx).color(theme::color_of(token), 6).hsla();
            div()
              .id(ElementId::Name(SharedString::from(format!(
                "primary-{}",
                token.label()
              ))))
              .size(px(18.))
              .rounded(px(4.))
              .bg(fill)
              .when(primary == token, |d| {
                d.border(px(2.)).border_color(chrome.text)
              })
              .on_click(cx.listener(move |this, _, _window, cx| {
                this.set_theme(cx, move |theme| theme.primary = token);
              }))
          }))
          .into_any_element(),
        cx,
      ));
    }
    theme_rows.push(labelled(
      "Radius",
      chip_row(
        SizeToken::ALL
          .iter()
          .map(|s| (s.label().to_string(), s.label().to_string())),
        radius.label().to_string(),
        cx,
        move |this, label, cx| {
          if let Some(token) = SizeToken::parse(&label) {
            this.set_theme(cx, move |theme| theme.radius = token);
          }
        },
      ),
      cx,
    ));

    let theme_block = self.section("theme", "Theme", theme_rows, cx);

    let generator = self.section(
      "generator",
      "Generator",
      vec![
        labelled("Project", project_field.into_any_element(), cx),
        labelled("Module", module_field.into_any_element(), cx),
        labelled(
          "Flavour",
          chip_row(
            Flavor::ALL
              .iter()
              .map(|f| (f.label().to_string(), f.label().to_string())),
            flavor.label().to_string(),
            cx,
            move |this, label, cx| {
              let next = if label == "macros" {
                Flavor::Macros
              } else {
                Flavor::Plain
              };
              std::sync::Arc::make_mut(&mut this.project).gen.flavor = next;
              this.dirty = true;
              this.refresh(cx);
            },
          ),
          cx,
        ),
      ],
      cx,
    );

    div()
      .flex()
      .flex_col()
      .gap(px(12.))
      .child(document)
      .child(theme_block)
      .child(generator)
      .child(self.render_state_editor(cx))
      .into_any_element()
  }

  /// The Load / Clear pair for a project's theme file. Loading reads the file
  /// straight into the document: a `.tailor` project is one file, so a path
  /// that only resolves on the machine that picked it would be a worse thing
  /// to store than the theme itself.
  fn theme_file_control(&mut self, has_json: bool, cx: &mut Context<Workbench>) -> AnyElement {
    let mut row = div().flex().gap(px(4.)).child(
      Button::new(
        "theme-file-load",
        if has_json { "Replace…" } else { "Load…" },
      )
      .size(Size::Xs)
      .variant(Variant::Default)
      .on_click(cx.listener(|this, _, _window, cx| this.load_theme_file(cx))),
    );
    if has_json {
      row = row.child(
        Button::new("theme-file-clear", "Clear")
          .size(Size::Xs)
          .variant(Variant::Subtle)
          .on_click(cx.listener(|this, _, _window, cx| {
            this.set_theme(cx, |theme| theme.json = String::new());
          })),
      );
    }
    row.into_any_element()
  }

  fn load_theme_file(&mut self, cx: &mut Context<Workbench>) {
    let receiver = cx.prompt_for_paths(gpui::PathPromptOptions {
      files: true,
      directories: false,
      multiple: false,
      prompt: None,
    });
    cx.spawn(async move |this, cx| {
      let Ok(Ok(Some(paths))) = receiver.await else {
        return;
      };
      let Some(path) = paths.into_iter().next() else {
        return;
      };
      let read = std::fs::read_to_string(&path);
      this
        .update(cx, |this, cx| match read {
          Ok(source) => {
            // Parsed here rather than on the way out, so a bad file is a
            // message now instead of a surprise at export.
            match guise::theme::Theme::from_json(&source) {
              Ok(_) => this.set_theme(cx, move |theme| {
                theme.preset = String::new();
                theme.json = source.trim().to_string();
              }),
              Err(err) => this.toasts.failed(format!("Not a theme file: {err}"), cx),
            }
          }
          Err(err) => this
            .toasts
            .failed(format!("Could not read that file: {err}"), cx),
        })
        .ok();
    })
    .detach();
  }

  pub fn set_theme(
    &mut self,
    cx: &mut Context<Self>,
    f: impl FnOnce(&mut tailor_model::ThemeSpec),
  ) {
    let before = self.project.clone();
    self.history.commit("Theme", &before);
    f(&mut Arc::make_mut(&mut self.project).theme);
    self.dirty = true;
    theme::install(&self.project.theme, cx);
    self.refresh(cx);
  }

  /// Renaming a document also rewrites every `@Name` that referred to it.
  pub fn rename_document(&mut self, name: &str, cx: &mut Context<Self>) {
    let Some(old) = self.doc().map(|doc| doc.name.clone()) else {
      return;
    };
    if old == name {
      return;
    }
    let before = self.project.clone();
    self.history.commit_run("Rename document", &before);
    self.dirty = true;
    let new = name.to_string();
    if let Some(doc) = self.doc_mut() {
      doc.name = new.clone();
    }
    for doc in &mut Arc::make_mut(&mut self.project).docs {
      for node in doc.nodes.values_mut() {
        if node.component_ref() == Some(old.as_str()) {
          node.kind = format!("@{new}");
        }
      }
    }
    self.refresh(cx);
  }

  // --- field cache -----------------------------------------------------------

  /// A cached single-line field. Cached by key so typing does not rebuild the
  /// entity — which would take the focus with it on the first keystroke.
  /// Focus the field the last command asked for, once whichever panel owns
  /// it has built it. Bounded, because a command can ask for a field the
  /// user then navigates away from before it is ever rendered.
  pub fn take_pending_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let Some((key, waited)) = self.focus_field.take() else {
      return;
    };
    match self.fields.get(&key) {
      Some(field) => {
        let handle = field.read(cx).focus_handle();
        window.focus(&handle);
        field.update(cx, |field, cx| field.select_all(cx));
      }
      None if waited < 4 => self.focus_field = Some((key, waited + 1)),
      None => {}
    }
  }

  pub fn field(
    &mut self,
    key: String,
    initial: String,
    cx: &mut Context<Self>,
    apply: impl Fn(&mut Workbench, String, &mut Context<Workbench>) + 'static,
  ) -> Entity<TextInput> {
    if let Some(field) = self.fields.get(&key) {
      return field.clone();
    }
    let field = cx.new(|cx| TextInput::new(cx).value(&initial).size(Size::Sm));
    let sub = cx.subscribe(
      &field,
      move |this: &mut Workbench, _, event: &TextInputEvent, cx| {
        if let TextInputEvent::Change(text) = event {
          apply(this, text.clone(), cx);
        }
      },
    );
    self.subs.push(sub);
    self.fields.insert(key, field.clone());
    field
  }

  /// The multi-line version, for list props.
  fn area(
    &mut self,
    key: String,
    initial: String,
    cx: &mut Context<Self>,
    apply: impl Fn(&mut Workbench, String, &mut Context<Workbench>) + 'static,
  ) -> Entity<TextArea> {
    if let Some(field) = self.areas.get(&key) {
      return field.clone();
    }
    let field = cx.new(|cx| TextArea::new(cx).value(&initial).rows(3).size(Size::Sm));
    let sub = cx.subscribe(
      &field,
      move |this: &mut Workbench, _, event: &TextAreaEvent, cx| {
        apply(this, event.0.clone(), cx);
      },
    );
    self.subs.push(sub);
    self.areas.insert(key, field.clone());
    field
  }
}

fn bindable(prop: &PropSpec) -> bool {
  matches!(
    prop.ty,
    PropType::Text | PropType::MultilineText | PropType::Int | PropType::Float | PropType::Bool
  )
}

impl Workbench {
  /// A titled group of controls that folds away. The key is what persists,
  /// so a section stays folded across selections and across launches.
  fn section(
    &self,
    key: &'static str,
    title: impl Into<SharedString>,
    children: Vec<AnyElement>,
    cx: &mut Context<Self>,
  ) -> AnyElement {
    let folded = self.settings.is_folded(key);
    div()
      .flex()
      .flex_col()
      .gap(px(9.))
      .child(self.fold_header(key, title, cx))
      .when(!folded, |d| d.children(children))
      .into_any_element()
  }
}

/// A label above a control.
fn labelled(
  label: impl Into<SharedString>,
  control: AnyElement,
  cx: &mut Context<Workbench>,
) -> AnyElement {
  let chrome = theme::colors(cx);
  div()
    .flex()
    .flex_col()
    .gap(px(3.))
    .child(
      div()
        .text_size(px(11.))
        .text_color(chrome.dimmed)
        .child(label.into()),
    )
    .child(control)
    .into_any_element()
}

fn note(text: impl Into<SharedString>, cx: &mut Context<Workbench>) -> AnyElement {
  let chrome = theme::colors(cx);
  div()
    .text_size(px(11.))
    .text_color(chrome.dimmed)
    .child(text.into())
    .into_any_element()
}

fn empty(text: impl Into<SharedString>, cx: &mut Context<Workbench>) -> AnyElement {
  note(text, cx)
}

/// A row of small selectable chips — the control this inspector uses for every
/// enum, because a dropdown for four options is three clicks too many.
fn chip_row(
  options: impl IntoIterator<Item = (String, String)>,
  selected: String,
  cx: &mut Context<Workbench>,
  apply: impl Fn(&mut Workbench, String, &mut Context<Workbench>) + Clone + 'static,
) -> AnyElement {
  let chrome = theme::colors(cx);
  div()
    .flex()
    .flex_wrap()
    .gap(px(3.))
    .children(options.into_iter().map(|(label, value)| {
      let active = value == selected;
      let apply = apply.clone();
      let for_click = value.clone();
      div()
        .id(ElementId::Name(SharedString::from(format!(
          "chip-{label}-{value}"
        ))))
        .px(px(7.))
        .py(px(3.))
        .rounded(px(5.))
        .text_size(px(11.))
        .when(active, |d| {
          d.bg(chrome.accent_soft).text_color(chrome.accent)
        })
        .when(!active, |d| d.bg(chrome.raised).text_color(chrome.dimmed))
        .child(SharedString::from(label))
        .on_click(cx.listener(move |this, _, _window, cx| {
          apply(this, for_click.clone(), cx);
        }))
    }))
    .into_any_element()
}

/// A number from a field. Non-finite text — "inf", "NaN" — parses fine in Rust
/// and then cannot be written to JSON at all, so it is refused here rather than
/// on the way to a file that would no longer load.
fn number(text: &str) -> f64 {
  text
    .trim()
    .parse::<f64>()
    .ok()
    .filter(|value| value.is_finite())
    .unwrap_or(0.0)
}

/// `12` rather than `12.0`, and `0.5` rather than `0.5000000001`.
fn trim_float(value: f64) -> String {
  if value.fract().abs() < f64::EPSILON {
    format!("{}", value.trunc() as i64)
  } else {
    format!("{value:.3}")
      .trim_end_matches('0')
      .trim_end_matches('.')
      .to_string()
  }
}
