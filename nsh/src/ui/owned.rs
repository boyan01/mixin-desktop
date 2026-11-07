//! A real component boundary for Rust-only feature roots and their list context.
use nsh_core::prelude::*;

#[derive(Clone, PartialEq, Props)]
struct OwnedViewProps {
    render: Prop<Option<Renderer<bool>>>,
}

#[component]
fn OwnedView(props: OwnedViewProps) -> impl IntoElement {
    props
        .render
        .get()
        .expect("An owned feature has a renderer")
        .render("content", false)
        .flex_1()
        .min_w_0()
        .min_h_0()
}

/// Virtual cells inherit their nearest component's context, rather than the
/// dynamic binding that constructed a plain layout helper. Keep that context
/// and the feature's disposable state under the same actual component owner.
/// The renderer runs once; its key must change with the feature's route identity.
pub(crate) fn owned<E: IntoElement>(
    key: impl Into<SharedString>,
    render: impl Fn() -> E + 'static,
) -> impl IntoElement {
    owned_view().key(key).render(move |_: &bool| render())
}
