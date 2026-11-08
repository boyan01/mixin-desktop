//! Pending states. Their spinners never settle, so this scene has no actions;
//! capture it without `--action`.

use nsh_core::prelude::*;

use super::Actions;
use crate::ui::{self, parts};

pub(super) const ACTIONS: &[&str] = &[];

pub(super) fn scene(_actions: Actions) -> View {
    view()
        .flex_1()
        .p(ui::space_lg())
        .gap(ui::space_md())
        .child(ui::busy_indicator(|| true, || "Loading".into()))
        .child(parts::status(|| "Loading…"))
}
