//! The right-click Cut / Copy / Paste / Select All menu every text surface
//! opens.
//!
//! Built fresh on each right-click, so it offers only what applies at that
//! moment: no Cut or Copy without a selection or in a masked field, no Cut or
//! Paste when the field is read-only. [`ContextMenu`] has no disabled items,
//! so leaving an entry out is how "not now" is said.
//!
//! The items don't touch the field. Each one sends the matching
//! [`actions`](crate::actions) action to the field's focus handle, so the
//! menu, the keyboard, and a host's Edit menu all run the same code.

use gpui::{Action, AppContext as _, Context, Entity, FocusHandle, Pixels, Point, Window};

use crate::actions;
use crate::icon::IconName;
use crate::overlay::ContextMenu;

/// What the menu should offer, decided by the field at the moment of the
/// click.
pub(crate) struct EditMenu {
  pub cut: bool,
  pub copy: bool,
  pub paste: bool,
}

impl EditMenu {
  /// The usual rules: copying needs a selection and an unmasked field,
  /// cutting also needs it to be editable, pasting only needs that.
  pub fn new(selection: bool, masked: bool, read_only: bool) -> Self {
    EditMenu {
      cut: selection && !masked && !read_only,
      copy: selection && !masked,
      paste: !read_only,
    }
  }
}

/// Build the menu, open it at `position`, and put it in `slot` for the field
/// to render. The field renders `slot` inside itself; the menu paints
/// deferred, so the field's clipping doesn't reach it.
pub(crate) fn open<V: 'static>(
  slot: &mut Option<Entity<ContextMenu>>,
  menu: EditMenu,
  focus: &FocusHandle,
  position: Point<Pixels>,
  window: &mut Window,
  cx: &mut Context<V>,
) {
  let send = |action: Box<dyn Action>| {
    let focus = focus.clone();
    move |window: &mut Window, cx: &mut gpui::App| {
      focus.dispatch_action(action.as_ref(), window, cx)
    }
  };
  let entity = cx.new(|cx| {
    let mut m = ContextMenu::new(cx).width(180.0);
    if menu.cut {
      m = m.item_icon(IconName::Scissors, "Cut", send(Box::new(actions::Cut)));
    }
    if menu.copy {
      m = m.item_icon(IconName::Copy, "Copy", send(Box::new(actions::Copy)));
    }
    if menu.paste {
      m = m.item_icon(
        IconName::ClipboardPaste,
        "Paste",
        send(Box::new(actions::Paste)),
      );
    }
    if menu.cut || menu.copy || menu.paste {
      m = m.divider();
    }
    m.item_icon(
      IconName::TextSelect,
      "Select All",
      send(Box::new(actions::SelectAll)),
    )
  });
  entity.update(cx, |m, cx| m.show(position, window, cx));
  *slot = Some(entity);
}

/// Whether a field's menu is open, in which case the field ignores keys: the
/// menu holds focus, but it is rendered inside the field, so its keys bubble
/// there.
pub(crate) fn is_open(slot: &Option<Entity<ContextMenu>>, cx: &gpui::App) -> bool {
  slot.as_ref().is_some_and(|m| m.read(cx).is_open())
}

/// The slot, wrapped so it takes no room: absolutely positioned, it is no
/// flex item and adds no gap.
pub(crate) fn slot(slot: &Option<Entity<ContextMenu>>) -> Option<gpui::Div> {
  use gpui::{div, ParentElement, Styled};
  slot.clone().map(|m| div().absolute().child(m))
}
