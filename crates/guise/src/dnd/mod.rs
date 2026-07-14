//! Drag and drop: [`Draggable`], [`DropTarget`], and [`SortableList`].
//!
//! gpui's drag system is typed — a drag carries a value, and only targets
//! expecting that type react. `Draggable`/`DropTarget` wrap the idiom with a
//! themed drag chip and hover highlight, generically over any `Clone`
//! payload; `SortableList` builds drag-to-reorder rows on top. Component
//! internals (panegroup tabs, table columns) keep their own specialized
//! drags; this module is the reusable surface.

mod chip;
mod draggable;
mod droptarget;
mod sortable;

pub use draggable::Draggable;
pub use droptarget::DropTarget;
pub use sortable::SortableList;

/// Move `items[from]` so it lands at position `to` (the usual list-reorder
/// semantics: remove, then insert). Out-of-range indices are a no-op.
pub fn apply_reorder<T>(items: &mut Vec<T>, from: usize, to: usize) {
    if from == to || from >= items.len() || to >= items.len() {
        return;
    }
    let item = items.remove(from);
    items.insert(to, item);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reorders_forward_and_backward() {
        let mut v = vec!["a", "b", "c", "d"];
        apply_reorder(&mut v, 0, 2);
        assert_eq!(v, ["b", "c", "a", "d"]);
        apply_reorder(&mut v, 3, 0);
        assert_eq!(v, ["d", "b", "c", "a"]);
    }

    #[test]
    fn same_or_out_of_range_is_a_no_op() {
        let mut v = vec![1, 2, 3];
        apply_reorder(&mut v, 1, 1);
        assert_eq!(v, [1, 2, 3]);
        apply_reorder(&mut v, 5, 0);
        assert_eq!(v, [1, 2, 3]);
        apply_reorder(&mut v, 0, 5);
        assert_eq!(v, [1, 2, 3]);
    }

    #[test]
    fn moves_to_the_ends() {
        let mut v = vec![1, 2, 3, 4];
        apply_reorder(&mut v, 1, 3);
        assert_eq!(v, [1, 3, 4, 2]);
        apply_reorder(&mut v, 2, 0);
        assert_eq!(v, [4, 1, 3, 2]);
    }
}
