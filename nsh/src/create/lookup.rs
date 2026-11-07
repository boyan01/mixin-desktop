use super::*;
use crate::runtime;

pub(super) fn screen(
    account: Arc<AccountClient>,
    close: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let account = stored(account);
    let query = signal(String::new());
    let composing = signal(false);
    let profile = signal(None::<String>);
    let busy = signal(false);
    let failed = signal(false);
    let i18n = i18n::current();
    let presence =
        use_context::<crate::motion::Presence>().expect("Creation surface provides motion");
    let cancel = move || {
        if presence.interactive() && !busy.get_untracked() && !composing.get_untracked() {
            close();
        }
    };
    let search = move || {
        if !presence.interactive() {
            return;
        }
        if busy.get_untracked()
            || composing.get_untracked()
            || query.with_untracked(|query| query.trim().is_empty())
        {
            return;
        }
        busy.set(true);
        failed.set(false);
        let client = account.with(|account| account.clone());
        let query = query.get_untracked().trim().to_owned();
        spawn(async move {
            match runtime::run(async move { client.user().search_user(query).await }).await {
                Ok(user) => profile.set(Some(user.user_id)),
                Err(_) => failed.set(true),
            }
            busy.set(false);
        });
    };
    modal(cancel).child(
        panel()
            .h(auto())
            .min_h_0()
            .overflow_hidden()
            .style_when(
                move || profile.with(Option::is_some),
                |style| style.w(px(340.0)).h(px(560.0)).p_0(),
            )
            .child(dynamic(move || {
                let user_id = profile.get();
                untracked(|| match user_id {
                    Some(user_id) => crate::conversation::profile_with_opened(
                        account.with(|account| account.clone()),
                        user_id,
                        move || profile.set(None),
                        close,
                    )
                    .into_element(),
                    None => view()
                        .flex_shrink(1.0)
                        .min_h_0()
                        .gap_4()
                        .child(
                            text(move || i18n.text(Key::SearchContact))
                                .flex_none()
                                .text_lg()
                                .font_weight(FontWeight::SEMIBOLD),
                        )
                        .child(
                            scroll().flex_shrink(1.0).min_h_0().child(
                                view()
                                    .gap_3()
                                    .child(
                                        ui::field()
                                            .label(move || i18n.text(Key::SearchContact))
                                            .placeholder(move || {
                                                i18n.text(Key::SearchUserHint).to_owned()
                                            })
                                            .auto_focus(true)
                                            .readonly(busy)
                                            .value(move || query.get())
                                            .on_key_press(move |key: &str| {
                                                if key == "Escape" {
                                                    cancel();
                                                }
                                            })
                                            .on_change(move |value: &str| {
                                                query.set(value.into());
                                                failed.set(false);
                                            })
                                            .on_composition_change(move |event: &Composition| {
                                                composing.set(event.composing)
                                            })
                                            .on_submit(move |_: &str| search()),
                                    )
                                    .child(when(failed, move || {
                                        text(move || i18n.text(Key::SearchFailed))
                                            .text_sm()
                                            .text_color(move || theme().destructive)
                                    }))
                                    .child(when(busy, move || {
                                        view()
                                            .flex_row()
                                            .items_center()
                                            .gap_2()
                                            .text_color(move || theme().muted_foreground)
                                            .child(ui::busy_indicator(
                                                move || busy.get(),
                                                move || tr("Searching…", "正在搜索…").into(),
                                            ))
                                            .child(
                                                text(move || tr("Searching…", "正在搜索…"))
                                                    .text_sm(),
                                            )
                                    })),
                            ),
                        )
                        .child(
                            view()
                                .flex_none()
                                .flex_row()
                                .justify_end()
                                .gap_2()
                                .child(
                                    ui::action(move || i18n.text(Key::Cancel), cancel)
                                        .disabled(move || busy.get() || composing.get()),
                                )
                                .child(
                                    ui::primary()
                                        .disabled(move || {
                                            busy.get()
                                                || composing.get()
                                                || query.with(|query| query.trim().is_empty())
                                        })
                                        .on_press(search)
                                        .child(text(move || i18n.text(Key::Search))),
                                ),
                        )
                        .into_element(),
                })
            })),
    )
}
