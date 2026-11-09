use mixin_desktop_api::{AccountClient, SnapshotDetailItem};
use nsh_core::prelude::*;
use std::sync::Arc;

use crate::{
    i18n::{self, Key},
    runtime,
    ui::{self, Typography as _},
};

#[derive(Clone)]
pub(crate) enum Reference {
    Snapshot(String),
    Safe(String),
    Trace(String),
}

pub(crate) fn screen(
    account: Arc<AccountClient>,
    reference: Reference,
    close: impl Fn() + Copy + 'static,
    back: impl IntoElement,
) -> impl IntoElement {
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
    let inscription =
        stored(
            Box::new(move |hash| text_content(preview_account.clone(), hash).into_element())
                as Box<dyn Fn(String) -> AnyElement>,
        );
    content(detail, loading, failed, retry, inscription, close, back)
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
    text_view(content, loading, failed, truncated, retry)
}

fn inscription_display_content(content: &str) -> String {
    static INVISIBLE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"[\s\p{Other}]").expect("valid inscription display pattern")
    });
    // Preserve Flutter's visible placeholder for whitespace and Unicode Other.
    INVISIBLE.replace_all(content, "■").into_owned()
}

pub(crate) fn load_error(retry: impl Fn() + 'static) -> View {
    let i18n = crate::i18n::current();
    view()
        .gap_2()
        .child(ui::error_text(move || i18n.text(Key::CommandFailed)))
        .child(ui::link(move || i18n.text(Key::Retry), retry))
}

pub(crate) fn content_error(retry: impl Fn() + 'static) -> View {
    let i18n = crate::i18n::current();
    view()
        .gap_2()
        .child(ui::error_text(move || {
            i18n.tr("Couldn’t load content", "无法加载内容")
        }))
        .child(ui::link(move || i18n.text(Key::Retry), retry))
}

/// The same compact detail surface is used by chat, links and native previews.
pub(crate) fn surface<S: Styled>(style: S) -> S {
    ui::dialog_surface(style, ui::DialogWidth::Regular)
        .h(ui::dialog_height_tall())
        .p_0()
        .gap_0()
        .overflow_hidden()
}

fn content(
    detail: Signal<Option<SnapshotDetailItem>>,
    loading: Signal<bool>,
    failed: Signal<bool>,
    retry: Signal<u64>,
    inscription: Stored<Box<dyn Fn(String) -> AnyElement>>,
    close: impl Fn() + Copy + 'static,
    back: impl IntoElement,
) -> impl IntoElement {
    let i18n = i18n::current();
    let open_failed = signal(false);
    view()
        .flex_1()
        .min_h_0()
        .child(
            view()
                .flex_row()
                .items_center()
                .px_6()
                .py_3()
                .gap_3()
                .child(back)
                .child(
                    ui::dialog_title(move || {
                        if detail.with(|value| {
                            value.as_ref().is_some_and(|v| v.inscription_hash.is_some())
                        }) {
                            i18n.tr("Inscription details", "铭文详情")
                        } else {
                            i18n.tr("Transfer details", "转账详情")
                        }
                    })
                    .flex_1()
                    .min_w_0(),
                )
                .child(ui::dialog_close(close).m_0()),
        )
        .child(
            scroll().flex_1().min_h_0().child(
                view()
                    .w_full()
                    .pb_6()
                    .child(when(loading, move || {
                        view().px_6().py_8().child(
                            text(move || i18n.tr("Loading details…", "正在加载详情…"))
                                .text_role(ui::TextRole::Body)
                                .text_color(ui::fg_secondary),
                        )
                    }))
                    .child(when(failed, move || {
                        view()
                            .px_6()
                            .py_4()
                            .child(load_error(move || retry.update(|v| *v += 1)))
                    }))
                    .child(dynamic(move || {
                        let value = detail.get();
                        untracked(|| {
                            let Some(value) = value else {
                                return view().hidden().into_element();
                            };
                            if value.inscription_hash.is_some() {
                                inscription_detail(value, inscription, open_failed).into_element()
                            } else {
                                transfer_detail(value).into_element()
                            }
                        })
                    }))
                    .child(when(open_failed, move || {
                        view().px_6().pt_3().child(ui::error_text(move || {
                            i18n.tr("Couldn’t open content", "无法打开内容")
                        }))
                    })),
            ),
        )
}

