use super::*;
use crate::presentation;

pub(super) fn list(
    rows: Signal<Vec<Row>>,
    section: Signal<Section>,
    pending: Signal<Option<Pending>>,
    loading: Signal<bool>,
    open: impl Fn(Row) + Copy + 'static,
) -> impl IntoElement {
    let i18n = i18n::current();
    virtual_list(
        move || rows.with(Vec::len),
        move |at| {
            let row = move || rows.with(|rows| rows.get(at.index()).cloned());
            dynamic(move || {
                let value = row();
                untracked(|| match value {
                    Some(Row::Header(kind, more)) => view()
                        .flex_row()
                        .items_center()
                        .px_4()
                        .py_2()
                        .gap_2()
                        .child(
                            text(move || kind.title())
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .flex_1()
                                .text_color(move || theme().muted_foreground),
                        )
                        .child(when(
                            move || section.get() == Section::All && more,
                            move || {
                                ui::command()
                                    .min_h_6()
                                    .py_0()
                                    .px_1()
                                    .on_press(move || open(Row::Header(kind, more)))
                                    .child(text(move || tr("More", "更多")).text_xs())
                            },
                        ))
                        .into_element(),
                    Some(value) => {
                        let content = ui::command()
                            .justify_start()
                            .min_h_20()
                            .gap_3()
                            .mx_2()
                            .px_3()
                            .py_3()
                            .disabled(move || loading.get() || pending.with(Option::is_some))
                            .on_press(move || {
                                if let Some(value) = row() {
                                    open(value);
                                }
                            })
                            .child(ui::avatar_view(
                                move || {
                                    row()
                                        .map(|row| match row {
                                            Row::User(user) => user.full_name,
                                            Row::Conversation(row) => row.name,
                                            Row::Message(message) => message.sender_name,
                                            Row::Header(..) => String::new(),
                                        })
                                        .unwrap_or_default()
                                        .into()
                                },
                                move || {
                                    row()
                                        .map(|row| match row {
                                            Row::User(user) => user.avatar_url,
                                            Row::Conversation(row) => row.avatar_url,
                                            Row::Message(message) => message.sender_avatar_url,
                                            Row::Header(..) => String::new(),
                                        })
                                        .unwrap_or_default()
                                },
                            ))
                            .child(
                                view()
                                    .flex_1()
                                    .min_w_0()
                                    .gap_1()
                                    .child(
                                        text(move || {
                                            row()
                                                .map(|row| match row {
                                                    Row::User(user) => user.full_name,
                                                    Row::Conversation(row) => row.name,
                                                    Row::Message(message) => message.sender_name,
                                                    Row::Header(..) => String::new(),
                                                })
                                                .unwrap_or_default()
                                        })
                                        .text_sm()
                                        .font_weight(FontWeight::MEDIUM)
                                        .number_of_lines(1),
                                    )
                                    .child(
                                        text(move || {
                                            row()
                                                .map(|row| match row {
                                                    Row::User(user) => user.identity_number,
                                                    Row::Conversation(row) => {
                                                        presentation::preview(
                                                            i18n,
                                                            row.last_message_category
                                                                .as_deref()
                                                                .unwrap_or_default(),
                                                            &row.last_message,
                                                        )
                                                    }
                                                    Row::Message(message) => presentation::preview(
                                                        i18n,
                                                        &message.category,
                                                        &message.content,
                                                    ),
                                                    Row::Header(..) => String::new(),
                                                })
                                                .unwrap_or_default()
                                        })
                                        .text_sm()
                                        .number_of_lines(2)
                                        .text_color(move || theme().muted_foreground),
                                    ),
                            )
                            .into_element();
                        if matches!(value, Row::Conversation(_)) {
                            crate::conversation_list::row_menu(
                                use_context::<crate::conversation_list::Commands>()
                                    .expect("Home provides conversation commands"),
                                move || match row() {
                                    Some(Row::Conversation(value)) => Some(*value),
                                    _ => None,
                                },
                                content,
                            )
                            .into_element()
                        } else {
                            content
                        }
                    }
                    None => view().hidden().into_element(),
                })
            })
        },
    )
    .keys(move || rows.with(|rows| rows.iter().map(Row::key).collect::<Vec<_>>()))
    .estimated_item_size(px(80.0))
    .flex_1()
    .min_h_0()
}
