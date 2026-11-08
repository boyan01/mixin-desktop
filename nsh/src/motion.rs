//! Product motion policy. Native values run on the engine's frame clock.

use nsh_core::{
    PointerEvents, TransformKind,
    animation::Easing,
    element::{AnimatedValue, StyleRefinement},
    prelude::*,
};

pub(crate) const FEEDBACK_MS: f32 = 120.0;

/// A pane that grows or shrinks: it and the panes beside it ease to their new frames.
pub(crate) const PANEL_WIDTH: nsh_core::LayoutTransition =
    nsh_core::LayoutTransition::new(nsh_core::LayoutCurve::EaseOut, 220.0);

/// Folding selected attachments into one ZIP summary.
pub(crate) const ATTACHMENT_FOLD_MS: f32 = 180.0;
pub(crate) const ATTACHMENT_FOLD: nsh_core::LayoutTransition =
    nsh_core::LayoutTransition::new(nsh_core::LayoutCurve::EaseOut, ATTACHMENT_FOLD_MS);

pub(crate) fn duration(ms: f32) -> f32 {
    if reduce_motion() { ms.min(80.0) } else { ms }
}

pub(crate) fn popup(style: StyleRefinement) -> StyleRefinement {
    style.transition().duration(|| duration(100.0)).ease_out()
}

pub(crate) fn popup_start(style: StyleRefinement) -> StyleRefinement {
    style.opacity(0.0).translate_y(popup_offset)
}

pub(crate) fn popup_end(style: StyleRefinement) -> StyleRefinement {
    style
        .opacity(0.0)
        .translate_y(popup_offset)
        .duration(|| duration(80.0))
}

fn popup_offset() -> Points {
    if reduce_motion() {
        Points::ZERO
    } else {
        crate::ui::space_xs()
    }
}

#[derive(Clone, Copy)]
enum Kind {
    Modal,
    Panel,
    Bottom,
    Fade,
}

impl Kind {
    fn enter_ms(self) -> f32 {
        match self {
            Self::Modal | Self::Panel | Self::Bottom => 180.0,
            Self::Fade => 100.0,
        }
    }

    fn exit_ms(self) -> f32 {
        match self {
            Self::Modal => 120.0,
            Self::Panel | Self::Bottom => 140.0,
            Self::Fade => 80.0,
        }
    }

    fn offset(self) -> f32 {
        match self {
            Self::Modal => 8.0,
            Self::Panel | Self::Bottom => 12.0,
            Self::Fade => 0.0,
        }
    }

    /// The scale a surface enters from and leaves to.
    fn hidden_scale(self, reduced: bool) -> f32 {
        if reduced || !matches!(self, Self::Modal) {
            1.0
        } else {
            0.985
        }
    }
}

/// Create once outside the conditional region; use `mounted` for that region.
/// The caller changes business/open state immediately. Only the presentation
/// remains until the native animation completes. Disposing the owner ends both.
#[derive(Clone, Copy)]
pub(crate) struct Presence {
    mounted: Signal<bool>,
    interactive: Signal<bool>,
    opacity: AnimatedValue,
    offset: AnimatedValue,
    scale: AnimatedValue,
    backdrop: AnimatedValue,
    kind: Kind,
}

impl Presence {
    pub(crate) fn modal(open: impl Fn() -> bool + 'static) -> Self {
        Self::new(open, Kind::Modal)
    }

    pub(crate) fn panel(open: impl Fn() -> bool + 'static) -> Self {
        Self::new(open, Kind::Panel)
    }

    pub(crate) fn bottom(open: impl Fn() -> bool + 'static) -> Self {
        Self::new(open, Kind::Bottom)
    }

    pub(crate) fn fade(open: impl Fn() -> bool + 'static) -> Self {
        Self::new(open, Kind::Fade)
    }

    fn new(open: impl Fn() -> bool + 'static, kind: Kind) -> Self {
        let presence = Self {
            mounted: signal(false),
            interactive: signal(false),
            opacity: animated_value(0.0),
            offset: animated_value(if reduce_motion() { 0.0 } else { kind.offset() }),
            scale: animated_value(kind.hidden_scale(reduce_motion())),
            backdrop: animated_value(0.0),
            kind,
        };
        // A content/position update with the same open value never replays entry.
        let open = computed(open);
        let generation = stored(0_u64);
        effect(move || {
            let open = open.get();
            let reduced = reduce_motion();
            untracked(move || {
                let change = generation.get().wrapping_add(1);
                generation.replace(change);
                presence.interactive.set(open);
                if !open && !presence.mounted.get_untracked() {
                    return;
                }
                if open {
                    if !presence.mounted.get_untracked() {
                        presence
                            .offset
                            .set(if reduced { 0.0 } else { kind.offset() });
                        presence.scale.set(kind.hidden_scale(reduced));
                    }
                    presence.mounted.set(true);
                }
                let ms = duration(if open {
                    kind.enter_ms()
                } else {
                    kind.exit_ms()
                });
                let target = if open { 1.0 } else { 0.0 };
                let offset = if open || reduced { 0.0 } else { kind.offset() };
                let scale = if open {
                    1.0
                } else {
                    kind.hidden_scale(reduced)
                };
                // Timing starts from the native value currently being sampled.
                // Setting transforms also stops a run if reduced motion changes.
                if reduced {
                    presence.offset.set(0.0);
                    presence.scale.set(1.0);
                } else {
                    let _ = presence.offset.timing(offset, ms, Easing::EaseOut);
                    let _ = presence.scale.timing(scale, ms, Easing::EaseOut);
                }
                let _ = presence.backdrop.timing(
                    target,
                    duration(if open { 140.0 } else { kind.exit_ms() }),
                    Easing::EaseOut,
                );
                let end = presence.opacity.timing(target, ms, Easing::EaseOut);
                if !open {
                    spawn(async move {
                        if end.await && generation.get() == change {
                            presence.mounted.set(false);
                        }
                    });
                }
            });
        });
        presence
    }

    pub(crate) fn mounted(self) -> bool {
        self.mounted.get()
    }

    pub(crate) fn interactive(self) -> bool {
        self.interactive.get()
    }

    pub(crate) fn surface(self, view: View) -> View {
        view.bind_opacity(&self.opacity)
            .disabled(move || !self.interactive())
            .accessibility_elements_hidden(move || !self.interactive())
            .bind_transform(
                if matches!(self.kind, Kind::Panel) {
                    TransformKind::TranslateX
                } else {
                    TransformKind::TranslateY
                },
                &self.offset,
            )
            .bind_transform(TransformKind::Scale, &self.scale)
            .pointer_events(move || {
                if self.interactive() {
                    PointerEvents::Auto
                } else {
                    PointerEvents::None
                }
            })
    }

    /// Bind to a separate backdrop view, so the card does not inherit its fade.
    pub(crate) fn backdrop(self, view: View) -> View {
        view.bind_opacity(&self.backdrop)
            .accessibility_elements_hidden(move || !self.interactive())
            .pointer_events(move || {
                if self.interactive() {
                    PointerEvents::Auto
                } else {
                    PointerEvents::None
                }
            })
    }
}
