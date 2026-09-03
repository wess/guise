//! The guise component for a node.
//!
//! One arm per catalog kind. Entity-backed components come out of the preview
//! store; the five closure-region containers are drawn from the theme here,
//! because a `'static` content closure is not something a designer can drop
//! into. Everything else is the real component, built from the node's props.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, App, ElementId, SharedString, Window};
use guise::prelude::*;
use tailor_model::node::DEFAULT_SLOT;
use tailor_model::{Node, NodeId};

use crate::chrome;
use crate::read::{align_of, justify_of, Reader};
use crate::store::Preview;
use crate::{Mode, RenderCtx};

use super::{label_of, render_component, slot_children};

pub fn element(ctx: &RenderCtx, node: &Node, window: &mut Window, cx: &mut App) -> AnyElement {
  if let Some(name) = node.component_ref() {
    return render_component(ctx, name, window, cx);
  }
  if let Some(preview) = ctx.store.read(cx).get(node.id).cloned() {
    return entity_element(preview);
  }
  let Some(doc) = ctx.doc() else {
    return chrome::missing("no document").into_any_element();
  };
  let read = Reader::new(node, doc);
  let id = || ElementId::Name(SharedString::from(node.id.element_id()));

  match node.kind.as_str() {
    // --- layout -------------------------------------------------------
    "stack" => {
      let mut stack = Stack::new().gap(read.size("gap"));
      if let Some(align) = read
        .get("align")
        .as_str()
        .and_then(tailor_model::AlignToken::parse)
      {
        stack = stack.align(align_of(align));
      }
      if let Some(justify) = read
        .get("justify")
        .as_str()
        .and_then(tailor_model::JustifyToken::parse)
      {
        stack = stack.justify(justify_of(justify));
      }
      stack
        .children(children(ctx, node, window, cx))
        .into_any_element()
    }
    "group" => {
      let mut group = Group::new()
        .gap(read.size("gap"))
        .wrap(read.bool("wrap"))
        .grow(read.bool("grow"));
      if let Some(align) = read
        .get("align")
        .as_str()
        .and_then(tailor_model::AlignToken::parse)
      {
        group = group.align(align_of(align));
      }
      if let Some(justify) = read
        .get("justify")
        .as_str()
        .and_then(tailor_model::JustifyToken::parse)
      {
        group = group.justify(justify_of(justify));
      }
      group
        .children(children(ctx, node, window, cx))
        .into_any_element()
    }
    "center" => Center::new()
      .inline(read.bool("inline"))
      .children(children(ctx, node, window, cx))
      .into_any_element(),
    "grid" => SimpleGrid::new(read.usize("cols").max(1))
      .spacing(read.size("spacing"))
      .children(children(ctx, node, window, cx))
      .into_any_element(),
    "container" => Container::new()
      .size(read.size("size"))
      .padding(read.size("padding"))
      .children(children(ctx, node, window, cx))
      .into_any_element(),
    "space" => {
      let size = read.size("size");
      if read.choice("axis") == "x" {
        Space::x(size).into_any_element()
      } else {
        Space::y(size).into_any_element()
      }
    }
    "divider" => {
      let mut divider = if read.choice("orientation") == "vertical" {
        Divider::vertical()
      } else {
        Divider::new()
      };
      if !read.text("label").is_empty() {
        divider = divider.label(read.text("label"));
      }
      divider.into_any_element()
    }
    "card" => Card::new()
      .padding(read.size("padding"))
      .radius(read.size("radius"))
      .with_border(read.bool("with_border"))
      .shadow(read.size("shadow"))
      .children(children(ctx, node, window, cx))
      .into_any_element(),
    "paper" => Paper::new()
      .padding(read.size("padding"))
      .radius(read.size("radius"))
      .with_border(read.bool("with_border"))
      .shadow(read.size("shadow"))
      .children(children(ctx, node, window, cx))
      .into_any_element(),
    "panel" => {
      let mut panel = Panel::new()
        .id(id())
        .padding(read.size("padding"))
        .radius(read.size("radius"))
        .with_border(read.bool("with_border"))
        .shadow(read.size("shadow"))
        .collapsed(read.bool("collapsed"));
      if !read.text("title").is_empty() {
        panel = panel.title(read.text("title"));
      }
      if !read.text("description").is_empty() {
        panel = panel.description(read.text("description"));
      }
      if read.bool("collapsible") {
        panel = panel.collapsible();
      }
      if let Some(icon) = first(ctx, node, "icon", window, cx) {
        panel = panel.icon(icon);
      }
      if let Some(action) = first(ctx, node, "action", window, cx) {
        panel = panel.action(action);
      }
      if let Some(footer) = first(ctx, node, "footer", window, cx) {
        panel = panel.footer(footer);
      }
      panel
        .children(children(ctx, node, window, cx))
        .into_any_element()
    }
    "scrollarea" => {
      let mut area = ScrollArea::new(id())
        .max_height(read.f32("max_height"))
        .horizontal(read.bool("horizontal"));
      if read.bool("fill") {
        area = area.fill();
      }
      area
        .children(children(ctx, node, window, cx))
        .into_any_element()
    }
    "appshell" => appshell(ctx, node, &read, window, cx),
    "splitpanel" => splitpanel(ctx, node, &read, window, cx),
    "flexrow" | "flexcolumn" => {
      // The flex family takes pixel gaps; on the canvas a plain flex box
      // with the same numbers is indistinguishable and keeps the drop
      // strips working.
      let mut root = div().flex().gap(px(read.f32("gap")));
      root = if node.kind == "flexrow" {
        root.flex_row()
      } else {
        root.flex_col()
      };
      root
        .children(children(ctx, node, window, cx))
        .into_any_element()
    }
    "expanded" => div()
      .flex_grow()
      .children(children(ctx, node, window, cx))
      .into_any_element(),

    // --- typography ---------------------------------------------------
    "text" => {
      let mut text = Text::new(read.text("content")).size(read.size("size"));
      text = match read.choice("weight").as_str() {
        "medium" => text.medium(),
        "semibold" | "bold" => text.bold(),
        _ => text,
      };
      if read.bool("dimmed") {
        text = text.dimmed();
      } else if node.prop("color").is_some() {
        text = text.color(read.color_value("color", cx));
      }
      text.into_any_element()
    }
    "title" => {
      let mut title = Title::new(read.text("content")).order(read.usize("order").clamp(1, 6) as u8);
      if node.prop("color").is_some() {
        title = title.color(read.color_value("color", cx));
      }
      title.into_any_element()
    }
    "anchor" => Anchor::new(id(), read.text("label"))
      .color(read.color_name("color"))
      .size(read.size("size"))
      .into_any_element(),
    "code" => Code::new(read.text("content"))
      .color(read.color_name("color"))
      .into_any_element(),
    "kbd" => Kbd::new(read.text("key")).into_any_element(),
    "kbdgroup" => div()
      .flex()
      .gap(px(4.))
      .children(
        read
          .items("keys")
          .into_iter()
          .map(|key| Kbd::new(key).into_any_element()),
      )
      .into_any_element(),
    "mark" => Mark::new(read.text("content"))
      .color(read.color_name("color"))
      .size(read.size("size"))
      .into_any_element(),
    "blockquote" => {
      let mut quote = Blockquote::new()
        .text(read.text("text"))
        .color(read.color_name("color"))
        .padding(read.size("padding"))
        .radius(read.size("radius"));
      if !read.text("cite").is_empty() {
        quote = quote.cite(read.text("cite"));
      }
      if let Some(icon) = read.icon("icon") {
        quote = quote.icon(icon);
      }
      quote.into_any_element()
    }
    "markdown" => Markdown::new(read.text("source"))
      .size(read.size("size"))
      .accent(read.color_name("accent"))
      .into_any_element(),
    "spoiler" => Spoiler::new(id())
      .max_height(read.f32("max_height"))
      .expanded(read.bool("expanded"))
      .color(read.color_name("color"))
      .size(read.size("size"))
      .children(children(ctx, node, window, cx))
      .into_any_element(),

    // --- controls -----------------------------------------------------
    "button" => {
      let mut button = Button::new(id(), read.text("label"))
        .variant(read.variant("variant"))
        .color(read.color("color", cx))
        .size(read.size("size"))
        .radius(read.size("radius"))
        .full_width(read.bool("full_width"))
        .disabled(read.bool("disabled"));
      if let Some(left) = first(ctx, node, "left", window, cx) {
        button = button.left_section(left);
      }
      if let Some(right) = first(ctx, node, "right", window, cx) {
        button = button.right_section(right);
      }
      button.into_any_element()
    }
    "actionicon" => {
      let glyph: Glyph = read
        .icon("icon")
        .map(Glyph::from)
        .unwrap_or_else(|| Glyph::from(read.text("icon")));
      let mut action = ActionIcon::new(id(), glyph)
        .variant(read.variant("variant"))
        .color(read.color("color", cx))
        .size(read.size("size"))
        .radius(read.size("radius"))
        .disabled(read.bool("disabled"));
      let label = read.text("label");
      if !label.is_empty() {
        action = action.label(label);
      }
      action.into_any_element()
    }
    "closebutton" => CloseButton::new(id())
      .size(read.size("size"))
      .into_any_element(),
    "badge" => Badge::new(read.text("label"))
      .variant(read.variant("variant"))
      .color(read.color("color", cx))
      .size(read.size("size"))
      .into_any_element(),
    "chip" => Chip::new(id(), read.text("label"))
      .checked(checked(ctx, node.id, read.bool("checked"), cx))
      .color(read.color("color", cx))
      .size(read.size("size"))
      .into_any_element(),
    "icon" => {
      let glyph: Glyph = read
        .icon("icon")
        .map(Glyph::from)
        .unwrap_or_else(|| Glyph::from(read.text("icon")));
      match read.icon("icon") {
        Some(name) => Icon::new(name)
          .size(read.size("size"))
          .color(read.color_name("color"))
          .into_any_element(),
        None => div().child(glyph).into_any_element(),
      }
    }
    "themeicon" => {
      let glyph: Glyph = read
        .icon("icon")
        .map(Glyph::from)
        .unwrap_or_else(|| Glyph::from(read.text("icon")));
      ThemeIcon::new(glyph)
        .variant(read.variant("variant"))
        .color(read.color_name("color"))
        .size(read.size("size"))
        .radius(read.size("radius"))
        .into_any_element()
    }
    "indicator" => {
      let child = first(ctx, node, "child", window, cx)
        .unwrap_or_else(|| chrome::empty_slot("Target", cx).into_any_element());
      let mut indicator = Indicator::new(child)
        .color(read.color_name("color"))
        .disabled(read.bool("disabled"));
      if !read.text("label").is_empty() {
        indicator = indicator.label(read.text("label"));
      }
      indicator.into_any_element()
    }
    "rating" => Rating::new(id())
      .count(read.usize("count").clamp(1, 10))
      .color(read.color("color", cx))
      .size(read.size("size"))
      .readonly(read.bool("readonly") || ctx.mode != Mode::Preview)
      .into_any_element(),

    // --- inputs that are not entities ---------------------------------
    "checkbox" => Checkbox::new(id())
      .checked(checked(ctx, node.id, read.bool("checked"), cx))
      .indeterminate(read.bool("indeterminate"))
      .label(read.text("label"))
      .size(read.size("size"))
      .color(read.color_name("color"))
      .disabled(read.bool("disabled"))
      .into_any_element(),
    "switch" => Switch::new(id())
      .checked(checked(ctx, node.id, read.bool("checked"), cx))
      .label(read.text("label"))
      .size(read.size("size"))
      .color(read.color_name("color"))
      .disabled(read.bool("disabled"))
      .into_any_element(),
    "radio" => Radio::new(id())
      .checked(checked(ctx, node.id, read.bool("checked"), cx))
      .label(read.text("label"))
      .size(read.size("size"))
      .color(read.color_name("color"))
      .disabled(read.bool("disabled"))
      .into_any_element(),
    "radiogroup" => {
      let mut group = RadioGroup::new()
        .options(read.items("options"))
        .color(read.color_name("color"))
        .size(read.size("size"));
      if !read.text("label").is_empty() {
        group = group.label(read.text("label"));
      }
      group.into_any_element()
    }
    "checkboxgroup" => {
      let mut group = CheckboxGroup::new()
        .options(read.items("options"))
        .color(read.color_name("color"))
        .size(read.size("size"));
      if !read.text("label").is_empty() {
        group = group.label(read.text("label"));
      }
      group.into_any_element()
    }
    "calendar" => Calendar::new(id())
      .month(
        read.usize("year") as i32,
        read.usize("month").clamp(1, 12) as u32,
      )
      .size(read.size("size"))
      .into_any_element(),
    "dropzone" => {
      let mut zone = Dropzone::new(id())
        .height(read.f32("height"))
        .accept(read.items("accept"));
      if !read.text("label").is_empty() {
        zone = zone.label(read.text("label"));
      }
      if !read.text("hint").is_empty() {
        zone = zone.hint(read.text("hint"));
      }
      if let Some(icon) = read.icon("icon") {
        zone = zone.icon(icon);
      }
      if read.bool("single") {
        zone = zone.single();
      }
      zone.into_any_element()
    }
    "field" => {
      let mut field = Field::new();
      if !read.text("label").is_empty() {
        field = field.label(read.text("label"));
      }
      if !read.text("description").is_empty() {
        field = field.description(read.text("description"));
      }
      if !read.text("error").is_empty() {
        field = field.error(read.text("error"));
      }
      for child in children(ctx, node, window, cx) {
        field = field.child(child);
      }
      field.into_any_element()
    }

    // --- data ----------------------------------------------------------
    "avatar" => Avatar::new(read.text("initials"))
      .color(read.color_name("color"))
      .variant(read.variant("variant"))
      .size(read.size("size"))
      .radius(read.size("radius"))
      .into_any_element(),
    "avatargroup" => AvatarGroup::new()
      .avatars(read.items("avatars"))
      .size(read.size("size"))
      .limit(read.usize("limit").max(1))
      .into_any_element(),
    "list" => {
      let mut list = List::new()
        .items(read.items("items"))
        .ordered(read.bool("ordered"))
        .size(read.size("size"))
        .spacing(read.size("spacing"));
      if let Some(icon) = read.icon("icon") {
        list = list.icon(icon);
      }
      list.into_any_element()
    }
    "table" => {
      let mut table = Table::new()
        .striped(read.bool("striped"))
        .highlight_on_hover(read.bool("highlight_on_hover"))
        .with_border(read.bool("with_border"));
      let head = read.items("head");
      if !head.is_empty() {
        table = table.head(head);
      }
      for row in read.raw_items("rows") {
        let cells: Vec<String> = row.split('|').map(|c| c.trim().to_string()).collect();
        table = table.row(cells);
      }
      table.into_any_element()
    }
    "timeline" => {
      let mut timeline = Timeline::new()
        .active(read.usize("active"))
        .color(read.color_name("color"));
      for item in read.raw_items("items") {
        match item.split_once('|') {
          Some((title, description)) => {
            timeline = timeline.item_desc(title.trim().to_string(), description.trim().to_string())
          }
          None => timeline = timeline.item(item.trim().to_string()),
        }
      }
      timeline.into_any_element()
    }
    "tabs" => tabs(ctx, node, &read, window, cx),
    "accordion" => accordion(ctx, node, &read, window, cx),
    "carousel" => carousel(ctx, node, window, cx),

    // --- feedback ------------------------------------------------------
    "alert" => {
      let mut alert = Alert::new(read.text("message"))
        .variant(read.variant("variant"))
        .color(read.color("color", cx));
      if !read.text("title").is_empty() {
        alert = alert.title(read.text("title"));
      }
      if let Some(icon) = read.icon("icon") {
        alert = alert.icon(icon);
      }
      alert.into_any_element()
    }
    "notification" => {
      let mut note = Notification::new(read.text("message")).color(read.color_name("color"));
      if !read.text("title").is_empty() {
        note = note.title(read.text("title"));
      }
      if let Some(icon) = read.icon("icon") {
        note = note.icon(icon);
      }
      note.into_any_element()
    }
    "loader" => Loader::new()
      .variant(match read.choice("variant").as_str() {
        "bars" => LoaderVariant::Bars,
        _ => LoaderVariant::Dots,
      })
      .size(read.size("size"))
      .color(read.color("color", cx))
      .into_any_element(),
    "progress" => Progress::new(read.f32("value"))
      .color(read.color("color", cx))
      .size(read.size("size"))
      .radius(read.size("radius"))
      .into_any_element(),
    "ringprogress" => {
      let mut ring = RingProgress::new(read.f32("value"))
        .size(read.f32("size"))
        .color(read.color_name("color"));
      if !read.text("label").is_empty() {
        ring = ring.label(read.text("label"));
      }
      ring.into_any_element()
    }
    "skeleton" => Skeleton::new()
      .width(read.f32("width"))
      .height(read.f32("height"))
      .radius(read.size("radius"))
      .into_any_element(),
    "modal" => {
      // A modal on the canvas is laid out in place rather than floated:
      // you are designing its contents, not watching it appear.
      let mut modal = Modal::new()
        .width(read.f32("width"))
        .padding(read.size("padding"))
        .radius(read.size("radius"));
      if !read.text("title").is_empty() {
        modal = modal.title(read.text("title"));
      }
      modal
        .children(children(ctx, node, window, cx))
        .into_any_element()
    }
    "drawer" => {
      let mut drawer = Drawer::new()
        .size(read.f32("size"))
        .padding(read.size("padding"));
      drawer = drawer.side(match read.choice("side").as_str() {
        "left" => Side::Left,
        "top" => Side::Top,
        "bottom" => Side::Bottom,
        _ => Side::Right,
      });
      if !read.text("title").is_empty() {
        drawer = drawer.title(read.text("title"));
      }
      drawer
        .children(children(ctx, node, window, cx))
        .into_any_element()
    }
    "tooltip" => {
      let child = first(ctx, node, "child", window, cx)
        .unwrap_or_else(|| chrome::empty_slot("Target", cx).into_any_element());
      let label = read.text("label");
      div()
        .id(id())
        .child(child)
        .tooltip(tooltip(label))
        .into_any_element()
    }
    "loadingoverlay" => LoadingOverlay::new()
      .visible(read.bool("visible"))
      .into_any_element(),

    // --- navigation ----------------------------------------------------
    "breadcrumbs" => {
      let mut crumbs = Breadcrumbs::new().items(read.items("items"));
      if !read.text("separator").is_empty() {
        crumbs = crumbs.separator(read.text("separator"));
      }
      crumbs.into_any_element()
    }
    "navlink" => {
      let mut link = NavLink::new(id(), read.text("label"))
        .color(read.color_name("color"))
        .active(read.bool("active"));
      if !read.text("description").is_empty() {
        link = link.description(read.text("description"));
      }
      if let Some(icon) = read.icon("icon") {
        link = link.icon(icon);
      }
      link.into_any_element()
    }
    "stepper" => {
      let mut stepper = Stepper::new()
        .active(read.usize("active"))
        .color(read.color_name("color"));
      for item in read.raw_items("steps") {
        match item.split_once('|') {
          Some((label, description)) => {
            stepper = stepper.step_desc(label.trim().to_string(), description.trim().to_string())
          }
          None => stepper = stepper.step(item.trim().to_string()),
        }
      }
      stepper.into_any_element()
    }
    "statusbar" => {
      let mut bar = StatusBar::new().height(read.f32("height"));
      if let Some(left) = first(ctx, node, "left", window, cx) {
        bar = bar.left(left);
      }
      if let Some(center) = first(ctx, node, "center", window, cx) {
        bar = bar.center(center);
      }
      if let Some(right) = first(ctx, node, "right", window, cx) {
        bar = bar.right(right);
      }
      bar.into_any_element()
    }

    // --- charts ---------------------------------------------------------
    "sparkline" => {
      let mut chart = Sparkline::new(read.numbers("values"))
        .color(read.color("color", cx))
        .stroke(read.f32("stroke"))
        .width(read.f32("width"))
        .height(read.f32("height"));
      if read.bool("fill") {
        chart = chart.fill();
      }
      chart.into_any_element()
    }
    "linechart" => {
      let mut chart = LineChart::new(read.numbers("values"))
        .color(read.color("color", cx))
        .stroke(read.f32("stroke"))
        .labels(read.items("labels"))
        .width(read.f32("width"))
        .height(read.f32("height"));
      if read.bool("fill") {
        chart = chart.fill();
      }
      if read.bool("axis") {
        chart = chart.axis();
      }
      if read.bool("hover") {
        chart = chart.hover();
      }
      chart.into_any_element()
    }
    "areachart" => {
      let mut chart = AreaChart::new(read.numbers("values"))
        .labels(read.items("labels"))
        .width(read.f32("width"))
        .height(read.f32("height"));
      if read.bool("axis") {
        chart = chart.axis();
      }
      if read.bool("overlaid") {
        chart = chart.overlaid();
      }
      chart.into_any_element()
    }
    "barchart" => {
      let values = read.numbers("values");
      let labels = read.items("labels");
      let mut chart = if labels.len() == values.len() && !labels.is_empty() {
        BarChart::entries(labels.into_iter().zip(values))
      } else {
        BarChart::new(values)
      };
      chart = chart
        .color(read.color("color", cx))
        .gap(read.f32("gap"))
        .width(read.f32("width"))
        .height(read.f32("height"));
      if read.bool("axis") {
        chart = chart.axis();
      }
      if read.bool("hover") {
        chart = chart.hover();
      }
      chart.into_any_element()
    }
    "piechart" => {
      let mut chart = PieChart::new(read.numbers("values")).size(read.f32("size"));
      let donut = read.f32("donut");
      if donut > 0.0 {
        chart = chart.donut(donut.clamp(0.05, 0.95));
      }
      chart.into_any_element()
    }
    "scatterchart" => {
      let values = read.numbers("values");
      let points: Vec<(f32, f32)> = values
        .chunks(2)
        .filter(|p| p.len() == 2)
        .map(|p| (p[0], p[1]))
        .collect();
      let mut chart = ScatterChart::new(points)
        .width(read.f32("width"))
        .height(read.f32("height"));
      if read.bool("hover") {
        chart = chart.hover();
      }
      chart.into_any_element()
    }

    // --- media -----------------------------------------------------------
    "image" => {
      let source = read.text("source");
      if source.is_empty() {
        chrome::blueprint_box("Image", cx).into_any_element()
      } else {
        Image::new(source.to_string())
          .width(read.f32("width"))
          .height(read.f32("height"))
          .radius(read.size("radius"))
          .fit(match read.choice("fit").as_str() {
            "fill" => ObjectFit::Fill,
            "contain" => ObjectFit::Contain,
            "none" => ObjectFit::None,
            _ => ObjectFit::Cover,
          })
          .into_any_element()
      }
    }

    // --- ai ------------------------------------------------------------
    "aimessage" => {
      let role = match read.choice("role").as_str() {
        "user" => AIRole::User,
        "system" => AIRole::System,
        "tool" => AIRole::Tool,
        _ => AIRole::Assistant,
      };
      let mut message = AIMessage::new(role, read.text("body"))
        .streaming(read.bool("streaming"))
        .avatar(read.bool("avatar"))
        .size(read.size("size"));
      if !read.text("name").is_empty() {
        message = message.name(read.text("name"));
      }
      if !read.text("meta").is_empty() {
        message = message.meta(read.text("meta"));
      }
      if !read.text("error").is_empty() {
        message = message.error(read.text("error"));
      }
      message.into_any_element()
    }
    "aistreamingtext" => AIStreamingText::new(read.text("text"))
      .caret(read.bool("caret"))
      .size(read.size("size"))
      .into_any_element(),
    "aithinking" => {
      let mut thinking = AIThinking::new()
        .color(read.color_name("color"))
        .size(read.size("size"));
      if !read.text("label").is_empty() {
        thinking = thinking.label(read.text("label"));
      }
      thinking.into_any_element()
    }
    "aireasoning" => {
      let mut reasoning = AIReasoning::new(id(), read.text("text"))
        .open(read.bool("open"))
        .streaming(read.bool("streaming"))
        .size(read.size("size"));
      if !read.text("label").is_empty() {
        reasoning = reasoning.label(read.text("label"));
      }
      reasoning.into_any_element()
    }
    "aitoolcall" => {
      let mut call = AIToolCall::new(id(), read.text("name"))
        .status(tool_status(&read.choice("status")))
        .open(read.bool("open"))
        .expandable(read.bool("expandable"))
        .size(read.size("size"));
      if !read.text("arguments").is_empty() {
        call = call.arguments(read.text("arguments"));
      }
      if !read.text("result").is_empty() {
        call = call.result(read.text("result"));
      }
      if !read.text("meta").is_empty() {
        call = call.meta(read.text("meta"));
      }
      call.into_any_element()
    }
    "aitokenmeter" => {
      let mut meter = AITokenMeter::new(read.usize("used") as u64, read.usize("limit") as u64)
        .bar(read.bool("bar"))
        .size(read.size("size"));
      if !read.text("label").is_empty() {
        meter = meter.label(read.text("label"));
      }
      meter.into_any_element()
    }
    "aicost" => {
      let usage = AIUsage::new(read.usize("input") as u64, read.usize("output") as u64);
      let pricing = AIPricing::new(
        price(&read.text("input_price")),
        price(&read.text("output_price")),
      );
      let mut cost = AICost::new(usage, pricing)
        .breakdown(read.bool("breakdown"))
        .size(read.size("size"));
      if !read.text("label").is_empty() {
        cost = cost.label(read.text("label"));
      }
      cost.into_any_element()
    }
    "aicitation" => {
      let mut citation = AICitation::new(id(), read.usize("index")).size(read.size("size"));
      if !read.text("label").is_empty() {
        citation = citation.label(read.text("label"));
      }
      citation.into_any_element()
    }
    "aisources" => {
      let mut sources =
        AISources::new(read.raw_items("sources").iter().map(|line| ai_source(line)))
          .excerpts(read.bool("excerpts"))
          .size(read.size("size"));
      if !read.text("title").is_empty() {
        sources = sources.title(read.text("title"));
      }
      sources.into_any_element()
    }

    // --- settings screens and app chrome --------------------------------
    "settingssection" => {
      let mut section = SettingsSection::new(read.text("title")).rule(read.bool("rule"));
      if !read.text("description").is_empty() {
        section = section.description(read.text("description"));
      }
      section
        .children(children(ctx, node, window, cx))
        .into_any_element()
    }
    "settingsrow" => {
      let mut row = SettingsRow::new(id(), read.text("label"))
        .modified(read.bool("modified"))
        .divider(read.bool("divider"));
      if !read.text("description").is_empty() {
        row = row.description(read.text("description"));
      }
      if let Some(control) = slot_children(ctx, node, "control", window, cx).pop() {
        row = row.control(control);
      }
      row.into_any_element()
    }
    "about" => {
      let mut about = About::new(read.text("name"));
      if !read.text("version").is_empty() {
        about = about.version(read.text("version"));
      }
      if !read.text("tagline").is_empty() {
        about = about.tagline(read.text("tagline"));
      }
      if !read.text("credits").is_empty() {
        about = about.credits(read.text("credits"));
      }
      about.into_any_element()
    }
    "windowcontrols" => WindowControls::new()
      .button_width(read.f32("button_width"))
      .height(read.f32("height"))
      .into_any_element(),
    "themepicker" => ThemePicker::new()
      .system_option(read.bool("system_option"))
      .layout(if read.choice("layout") == "grid" {
        ThemePickerLayout::Grid
      } else {
        ThemePickerLayout::List
      })
      .into_any_element(),
    "confirmmodal" => {
      let mut modal = ConfirmModal::new().width(read.f32("width"));
      if read.bool("danger") {
        modal = modal.danger();
      }
      if !read.text("title").is_empty() {
        modal = modal.title(read.text("title"));
      }
      if !read.text("message").is_empty() {
        modal = modal.message(read.text("message"));
      }
      if !read.text("confirm_label").is_empty() {
        modal = modal.confirm_label(read.text("confirm_label"));
      }
      if !read.text("cancel_label").is_empty() {
        modal = modal.cancel_label(read.text("cancel_label"));
      }
      modal
        .children(children(ctx, node, window, cx))
        .into_any_element()
    }
    "virtuallist" => {
      // Drawn: the real component takes a row builder, and the rows a
      // designer types are the only content there is to build.
      let rows = read.items("rows");
      VirtualList::new(id(), rows.len(), move |index, _window, cx| {
        let text = rows.get(index).cloned().unwrap_or_default();
        div()
          .px(px(10.0))
          .py(px(6.0))
          .text_color(theme(cx).text().hsla())
          .child(text)
      })
      .height(read.f32("height"))
      .into_any_element()
    }
    "settingsview" => settings_view(ctx, node, &read, window, cx),

    _ => chrome::blueprint_box(label_of(node), cx).into_any_element(),
  }
}

