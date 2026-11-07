use mixin_desktop_api::{AccountClient, SnapshotDetailItem};
use nsh_base::theme;
use nsh_core::prelude::*;
use std::sync::Arc;

use crate::{
    i18n::{self, Key},
    presentation, runtime, ui,
};

#[derive(Clone)]
pub(crate) enum Reference {
    Snapshot(String),
    Safe(String),
    Trace(String),
}

fn label(i18n: i18n::I18n, en: &'static str, zh: &'static str) -> &'static str {
    i18n.tr(en, zh)
}

pub(crate) fn screen(account: Arc<AccountClient>, reference: Reference) -> impl IntoElement {
    let i18n = i18n::current();
    let detail = signal_with(None::<SnapshotDetailItem>, SignalEquals::Never);
    let loading = signal(true);
    let failed = signal(false);
    let retry = signal(0_u64);
    let reference = stored(reference);
    let preview_account = account.clone();
    effect(move || {
        retry.get();
        let reference = reference.with(|reference| reference.clone());
        let client = account.clone();
        loading.set(true);
        failed.set(false);
        spawn(async move {
            let result = runtime::run(async move {
                match reference {
                    Reference::Snapshot(id) => client.snapshot_by_id(id).await,
                    Reference::Safe(id) => client.safe_snapshot_by_id(id).await,
                    Reference::Trace(trace) => client.snapshot_by_trace(trace).await,
                }
            })
            .await;
            match result {
                Ok(value) => detail.set(Some(value)),
                Err(_) => failed.set(true),
            }
            loading.set(false);
        });
    });
    scroll().flex_1().min_h_0().child(
        view()
            .w_full()
            .max_w(px(640.0))
            .p_6()
            .gap_4()
            .child(
                text(move || {
                    if detail.with(|value| {
                        value
                            .as_ref()
                            .is_some_and(|value| value.inscription_hash.is_some())
                    }) {
                        i18n.tr("Inscription", "铭文")
                    } else {
                        i18n.text(Key::Transfer)
                    }
                })
                .text_lg()
                .font_weight(FontWeight::SEMIBOLD),
            )
            .child(when(loading, move || {
                text(move || label(i18n, "Loading transfer…", "正在加载转账…"))
            }))
            .child(when(failed, move || {
                view()
                    .gap_2()
                    .child(
                        text(move || i18n.text(Key::CommandFailed))
                            .text_color(move || theme().destructive),
                    )
                    .child(ui::action(
                        move || i18n.text(Key::Retry),
                        move || retry.update(|v| *v += 1),
                    ))
            }))
            .child(dynamic(move || {
                let value = detail.get();
                untracked(|| {
                    let Some(value) = value else {
                        return view().hidden().into_element();
                    };
                    let inscription_text = value.inscription_hash.clone().filter(|_| {
                        value
                            .inscription_content_type
                            .as_deref()
                            .is_some_and(|kind| kind.starts_with("text/"))
                    });
                    let inscription_image = value.inscription_content_url.clone().filter(|_| {
                        value
                            .inscription_content_type
                            .as_deref()
                            .is_some_and(|kind| kind.starts_with("image/"))
                    });
                    let inscription_icon = value
                        .inscription_icon_url
                        .clone()
                        .filter(|url| !url.is_empty());
                    let content_url = value.inscription_content_url.clone().filter(|raw| {
                        url::Url::parse(raw)
                            .is_ok_and(|url| matches!(url.scheme(), "https" | "http"))
                    });
                    let memo = if value.is_safe {
                        decode_memo(value.memo.as_deref().unwrap_or_default())
                    } else {
                        value.memo.clone().unwrap_or_default()
                    };
                    let mut fields = vec![
                        (
                            label(i18n, "Amount", "金额"),
                            format!("{} {}", value.amount, value.symbol),
                        ),
                        (label(i18n, "Asset", "资产"), value.asset_name),
                        (
                            label(i18n, "Date", "时间"),
                            presentation::day(i18n, value.created_at_millis * 1000),
                        ),
                        (label(i18n, "Transfer ID", "转账 ID"), value.snapshot_id),
                    ];
                    for (title, content) in [
                        (label(i18n, "Sender", "发送者"), value.sender),
                        (label(i18n, "Recipient", "接收者"), value.receiver),
                        (label(i18n, "Contact", "联系人"), value.opponent_name),
                        (
                            label(i18n, "Transaction hash", "交易哈希"),
                            value.transaction_hash,
                        ),
                        (label(i18n, "Memo", "备注"), Some(memo)),
                    ] {
                        if let Some(content) = content.filter(|s| !s.is_empty()) {
                            fields.push((title, content));
                        }
                    }
                    for (title, content) in [
                        (i18n.tr("Inscription", "铭文"), value.inscription_name),
                        (
                            i18n.tr("Inscription hash", "铭文哈希"),
                            value.inscription_hash,
                        ),
                        (
                            i18n.tr("Collection hash", "合集哈希"),
                            value.inscription_collection_hash,
                        ),
                        (
                            i18n.tr("Sequence", "序号"),
                            value
                                .inscription_sequence
                                .map(|sequence| format!("#{sequence}")),
                        ),
                        (
                            i18n.tr("Content type", "内容类型"),
                            value.inscription_content_type,
                        ),
                    ] {
                        if let Some(content) = content.filter(|value| !value.is_empty()) {
                            fields.push((title, content));
                        }
                    }
                    if let Some(confirmations) = value.confirmations {
                        fields.push((
                            label(i18n, "Confirmations", "确认数"),
                            confirmations.to_string(),
                        ));
                    }
                    let mut content = view().gap_4();
                    if let Some(hash) = inscription_text {
                        content = content.child(text_content(preview_account.clone(), hash));
                    }
                    if let Some(url) = inscription_image {
                        content = content.child(
                            image(url)
                                .w(px(260.0))
                                .h(px(260.0))
                                .max_w(relative(1.0))
                                .object_fit(ResizeMode::Contain)
                                .label(move || i18n.tr("Inscription", "铭文")),
                        );
                    }
                    if let Some(url) = inscription_icon {
                        content = content.child(
                            image(url)
                                .size_10()
                                .object_fit(ResizeMode::Contain)
                                .label(move || i18n.tr("Collection", "合集")),
                        );
                    }
                    if let Some(url) = content_url {
                        content = content.child(ui::action(
                            move || i18n.tr("Open content", "打开内容"),
                            move || {
                                let url = url.clone();
                                spawn(async move {
                                    failed.set(!open_url(url).await);
                                });
                            },
                        ));
                    }
                    content
                        .children(fields.into_iter().map(move |(title, value)| {
                            view()
                                .gap_1()
                                .child(
                                    text(title)
                                        .text_xs()
                                        .text_color(move || theme().muted_foreground),
                                )
                                .child(text(value).text_sm().select_text())
                        }))
                        .into_element()
                })
            })),
    )
}

