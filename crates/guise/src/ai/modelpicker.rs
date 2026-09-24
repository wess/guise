//! `AIModelPicker` — choose the model.
//!
//! A model is more than a name: which one is selected changes what a request
//! costs and how much context it has, and both matter at the moment of
//! choosing. So each row carries its description and context size, and the
//! picker hands back the whole [`AIModel`] rather than an index the host has
//! to look up.

use gpui::prelude::*;
use gpui::{
  deferred, div, px, Context, EventEmitter, FocusHandle, IntoElement, MouseButton, SharedString,
  Window,
};

use super::cost::AIPricing;
use super::tokenmeter::compact;
use crate::devtools::ProbedAny;
use crate::icon::{Icon, IconName};
use crate::input::{control_metrics, Field};
use crate::style::TextOverflowExt;
use crate::theme::{theme, ColorName, Size};

/// One selectable model.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AIModel {
  /// What the API is called with.
  pub id: String,
  /// What the user reads.
  pub label: String,
  /// One line on what it's for.
  pub description: Option<String>,
  /// Context window in tokens, 0 when unknown.
  pub context: u64,
  pub pricing: Option<AIPricing>,
}

impl AIModel {
  pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
    AIModel {
      id: id.into(),
      label: label.into(),
      description: None,
      context: 0,
      pricing: None,
    }
  }

  pub fn description(mut self, description: impl Into<String>) -> Self {
    self.description = Some(description.into());
    self
  }

  pub fn context(mut self, tokens: u64) -> Self {
    self.context = tokens;
    self
  }

  pub fn pricing(mut self, pricing: AIPricing) -> Self {
    self.pricing = Some(pricing);
    self
  }
}

/// Emitted when the selection changes. Carries the chosen model.
#[derive(Debug, Clone)]
pub struct AIModelPickerEvent(pub AIModel);

/// A dropdown over a list of models.
pub struct AIModelPicker {
  models: Vec<AIModel>,
  selected: usize,
  open: bool,
  focus: FocusHandle,
  label: Option<SharedString>,
  placeholder: SharedString,
  size: Size,
  disabled: bool,
}

impl EventEmitter<AIModelPickerEvent> for AIModelPicker {}

impl AIModelPicker {
  pub fn new(cx: &mut Context<Self>) -> Self {
    AIModelPicker {
      models: Vec::new(),
      selected: 0,
      open: false,
      focus: cx.focus_handle().tab_stop(true),
      label: None,
      placeholder: SharedString::new_static("Select a model"),
      size: Size::Sm,
      disabled: false,
    }
  }

  pub fn models(mut self, models: impl IntoIterator<Item = AIModel>) -> Self {
    self.models = models.into_iter().collect();
    self
  }

  /// Select by model id, which is what a saved preference holds.
  pub fn selected_id(mut self, id: &str) -> Self {
    if let Some(index) = self.models.iter().position(|model| model.id == id) {
      self.selected = index;
    }
    self
  }

  pub fn label(mut self, label: impl Into<SharedString>) -> Self {
    self.label = Some(label.into());
    self
  }

  pub fn size(mut self, size: Size) -> Self {
    self.size = size;
    self
  }

  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  pub fn focus_handle(&self) -> FocusHandle {
    self.focus.clone()
  }

  /// The chosen model, or `None` when the list is empty or the index is
  /// stale.
  pub fn selection(&self) -> Option<&AIModel> {
    self.models.get(self.selected)
  }

  /// Choose by id at runtime. Returns whether the id was found.
  pub fn select_id(&mut self, id: &str, cx: &mut Context<Self>) -> bool {
    let Some(index) = self.models.iter().position(|model| model.id == id) else {
      return false;
    };
    self.choose(index, cx);
    true
  }

  /// Replace the list, keeping the selection on the same model id when it
  /// survives the swap.
  pub fn set_models(&mut self, models: Vec<AIModel>, cx: &mut Context<Self>) {
    let current = self.selection().map(|model| model.id.clone());
    self.models = models;
    self.selected = current
      .and_then(|id| self.models.iter().position(|model| model.id == id))
      .unwrap_or(0);
    cx.notify();
  }

  fn choose(&mut self, index: usize, cx: &mut Context<Self>) {
    let Some(model) = self.models.get(index).cloned() else {
      return;
    };
    self.selected = index;
    self.open = false;
    cx.emit(AIModelPickerEvent(model));
    cx.notify();
  }
}