/// The default slot's children.
fn children(ctx: &RenderCtx, node: &Node, window: &mut Window, cx: &mut App) -> Vec<AnyElement> {
  slot_children(ctx, node, DEFAULT_SLOT, window, cx)
}

/// The single child of a named slot, or `None` when it is empty.
fn first(
  ctx: &RenderCtx,
  node: &Node,
  slot: &str,
  window: &mut Window,
  cx: &mut App,
) -> Option<AnyElement> {
  let id = node.slot(slot).first().copied()?;
  Some(super::render_in(ctx, id, false, window, cx))
}

/// A controlled component's value: what preview mode last set, or the prop.
fn checked(ctx: &RenderCtx, id: NodeId, fallback: bool, cx: &App) -> bool {
  crate::store::controlled_bool(ctx.store.read(cx), id, fallback)
}

/// The live entity, drawn as itself.
fn entity_element(preview: Preview) -> AnyElement {
  match preview {
    Preview::TextInput(e) => e.into_any_element(),
    Preview::TextArea(e) => e.into_any_element(),
    Preview::NumberInput(e) => e.into_any_element(),
    Preview::PasswordInput(e) => e.into_any_element(),
    Preview::PinInput(e) => e.into_any_element(),
    Preview::Select(e) => e.into_any_element(),
    Preview::Combobox(e) => e.into_any_element(),
    Preview::Autocomplete(e) => e.into_any_element(),
    Preview::Segmented(e) => e.into_any_element(),
    Preview::Slider(e) => e.into_any_element(),
    Preview::RangeSlider(e) => e.into_any_element(),
    Preview::ColorInput(e) => e.into_any_element(),
    Preview::TagsInput(e) => e.into_any_element(),
    Preview::DatePicker(e) => e.into_any_element(),
    Preview::TimePicker(e) => e.into_any_element(),
    Preview::FileInput(e) => e.into_any_element(),
    Preview::Transfer(e) => e.into_any_element(),
    Preview::TreeView(e) => e.into_any_element(),
    Preview::TabBar(e) => e.into_any_element(),
    Preview::Pagination(e) => e.into_any_element(),
    Preview::NavigationMenu(e) => e.into_any_element(),
    Preview::Editor(e) => e.into_any_element(),
    Preview::MarkdownEditor(e) => e.into_any_element(),
    Preview::WebView(e) => e.into_any_element(),
    Preview::CopyButton(e) => e.into_any_element(),
    Preview::AIChatView(e) => e.into_any_element(),
    Preview::AIComposer(e) => e.into_any_element(),
    Preview::AIModelPicker(e) => e.into_any_element(),
    Preview::AISettings(e) => e.into_any_element(),
    Preview::Menu(e) => e.into_any_element(),
    Preview::ContextMenu(e) => e.into_any_element(),
  }
}