fn field(title: &'static str, value: String, copy: bool) -> View {
    let i18n = i18n::current();
    let value = stored(value);
    view()
        .min_w_0()
        .gap_1()
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    text(title)
                        .flex_1()
                        .min_w_0()
                        .text_role(ui::TextRole::Caption)
                        .text_color(ui::fg_secondary),
                )
                .child(copy.then(|| {
                    ui::round_button()
                        .size_6()
                        .accessibility_label(move || {
                            Some(format!("{} {title}", i18n.tr("Copy", "复制")).into())
                        })
                        .on_press(move || set_clipboard(value.get()))
                        .child(
                            ui::icon(ui::Icon::Copy)
                                .size(ui::icon_sm())
                                .text_color(ui::fg_secondary),
                        )
                })),
        )
        .child(
            text(move || value.get())
                .w_full()
                .min_w_0()
                .text_role(ui::TextRole::Body)
                .select_text(),
        )
}

fn optional_field(content: View, title: &'static str, value: Option<String>, copy: bool) -> View {
    match value.filter(|value| !value.is_empty()) {
        Some(value) => content.child(field(title, value, copy)),
        None => content,
    }
}

fn transfer_detail(value: SnapshotDetailItem) -> View {
    let i18n = i18n::current();
    let kind = value.snapshot_type.as_str();
    let pending = kind == "pending";
    let outgoing = value.amount.trim_start().starts_with('-');
    let mut summary = view()
        .items_center()
        .px_6()
        .pt_3()
        .pb_6()
        .gap_3()
        .child(asset_icon(&value))
        .child(
            view()
                .flex_row()
                .items_baseline()
                .justify_center()
                .flex_wrap()
                .gap_2()
                .w_full()
                .child(
                    text(value.amount.clone())
                        .min_w_0()
                        .flex_shrink(1.0)
                        .text_center()
                        .text_size(ui::detail_amount_size())
                        .text_emphasis()
                        .text_color(move || {
                            if pending {
                                ui::fg_primary()
                            } else if outgoing {
                                ui::danger()
                            } else {
                                ui::success()
                            }
                        })
                        .select_text(),
                )
                .child(
                    text(value.symbol.clone())
                        .text_role(ui::TextRole::Body)
                        .select_text(),
                ),
        );
    // Price estimates are display-only USD values; the exact asset amount stays a string.
    for (title, price) in [
        (i18n.tr("Value now", "当前价值"), value.price_usd.as_deref()),
        (
            i18n.tr("Value then", "当时价值"),
            value.ticker_price_usd.as_deref(),
        ),
    ] {
        if let Some(estimate) = usd_value(&value.amount, price) {
            summary = summary.child(
                text(format!("{title} {estimate} USD"))
                    .text_role(ui::TextRole::Caption)
                    .text_color(ui::fg_secondary)
                    .text_center(),
            );
        }
    }
    let mut fields = view().px_6().pt_5().gap_4();
    if kind == "pending" {
        fields = fields.child(field(
            i18n.tr("Status", "状态"),
            format!(
                "{} · {} / {}",
                i18n.tr("Pending confirmation", "等待确认"),
                value.confirmations.unwrap_or(0),
                value.asset_confirmations
            ),
            false,
        ));
    }
    fields = fields.child(field(
        i18n.tr("Transaction ID", "交易编号"),
        value.snapshot_id,
        true,
    ));
    fields = optional_field(
        fields,
        i18n.tr("Transaction hash", "交易哈希"),
        value.transaction_hash,
        true,
    );
    fields = optional_field(
        fields,
        i18n.tr("Snapshot hash", "快照哈希"),
        value.snapshot_hash,
        true,
    );
    if !value.is_safe {
        fields = fields.child(field(i18n.tr("Asset", "资产"), value.asset_name, false));
    }
    match kind {
        "transfer" if value.is_safe => {
            fields = optional_field(
                fields,
                if outgoing {
                    i18n.tr("To", "转给")
                } else {
                    i18n.tr("From", "来自")
                },
                value.opponent_name.or(value.opponent_id),
                false,
            );
        }
        "transfer" => {
            let (sender, receiver) = if outgoing {
                (
                    Some(value.current_user_name),
                    value.opponent_name.or(value.receiver),
                )
            } else {
                (
                    value.opponent_name.or(value.sender),
                    Some(value.current_user_name),
                )
            };
            fields = optional_field(fields, i18n.tr("From", "来自"), sender, false);
            fields = optional_field(fields, i18n.tr("To", "转给"), receiver, false);
        }
        "withdrawal" => {
            fields = optional_field(
                fields,
                i18n.tr("To", "转给"),
                value.withdrawal_receiver.or(value.receiver),
                true,
            );
            fields = optional_field(
                fields,
                i18n.tr("Withdrawal hash", "提现哈希"),
                value.withdrawal_hash,
                true,
            );
        }
        _ => {
            fields = optional_field(fields, i18n.tr("From", "来自"), value.sender, true);
            fields = optional_field(fields, i18n.tr("To", "转给"), value.receiver, true);
        }
    }
    fields = optional_field(
        fields,
        i18n.tr("Deposit hash", "充值哈希"),
        value.deposit_hash,
        true,
    );
    let memo = value.memo.map(|memo| {
        if value.is_safe {
            decode_memo(&memo)
        } else {
            memo
        }
    });
    fields = optional_field(fields, i18n.tr("Memo", "备注"), memo, true);
    fields = optional_field(
        fields,
        i18n.tr("Opening balance", "期初余额"),
        value.opening_balance,
        false,
    );
    fields = optional_field(
        fields,
        i18n.tr("Closing balance", "期末余额"),
        value.closing_balance,
        false,
    );
    let time = chrono::DateTime::from_timestamp_millis(value.created_at_millis)
        .map(|date| {
            date.with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        })
        .unwrap_or_default();
    fields = fields.child(field(i18n.tr("Time", "时间"), time, false));
    view()
        .child(summary)
        .child(view().h_2().bg(ui::surface_chat))
        .child(fields)
}