fn decode_memo(raw: &str) -> String {
    if !raw.len().is_multiple_of(2) {
        return raw.to_owned();
    }
    let bytes = raw
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let hi = char::from(pair[0]).to_digit(16)?;
            let lo = char::from(pair[1]).to_digit(16)?;
            Some(((hi << 4) | lo) as u8)
        })
        .collect::<Option<Vec<_>>>();
    bytes
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .unwrap_or_else(|| raw.to_owned())
}

/// Inscription documents are rendered as selectable plain text, including HTML.
fn text_content(account: Arc<AccountClient>, hash: String) -> impl IntoElement {
    let i18n = i18n::current();
    let content = signal(None::<SharedString>);
    let loading = signal(true);
    let failed = signal(false);
    let truncated = signal(false);
    let retry = signal(0_u64);
    let hash = stored(hash);
    effect(move || {
        retry.get();
        let account = account.clone();
        let hash = hash.with(|hash| hash.clone());
        loading.set(true);
        failed.set(false);
        truncated.set(false);
        spawn(async move {
            match runtime::run(async move { account.inscription_content(hash).await }).await {
                Ok(value) => {
                    let value = value.map(|value| {
                        let mut characters = value.chars();
                        let preview = characters.by_ref().take(8192).collect::<String>();
                        truncated.set(characters.next().is_some());
                        inscription_display_content(&preview).into()
                    });
                    content.set(value);
                }
                Err(_) => failed.set(true),
            }
            loading.set(false);
        });
    });
    view()
        .gap_2()
        .child(
            text(move || i18n.tr("Text content", "文本内容"))
                .text_xs()
                .text_color(move || theme().muted_foreground),
        )
        .child(when(loading, move || {
            text(move || i18n.tr("Loading content…", "正在加载内容…"))
                .text_sm()
                .text_color(move || theme().muted_foreground)
        }))
        .child(when(failed, move || {
            view()
                .gap_2()
                .child(
                    text(move || i18n.tr("Couldn’t load content", "无法加载内容"))
                        .text_sm()
                        .text_color(move || theme().destructive),
                )
                .child(ui::action(
                    move || i18n.text(Key::Retry),
                    move || retry.update(|value| *value += 1),
                ))
        }))
        .child(when(
            move || !loading.get() && !failed.get(),
            move || {
                view()
                    .gap_2()
                    .child(
                        text(move || {
                            content.get().unwrap_or_else(|| {
                                i18n.tr("Content unavailable", "内容不可用").into()
                            })
                        })
                        .text_sm()
                        .select_text(),
                    )
                    .child(when(truncated, move || {
                        text(move || {
                            i18n.tr(
                                "Showing the first 8,192 characters",
                                "仅显示前 8,192 个字符",
                            )
                        })
                        .text_xs()
                        .text_color(move || theme().muted_foreground)
                    }))
            },
        ))
}

fn inscription_display_content(content: &str) -> String {
    static INVISIBLE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"[\s\p{Other}]").expect("valid inscription display pattern")
    });
    // Preserve Flutter's visible placeholder for whitespace and Unicode Other.
    INVISIBLE.replace_all(content, "■").into_owned()
}