/// Tabs, drawn from the theme so the panel behind a tab is a real drop target
/// and clicking a tab reveals its slot.
fn tabs(
  ctx: &RenderCtx,
  node: &Node,
  read: &Reader<'_>,
  window: &mut Window,
  cx: &mut App,
) -> AnyElement {
  let labels = read.raw_items("tabs");
  let active = ctx
    .store
    .read(cx)
    .page(node.id)
    .min(labels.len().saturating_sub(1));
  let border = theme(cx).border().hsla();
  let accent = theme(cx).color(ColorName::Blue, 6).hsla();
  let dimmed = theme(cx).dimmed().hsla();
  let text = theme(cx).text().hsla();

  let mut strip = div()
    .flex()
    .flex_row()
    .gap(px(2.))
    .border_b(px(1.))
    .border_color(border);
  for (index, label) in labels.iter().enumerate() {
    let selected = index == active;
    let reveal = ctx.hooks.reveal.clone();
    let id = node.id;
    strip = strip.child(
      div()
        .id(ElementId::Name(SharedString::from(format!(
          "tab-{}-{index}",
          node.id
        ))))
        .px(px(12.))
        .py(px(8.))
        .text_size(px(13.))
        .text_color(if selected { text } else { dimmed })
        .border_b(px(2.))
        .border_color(if selected {
          accent
        } else {
          gpui::transparent_black()
        })
        .child(SharedString::from(label.clone()))
        .on_mouse_down(gpui::MouseButton::Left, move |_, _window, cx| {
          cx.stop_propagation();
          reveal(id, index, cx);
        }),
    );
  }

  let slot = format!("tab:{active}");
  let panel = div()
    .flex()
    .flex_col()
    .gap(px(8.))
    .pt(px(12.))
    .children(slot_children(ctx, node, &slot, window, cx));

  div()
    .flex()
    .flex_col()
    .child(strip)
    .child(panel)
    .into_any_element()
}

