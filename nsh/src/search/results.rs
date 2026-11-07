use super::*;
use crate::presentation;

/// What a recycled cell draws; a row of the same shape only rebinds its text.
#[derive(Clone, Copy, PartialEq)]
enum Shape {
    Header(Section, bool),
    Conversation,
    Item,
    Missing,
}

pub(super) fn list(
    state: query::State,
    pending: Signal<bool>,
    open: impl Fn(Row) + Copy + 'static,
) -> impl IntoElement {
    let rows = state.rows;
    virtual_list(
        move || rows.with(Vec::len),
        move |at| {
            let row = move || rows.with(|rows| rows.get(at.index()).cloned());
            let shape = computed(move || {
                rows.with(|rows| match rows.get(at.index()) {
                    Some(Row::Header(section, more)) => Shape::Header(*section, *more),
                    Some(Row::Conversation(_)) => Shape::Conversation,
                    Some(_) => Shape::Item,
                    None => Shape::Missing,
                })
            });
            dynamic(move || {
                let shape = shape.get();
                untracked(|| match shape {
                    Shape::Header(section, more) => {
                        header(state, section, more, open).into_element()
                    }
                    Shape::Conversation => crate::conversation_list::row_menu(
                        use_context().expect("Home provides conversation commands"),
                        move || match row() {
                            Some(Row::Conversation(value)) => Some(*value),
                            _ => None,
                        },
                        item(row, pending, open),
                    )
                    .into_element(),
                    Shape::Item => item(row, pending, open).into_element(),
                    Shape::Missing => view().hidden().into_element(),
                })
            })
        },
    )
    .keys(move || rows.with(|rows| rows.iter().map(Row::key).collect::<Vec<_>>()))
    .element_ref(&state.list)
    .estimated_item_size(ui::list_row_height())
    .flex_1()
    .min_h_0()
}

fn header(
    state: query::State,
    section: Section,
    more: bool,
    open: impl Fn(Row) + Copy + 'static,
) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .pr(ui::space_lg())
        .py(ui::space_sm())
        .gap(ui::space_sm())
        .child(ui::section_label(move || section.title()).flex_1())
        .child(when(
            move || state.section.get() == Section::All && more,
            move || {
                ui::button(ui::ButtonTone::Ghost, ui::ButtonSize::Small)
                    .on_press(move || open(Row::Header(section, more)))
                    .child(text(move || tr("More", "更多")).text_role(ui::TextRole::Caption))
            },
        ))
}

fn item(
    row: impl Fn() -> Option<Row> + Copy + 'static,
    pending: Signal<bool>,
    open: impl Fn(Row) + Copy + 'static,
) -> impl IntoElement {
    let i18n = i18n::current();
    ui::list_row()
        .disabled(pending)
        .on_press(move || {
            if let Some(value) = row() {
                open(value);
            }
        })
        .child(ui::avatar_view(
            move || row().map(|row| row.title()).unwrap_or_default().into(),
            move || row().map(|row| row.avatar_url()).unwrap_or_default(),
            ui::AvatarSize::Medium,
        ))
        .child(
            view()
                .flex_1()
                .min_w_0()
                .gap_1()
                .child(
                    text(move || row().map(|row| row.title()).unwrap_or_default())
                        .text_role(ui::TextRole::Label)
                        .number_of_lines(1),
                )
                .child(
                    text(move || {
                        row()
                            .map(|row| match row {
                                Row::User(user) => user.identity_number,
                                Row::Conversation(row) => presentation::preview(
                                    i18n,
                                    row.last_message_category.as_deref().unwrap_or_default(),
                                    &row.last_message,
                                ),
                                Row::Message(message) => {
                                    presentation::preview(i18n, &message.category, &message.content)
                                }
                                Row::Header(..) => String::new(),
                            })
                            .unwrap_or_default()
                    })
                    .text_role(ui::TextRole::Body)
                    .number_of_lines(2)
                    .text_color(ui::fg_secondary),
                ),
        )
}