fn usd_value(amount: &str, price: Option<&str>) -> Option<String> {
    let estimate = amount.parse::<f64>().ok()?.abs() * price?.parse::<f64>().ok()?;
    (estimate.is_finite() && estimate >= 0.0).then(|| format!("${estimate:.2}"))
}

fn asset_icon(value: &SnapshotDetailItem) -> View {
    let icon = value.asset_icon_url.clone();
    let failed = signal(icon.is_empty());
    let mut frame = view()
        .size_16()
        .relative()
        .flex_none()
        .child(
            view()
                .absolute()
                .inset_0()
                .rounded_full()
                .bg(ui::surface_chat)
                .items_center()
                .justify_center()
                .child(
                    text(value.symbol.chars().next().unwrap_or(' ').to_string())
                        .text_role(ui::TextRole::Headline),
                ),
        )
        .child(when(
            move || !failed.get(),
            move || {
                image(icon.clone())
                    .size_16()
                    .rounded_full()
                    .object_fit(ResizeMode::Cover)
                    .on_error(move |_: &str| failed.set(true))
                    .label("Asset")
            },
        ));
    if !value.chain_icon_url.is_empty() {
        frame = frame.child(
            image(value.chain_icon_url.clone())
                .absolute()
                .bottom_0()
                .right_0()
                .size_5()
                .rounded_full()
                .border_2()
                .border_color(ui::surface_popover)
                .object_fit(ResizeMode::Cover)
                .label("Chain"),
        );
    }
    frame
}