/// The settings screen, drawn: a page list beside the page's slot.
///
/// `SettingsView` takes its content as a `'static` closure keyed by page id,
/// which is the one shape a designer cannot drop into — so the sidebar is
/// drawn from the theme here and each page is a real slot. Generated code uses
/// the real component.
fn settings_view(
  ctx: &RenderCtx,
  node: &Node,
  read: &Reader<'_>,
  window: &mut Window,
  cx: &mut App,
) -> AnyElement {
  let pages = read.raw_items("pages");
  let active = ctx
    .store
    .read(cx)
    .page(node.id)
    .min(pages.len().saturating_sub(1));
  let border = theme(cx).border().hsla();
  let surface = theme(cx).surface().hsla();
  let accent_soft = theme(cx).color(ColorName::Blue, 6).alpha(0.18);
  let dimmed = theme(cx).dimmed().hsla();
  let text = theme(cx).text().hsla();
  let radius = theme(cx).radius(Size::Sm);

  let mut list = div()
    .flex()
    .flex_col()
    .gap(px(2.))
    .w(px(read.f32("sidebar_width").max(120.0)))
    .p(px(8.))
    .bg(surface)
    .border_r(px(1.))
    .border_color(border);
  if read.bool("searchable") {
    list = list.child(
      div()
        .px(px(8.))
        .py(px(5.))
        .mb(px(6.))
        .rounded(px(radius))
        .border_1()
        .border_color(border)
        .text_size(px(12.))
        .text_color(dimmed)
        .child(SharedString::new_static("Search")),
    );
  }
  for (index, label) in pages.iter().enumerate() {
    let selected = index == active;
    let reveal = ctx.hooks.reveal.clone();
    let id = node.id;
    list = list.child(
      div()
        .id(ElementId::Name(SharedString::from(format!(
          "settings-page-{}-{index}",
          node.id
        ))))
        .px(px(8.))
        .py(px(6.))
        .rounded(px(radius))
        .text_size(px(13.))
        .text_color(if selected { text } else { dimmed })
        .when(selected, |el| el.bg(accent_soft))
        .child(SharedString::from(label.clone()))
        .on_mouse_down(gpui::MouseButton::Left, move |_, _window, cx| {
          cx.stop_propagation();
          reveal(id, index, cx);
        }),
    );
  }

  let slot = format!("page:{active}");
  let pane = div()
    .flex()
    .flex_col()
    .flex_1()
    .gap(px(12.))
    .p(px(16.))
    .children(slot_children(ctx, node, &slot, window, cx));

  div()
    .flex()
    .flex_row()
    .border_1()
    .border_color(border)
    .rounded(px(theme(cx).radius(Size::Md)))
    .overflow_hidden()
    .child(list)
    .child(pane)
    .into_any_element()
}

