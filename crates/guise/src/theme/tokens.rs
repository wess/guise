//! Sizing tokens: the `xs..xl` scale used for spacing, radius, and font size,
//! matching Mantine's defaults (authored in px).

/// A named size on the `xs..xl` scale. The library default is `Md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Size {
    Xs,
    Sm,
    Md,
    Lg,
    Xl,
}

impl Default for Size {
    fn default() -> Self {
        Size::Md
    }
}

/// Five px values addressed by [`Size`]. Used for spacing, radius and font.
#[derive(Debug, Clone, Copy)]
pub struct Scale {
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
}

impl Scale {
    pub const fn new(xs: f32, sm: f32, md: f32, lg: f32, xl: f32) -> Self {
        Scale { xs, sm, md, lg, xl }
    }

    /// Resolve a [`Size`] to its px value.
    pub fn get(&self, size: Size) -> f32 {
        match size {
            Size::Xs => self.xs,
            Size::Sm => self.sm,
            Size::Md => self.md,
            Size::Lg => self.lg,
            Size::Xl => self.xl,
        }
    }

    pub fn spacing() -> Self {
        Scale::new(10.0, 12.0, 16.0, 20.0, 32.0)
    }

    pub fn radius() -> Self {
        Scale::new(2.0, 4.0, 8.0, 16.0, 32.0)
    }

    pub fn font_size() -> Self {
        Scale::new(12.0, 14.0, 16.0, 18.0, 20.0)
    }
}