fn inscription_detail(
    value: SnapshotDetailItem,
    inscription: Stored<Box<dyn Fn(String) -> AnyElement>>,
    open_failed: Signal<bool>,
) -> View {
    let i18n = i18n::current();
    let hash = value.inscription_hash.unwrap_or_default();
    let kind = value
        .inscription_content_type
        .as_deref()
        .unwrap_or_default();
    let mut artwork = view()
        .w_full()
        .aspect_ratio(1.0)
        .rounded(ui::radius_lg())
        .overflow_hidden()
        .bg(ui::surface_chat)
        .relative();
    if kind.starts_with("text/") {
        artwork = artwork.child(
            ui::inscription_background()
                .absolute()
                .inset_0()
                .w_full()
                .h_full()
                .object_fit(ResizeMode::Cover),
        );
        let mut inner = view()
            .absolute()
            .inset_0()
            .p_6()
            .items_center()
            .justify_center()
            .gap_4();
        inner = inner.child(collection_icon(value.inscription_icon_url.clone()).size_20());
        artwork = artwork.child(inner.child(inscription.with(|render| render(hash.clone()))));
    } else {
        let failed = signal(!kind.starts_with("image/"));
        let url = value.inscription_content_url.clone().unwrap_or_default();
        artwork = artwork
            .child(when(failed, || {
                svg(nsh_core::element::ImageSrc::svg_markup(include_str!(
                    "../assets/inscription/inscription_placeholder.svg"
                )))
                .absolute()
                .inset_0()
                .w_full()
                .h_full()
            }))
            .child(when(
                move || !failed.get(),
                move || {
                    image(url.clone())
                        .absolute()
                        .inset_0()
                        .w_full()
                        .h_full()
                        .object_fit(ResizeMode::Contain)
                        .on_error(move |_: &str| failed.set(true))
                        .label("Inscription")
                },
            ));
    }
    let mut fields = view()
        .px_6()
        .pt_5()
        .gap_4()
        .child(field(i18n.tr("Hash", "哈希"), hash, true));
    if let Some(sequence) = value.inscription_sequence {
        fields = fields.child(field(i18n.tr("ID", "编号"), format!("#{sequence}"), false));
    }
    let mut collection = view().gap_2().child(
        text(i18n.tr("Collection", "合集"))
            .text_role(ui::TextRole::Caption)
            .text_color(ui::fg_secondary),
    );
    collection = collection.child(
        view()
            .flex_row()
            .items_center()
            .gap_3()
            .child(collection_icon(value.inscription_icon_url.clone()))
            .child(
                text(
                    value
                        .inscription_name
                        .unwrap_or_else(|| i18n.tr("Unknown collection", "未知合集").into()),
                )
                .flex_1()
                .min_w_0()
                .text_role(ui::TextRole::Body)
                .select_text(),
            ),
    );
    fields = fields.child(collection);
    fields = optional_field(
        fields,
        i18n.tr("Collection hash", "合集哈希"),
        value.inscription_collection_hash,
        true,
    );
    fields = optional_field(
        fields,
        i18n.tr("Content type", "内容类型"),
        value.inscription_content_type,
        false,
    );
    if let Some(url) = value.inscription_content_url.filter(|raw| {
        url::Url::parse(raw).is_ok_and(|url| matches!(url.scheme(), "https" | "http"))
    }) {
        fields = fields.child(
            ui::button(ui::ButtonTone::Ghost, ui::ButtonSize::Small)
                .self_start()
                .on_press(move || {
                    let url = url.clone();
                    spawn(async move {
                        open_failed.set(!open_url(url).await);
                    });
                })
                .child(ui::icon(ui::Icon::Globe).size(ui::icon_sm()))
                .child(text(move || i18n.tr("Open content", "打开内容"))),
        );
    }
    view()
        .child(view().px_6().pt_2().child(artwork))
        .child(fields)
}
fn collection_icon(source: Option<String>) -> View {
    let source = source.filter(|value| !value.is_empty());
    let failed = signal(source.is_none());
    view()
        .size_10()
        .relative()
        .flex_none()
        .rounded(ui::radius_md())
        .overflow_hidden()
        .child(
            svg(nsh_core::element::ImageSrc::svg_markup(include_str!(
                "../assets/inscription/collection_placeholder.svg"
            )))
            .absolute()
            .inset_0()
            .w_full()
            .h_full(),
        )
        .child(when(
            move || !failed.get(),
            move || {
                image(source.clone().unwrap_or_default())
                    .absolute()
                    .inset_0()
                    .w_full()
                    .h_full()
                    .object_fit(ResizeMode::Contain)
                    .on_error(move |_: &str| failed.set(true))
                    .label("Collection")
            },
        ))
}