/// Accordion, likewise drawn so each section is a slot you can drop into.
fn accordion(
  ctx: &RenderCtx,
  node: &Node,
  read: &Reader<'_>,
  window: &mut Window,
  cx: &mut App,
) -> AnyElement {
  let labels = read.raw_items("items");
  let open = ctx.store.read(cx).page(node.id);
  let border = theme(cx).border().hsla();
  let text = theme(cx).text().hsla();

  let mut root = div()
    .flex()
    .flex_col()
    .border(px(1.))
    .border_color(border)
    .rounded(px(6.));
  for (index, label) in labels.iter().enumerate() {
    let expanded = index == open || read.bool("multiple");
    let reveal = ctx.hooks.reveal.clone();
    let id = node.id;
    let mut section = div().flex().flex_col();
    section = section.child(
      div()
        .id(ElementId::Name(SharedString::from(format!(
          "acc-{}-{index}",
          node.id
        ))))
        .flex()
        .items_center()
        .justify_between()
        .px(px(12.))
        .py(px(10.))
        .text_size(px(13.))
        .text_color(text)
        .when(index > 0, |d| d.border_t(px(1.)).border_color(border))
        .child(SharedString::from(label.clone()))
        .child(Icon::new(if expanded {
          IconName::ChevronDown
        } else {
          IconName::ChevronRight
        }))
        .on_mouse_down(gpui::MouseButton::Left, move |_, _window, cx| {
          cx.stop_propagation();
          reveal(id, index, cx);
        }),
    );
    if expanded {
      let slot = format!("item:{index}");
      section = section.child(
        div()
          .flex()
          .flex_col()
          .gap(px(8.))
          .px(px(12.))
          .pb(px(12.))
          .children(slot_children(ctx, node, &slot, window, cx)),
      );
    }
    root = root.child(section);
  }
  root.into_any_element()
}