impl Render for AIModelPicker {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let t = theme(cx);
    let font_xs = t.font_size(Size::Xs);
    // From `control_metrics`, so the picker lines up with a `Select` or a
    // `TextInput` beside it — the ad-hoc `30.0 + font` made it 44px where
    // every other control at `Size::Sm` is 36.
    let (height, pad_x, font) = control_metrics(self.size);
    let radius = t.radius(t.default_radius);
    let focused = self.focus.is_focused(window) && !self.disabled;
    let border = if focused { t.primary() } else { t.border() }.hsla();
    let plain_border = t.border().hsla();
    let text_color = t.text().hsla();
    let dimmed = t.dimmed().hsla();
    let surface = t.surface().hsla();
    let surface_hover = t.surface_hover().hsla();
    let selected_bg = t.primary().alpha(0.12);

    let current = self.selection();
    let trigger = div()
      .id("guise-ai-modelpicker")
      .track_focus(&self.focus)
      .flex()
      .items_center()
      .justify_between()
      .gap(px(8.0))
      .h(px(height))
      .px(px(pad_x))
      .rounded(px(radius))
      .border_1()
      .border_color(border)
      .bg(surface)
      .text_size(px(font))
      .when(!self.disabled, |trigger| trigger.cursor_pointer())
      .child(
        div()
          .flex()
          .items_center()
          .gap(px(6.0))
          .flex_1()
          .min_w(px(0.0))
          .child(
            Icon::new(IconName::Cpu)
              .size(Size::Xs)
              .color(ColorName::Gray),
          )
          .child(match current {
            Some(model) => div()
              .truncate_text()
              .text_color(text_color)
              .child(SharedString::from(model.label.clone())),
            None => div()
              .truncate_text()
              .text_color(dimmed)
              .child(self.placeholder.clone()),
          }),
      )
      .when_some(
        current.filter(|model| model.context > 0).map(|m| m.context),
        |trigger, context| {
          trigger.child(
            div()
              .text_size(px(font_xs))
              .text_color(dimmed)
              .child(SharedString::from(compact(context))),
          )
        },
      )
      .child(
        Icon::new(IconName::ChevronDown)
          .size(Size::Xs)
          .color(ColorName::Gray),
      )
      .on_mouse_down(
        MouseButton::Left,
        cx.listener(|this, _event, window, cx| {
          if !this.disabled {
            this.open = !this.open;
            window.focus(&this.focus, cx);
            cx.notify();
          }
        }),
      );

    let mut wrap = div().relative().child(trigger);

    if self.open && !self.disabled {
      let mut menu = div()
        .occlude()
        .absolute()
        .top(px(height + 6.0))
        .left(px(0.0))
        .min_w(px(260.0))
        .flex()
        .flex_col()
        .gap(px(2.0))
        .p(px(4.0))
        .rounded(px(radius))
        .border_1()
        .border_color(plain_border)
        .bg(surface)
        .shadow_md();

      for (index, model) in self.models.iter().enumerate() {
        let chosen = index == self.selected;
        menu = menu.child(
          div()
            .id(("guise-ai-model", index))
            .flex()
            .flex_col()
            .gap(px(1.0))
            .px(px(8.0))
            .py(px(6.0))
            .rounded(px(radius))
            .cursor_pointer()
            .when(chosen, |row| row.bg(selected_bg))
            .hover(move |row| row.bg(surface_hover))
            .child(
              div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(10.0))
                .text_size(px(font))
                .text_color(text_color)
                .child(SharedString::from(model.label.clone()))
                .when(model.context > 0, |row| {
                  row.child(div().text_size(px(font_xs)).text_color(dimmed).child(
                    SharedString::from(format!("{} ctx", compact(model.context))),
                  ))
                }),
            )
            .when_some(model.description.clone(), |row, description| {
              row.child(
                div()
                  .text_size(px(font_xs))
                  .text_color(dimmed)
                  .child(SharedString::from(description)),
              )
            })
            .on_click(cx.listener(move |this, _event, _window, cx| this.choose(index, cx))),
        );
      }
      wrap = wrap.child(deferred(menu));
    }

    // `Field` is the shared label/description/error chrome every other
    // input composes; the picker had grown its own copy of the label half.
    let mut chrome = Field::new().child(wrap);
    if let Some(label) = self.label.clone() {
      chrome = chrome.label(label);
    }
    chrome.probe_any("AIModelPicker")
  }
}