#[cfg(debug_assertions)]
pub(crate) fn preview(detail: SnapshotDetailItem, state: Signal<&'static str>) -> impl IntoElement {
    let failed = signal(false);
    let content_failed = signal(false);
    let details = signal_with(Some(detail.clone()), SignalEquals::Never);
    effect(move || {
        let mut value = detail.clone();
        match state.get() {
            "Text" | "Long text" | "Content error" => {
                value.inscription_content_type = Some("text/plain".into());
                value.inscription_icon_url =
                    Some(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/logo.png").into());
            }
            "Image" => {
                value.inscription_content_type = Some("image/jpeg".into());
                value.inscription_content_url =
                    Some(concat!(env!("CARGO_MANIFEST_DIR"), "/preview/fixtures/photo.jpg").into());
            }
            "Sent" => value.amount = "-0.0393".into(),
            "Pending" => {
                value.snapshot_type = "pending".into();
                value.confirmations = Some(3);
                value.asset_confirmations = 12;
            }
            "Deposit" => {
                value.snapshot_type = "deposit".into();
                value.deposit_hash = value.transaction_hash.clone();
            }
            "Withdrawal" => {
                value.amount = "-0.0393".into();
                value.snapshot_type = "withdrawal".into();
                value.withdrawal_receiver =
                    Some("0x0d1b3a7e5f9123456789abcdef0123456789abcdef".into());
                value.withdrawal_hash = value.transaction_hash.clone();
            }
            "Legacy" => {
                value.is_safe = false;
                value.opening_balance = Some("1.20393".into());
                value.closing_balance = Some("1.24323".into());
            }
            _ => {}
        }
        details.set(Some(value));
        failed.set(state.get() == "Error");
        content_failed.set(state.get() == "Content error");
    });

    content(
        details,
        signal(false),
        failed,
        signal(0),
        stored(Box::new(move |_| {
            text_view(
                signal(Some(if state.get_untracked() == "Long text" {
                    "Long inscription content 中文内容 ".repeat(64).into()
                } else {
                    "web3.mao".into()
                })),
                signal(false),
                content_failed,
                signal(false),
                signal(0),
            )
            .into_element()
        }) as Box<dyn Fn(String) -> AnyElement>),
        || {},
        view().hidden(),
    )
}

fn text_view(
    content: Signal<Option<SharedString>>,
    loading: Signal<bool>,
    failed: Signal<bool>,
    truncated: Signal<bool>,
    retry: Signal<u64>,
) -> View {
    let i18n = i18n::current();
    view()
        .gap_2()
        .child(when(loading, move || {
            text(move || i18n.tr("Loading content…", "正在加载内容…"))
                .text_role(ui::TextRole::Body)
                .text_color(ui::viewer_secondary)
        }))
        .child(when(failed, move || {
            view()
                .p_3()
                .rounded(ui::radius_md())
                .bg(ui::surface_popover)
                .child(content_error(move || retry.update(|value| *value += 1)))
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
                        .text_role(ui::TextRole::Headline)
                        .text_color(ui::inscription_ink)
                        .text_center()
                        .number_of_lines(3)
                        .select_text(),
                    )
                    .child(when(truncated, move || {
                        text(move || {
                            i18n.tr(
                                "Showing the first 8,192 characters",
                                "仅显示前 8,192 个字符",
                            )
                        })
                        .text_role(ui::TextRole::Caption)
                        .text_color(ui::fg_secondary)
                    }))
            },
        ))
}