/// A split panel's two regions, side by side at the configured ratio.
fn splitpanel(
  ctx: &RenderCtx,
  node: &Node,
  read: &Reader<'_>,
  window: &mut Window,
  cx: &mut App,
) -> AnyElement {
  let vertical = read.choice("direction") == "vertical";
  let ratio = read.f32("ratio").clamp(0.1, 0.9);
  let border = theme(cx).border().hsla();
  let handle = read.f32("handle_size").max(1.0);

  let first = div()
    .flex()
    .flex_col()
    .when(!vertical, |d| d.w(gpui::relative(ratio)))
    .when(vertical, |d| d.h(gpui::relative(ratio)))
    .children(slot_children(ctx, node, "first", window, cx));
  let divider = div()
    .bg(border)
    .when(!vertical, |d| d.w(px(handle)).h_full())
    .when(vertical, |d| d.h(px(handle)).w_full());
  let second = div()
    .flex()
    .flex_col()
    .flex_grow()
    .children(slot_children(ctx, node, "second", window, cx));

  div()
    .flex()
    .when(!vertical, |d| d.flex_row())
    .when(vertical, |d| d.flex_col())
    .size_full()
    .child(first)
    .child(divider)
    .child(second)
    .into_any_element()
}

/// The app shell's five regions, at the sizes the node carries.
fn appshell(
  ctx: &RenderCtx,
  node: &Node,
  read: &Reader<'_>,
  window: &mut Window,
  cx: &mut App,
) -> AnyElement {
  let border = theme(cx).border().hsla();
  let design = ctx.mode != Mode::Preview;

  // A region with nothing in it is not called at all in generated code, so
  // the canvas must not reserve its width either. In design mode it collapses
  // to a thin strip you can still aim a drop at; in preview it disappears.
  let size_of = |slot: &str, configured: f32| {
    if node.slot(slot).is_empty() {
      if design {
        28.0
      } else {
        0.0
      }
    } else {
      configured
    }
  };
  let header_h = size_of("header", read.f32("header_height"));
  let navbar_w = size_of("navbar", read.f32("navbar_width"));
  let aside_w = size_of("aside", read.f32("aside_width"));
  let footer_h = size_of("footer", read.f32("footer_height"));

  let header = div()
    .flex()
    .items_center()
    .h(px(header_h))
    .w_full()
    .border_b(px(1.))
    .border_color(border)
    .children(slot_children(ctx, node, "header", window, cx));
  let navbar = div()
    .flex()
    .flex_col()
    .w(px(navbar_w))
    .h_full()
    .border_r(px(1.))
    .border_color(border)
    .children(slot_children(ctx, node, "navbar", window, cx));
  let aside = div()
    .flex()
    .flex_col()
    .w(px(aside_w))
    .h_full()
    .border_l(px(1.))
    .border_color(border)
    .children(slot_children(ctx, node, "aside", window, cx));
  let footer = div()
    .flex()
    .items_center()
    .h(px(footer_h))
    .w_full()
    .border_t(px(1.))
    .border_color(border)
    .children(slot_children(ctx, node, "footer", window, cx));
  let body = div().flex().flex_col().flex_grow().children(slot_children(
    ctx,
    node,
    DEFAULT_SLOT,
    window,
    cx,
  ));

  div()
    .flex()
    .flex_col()
    .size_full()
    .overflow_hidden()
    .child(header)
    .child(
      div()
        .flex()
        .flex_row()
        .flex_grow()
        .overflow_hidden()
        .child(navbar)
        .child(body)
        .child(aside),
    )
    .child(footer)
    .into_any_element()
}

