//! Video — a surface for frames the host decodes.
//!
//! Like `ai/`, this is transport-agnostic on purpose: guise opens no socket,
//! links no codec and owns no clock. The host decodes (ffmpeg, a camera API, a
//! WebRTC track, a screen capture) and hands over pixels; [`VideoView`] uploads
//! the newest one through gpui's GPU image path and fits it into its box.
//!
//! - [`VideoFrame`] — one decoded picture, from RGBA, BGRA or planar I420.
//! - [`VideoView`] — the entity that shows frames. `push_frame` from the UI
//!   thread, or `feed` for a [`VideoFeed`] a decoder thread can send into.
//! - [`VideoFit`] — how a frame maps into the box (`Contain`/`Cover`/`Stretch`).
//!
//! It is a picture surface, not a player: play/pause/seek belong to whatever is
//! decoding, and controls are ordinary components around the view.

mod feed;
mod fit;
mod frame;
mod view;

pub use feed::VideoFeed;
pub use fit::{fit_rect, VideoFit};
pub use frame::VideoFrame;
pub use view::{VideoEvent, VideoStats, VideoView};
