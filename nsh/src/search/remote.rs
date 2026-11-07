use mixin_desktop_api::UserProfileItem;
use url::Url;

use super::*;

pub(super) fn banner(
    account: Arc<AccountClient>,
    query: Signal<String>,
    composing: Signal<bool>,
    users_empty: Signal<bool>,
    profile: Signal<Option<String>>,
) -> impl IntoElement {
    let account = stored(account);
    let mao = signal_with(None::<UserProfileItem>, SignalEquals::Never);
    let failed = signal(false);
    let busy = signal(false);
    let request = signal(0_u64);
    let router =
        use_context::<crate::links::LinkNavigation>().expect("Home provides link navigation");
    effect(move || {
        if composing.get() {
            return;
        }
        let keyword = query.get().trim().to_owned();
        let ticket = request.get_untracked().wrapping_add(1);
        request.set(ticket);
        mao.set(None);
        failed.set(false);
        if !valid_mao(&keyword) || Url::parse(&keyword).is_ok() {
            return;
        }
        let client = account.with(|account| account.clone());
        spawn(async move {
            timer(std::time::Duration::from_millis(150)).await;
            let result =
                runtime::run(async move { client.user().search_mao_user(keyword).await }).await;
            if request.get_untracked() == ticket
                && let Ok(value) = result
            {
                mao.set(value);
            }
        });
    });
    let numeric = move || {
        query.with(|query| {
            let query = query.trim();
            !query.is_empty() && query.chars().all(|value| value.is_ascii_digit())
        })
    };
    let url = move || {
        query.with(|query| {
            Url::parse(query.trim())
                .ok()
                .filter(|url| matches!(url.scheme(), "https" | "http"))
        })
    };
    let lookup = move || {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        failed.set(false);
        let client = account.with(|account| account.clone());
        let keyword = query.get_untracked().trim().to_owned();
        let ticket = request.get_untracked();
        spawn(async move {
            let result =
                runtime::run(async move { client.user().search_user(keyword).await }).await;
            busy.set(false);
            if request.get_untracked() != ticket {
                return;
            }
            match result {
                Ok(user) => profile.set(Some(user.user_id)),
                Err(_) => failed.set(true),
            }
        });
    };
    view()
        .gap_1()
        .child(when(
            move || mao.with(Option::is_some),
            move || {
                ui::command()
                    .justify_start()
                    .px_4()
                    .py_3()
                    .gap_3()
                    .on_press(move || {
                        if let Some(user) = mao
                            .with_untracked(|user| user.as_ref().map(|user| user.user_id.clone()))
                        {
                            profile.set(Some(user));
                        }
                    })
                    .child(ui::avatar_view(
                        move || {
                            mao.with(|user| {
                                user.as_ref()
                                    .map(|user| user.full_name.clone())
                                    .unwrap_or_default()
                                    .into()
                            })
                        },
                        move || {
                            mao.with(|user| {
                                user.as_ref()
                                    .map(|user| user.avatar_url.clone())
                                    .unwrap_or_default()
                            })
                        },
                    ))
                    .child(
                        view()
                            .flex_1()
                            .min_w_0()
                            .gap_1()
                            .child(
                                text(move || {
                                    mao.with(|user| {
                                        user.as_ref()
                                            .map(|user| user.full_name.clone())
                                            .unwrap_or_default()
                                    })
                                })
                                .text_sm()
                                .number_of_lines(1),
                            )
                            .child(
                                text(move || complete_mao(query.get().trim()))
                                    .text_xs()
                                    .text_color(move || theme().muted_foreground),
                            ),
                    )
            },
        ))
        .child(when(
            move || users_empty.get() && numeric(),
            move || {
                ui::command()
                    .justify_start()
                    .px_4()
                    .py_3()
                    .disabled(busy)
                    .on_press(lookup)
                    .child(
                        text(move || {
                            format!(
                                "{} {}",
                                tr("Search Mixin ID", "搜索 Mixin ID"),
                                query.get().trim()
                            )
                        })
                        .text_sm(),
                    )
            },
        ))
        .child(when(
            move || users_empty.get() && url().is_some(),
            move || {
                ui::command()
                    .justify_start()
                    .px_4()
                    .py_3()
                    .disabled(busy)
                    .on_press(move || {
                        if let Some(url) = url() {
                            router.open(url.to_string(), None);
                        }
                    })
                    .child(
                        text(move || {
                            format!("{} {}", tr("Open link", "打开链接"), query.get().trim())
                        })
                        .text_sm()
                        .number_of_lines(2),
                    )
            },
        ))
        .child(when(failed, || {
            text(move || {
                tr(
                    "Couldn’t open this result. Check it and try again.",
                    "无法打开此结果，请检查后重试。",
                )
            })
            .px_4()
            .text_sm()
            .text_color(move || theme().destructive)
        }))
}

fn valid_mao(keyword: &str) -> bool {
    let candidate = keyword.trim_end_matches('.');
    !candidate.is_empty()
        && candidate.chars().count() <= 128
        && !candidate
            .chars()
            .all(|character| character.is_ascii_digit())
        && !candidate
            .chars()
            .any(|character| character.is_whitespace() || character.is_ascii_uppercase())
}

fn complete_mao(keyword: &str) -> String {
    if keyword.ends_with(".mao") {
        keyword.into()
    } else if keyword.ends_with(".ma") {
        format!("{keyword}o")
    } else if keyword.ends_with(".m") {
        format!("{keyword}ao")
    } else if keyword.ends_with('.') {
        format!("{keyword}mao")
    } else {
        format!("{keyword}.mao")
    }
}
