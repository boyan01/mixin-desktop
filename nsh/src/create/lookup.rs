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
                Ok(user) => {
                    busy.set(false);
                    profile.set(Some(user.user_id));
                }
                Err(_) => {
                    busy.set(false);
                    failed.set(true);
                }
            }
        });
    };
    modal(cancel).child(
        panel()
            .style_when(
                move || profile.with(Option::is_some),
                |style| style.h(ui::dialog_height_tall()).p_0(),
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
                        .child(ui::modal_title(move || i18n.text(Key::SearchContact)))
                        .child(
                            scroll().flex_shrink(1.0).min_h_0().child(
                                view()
                                    .gap_3()
                                    .child(
                                        ui::search_pill().flex_none().child(
                                            ui::search_input()
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
                                                .on_composition_change(
                                                    move |event: &Composition| {
                                                        composing.set(event.composing)
                                                    },
                                                )
                                                .on_submit(move |_: &str| search()),
                                        ),
                                    )
                                    .child(when(failed, move || {
                                        ui::error_text(move || i18n.text(Key::SearchFailed))
                                    }))
                                    .child(when(busy, || {
                                        parts::status(move || tr("Searching…", "正在搜索…"))
                                    })),
                            ),
                        )
                        .child(
                            parts::dialog_actions()
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