/// The carousel's slides, one at a time with the page control below.
fn carousel(ctx: &RenderCtx, node: &Node, window: &mut Window, cx: &mut App) -> AnyElement {
  let slides = node.slot(DEFAULT_SLOT).to_vec();
  let page = ctx
    .store
    .read(cx)
    .page(node.id)
    .min(slides.len().saturating_sub(1));
  let accent = theme(cx).color(ColorName::Blue, 6).hsla();
  let idle = theme(cx).border().hsla();

  let content = if slides.is_empty() {
    chrome::empty_slot("Drop slides here", cx).into_any_element()
  } else {
    super::render_in(ctx, slides[page], false, window, cx)
  };

  let mut dots = div().flex().gap(px(6.)).justify_center().pt(px(8.));
  for index in 0..slides.len() {
    let reveal = ctx.hooks.reveal.clone();
    let id = node.id;
    dots = dots.child(
      div()
        .id(ElementId::Name(SharedString::from(format!(
          "dot-{}-{index}",
          node.id
        ))))
        .size(px(7.))
        .rounded(px(4.))
        .bg(if index == page { accent } else { idle })
        .on_mouse_down(gpui::MouseButton::Left, move |_, _window, cx| {
          cx.stop_propagation();
          reveal(id, index, cx);
        }),
    );
  }

  div()
    .flex()
    .flex_col()
    .child(content)
    .child(dots)
    .into_any_element()
}

fn tool_status(value: &str) -> AIToolStatus {
  match value {
    "pending" => AIToolStatus::Pending,
    "running" => AIToolStatus::Running,
    "error" => AIToolStatus::Error,
    _ => AIToolStatus::Ok,
  }
}

/// A price per million tokens, typed into a text prop. An unreadable one is
/// zero rather than an error: the canvas keeps drawing while you type.
fn price(text: &str) -> f64 {
  text.trim().trim_start_matches('$').parse().unwrap_or(0.0)
}

/// `title — location` on one line, which is how the sources prop is typed.
fn ai_source(line: &str) -> AISource {
  match line
    .split_once('\u{2014}')
    .or_else(|| line.split_once(" - "))
  {
    Some((title, location)) => AISource::new(title.trim(), location.trim()),
    None => AISource::new(line.trim(), ""),
  }
}
