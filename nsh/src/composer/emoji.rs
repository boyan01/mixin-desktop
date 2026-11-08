use emojis::{Emoji, Group, SkinTone};
use mixin_desktop_api::SettingsClient;
use nsh_core::prelude::*;
use serde_json::Value;
use std::{collections::HashMap, rc::Rc};
use tokio::sync::watch;

use super::{
    stickers::{TabLabel, columns, panel_tab},
    tr,
};
use crate::{
    i18n::{self, Key},
    runtime,
    ui::{self, Typography as _},
};

const RECENT_LIMIT: usize = 64;
const TONES: [SkinTone; 6] = [
    SkinTone::Default,
    SkinTone::Light,
    SkinTone::MediumLight,
    SkinTone::Medium,
    SkinTone::MediumDark,
    SkinTone::Dark,
];
/// Section 0 is the recent emoji; sections 1.. are `SECTIONS`.
const RECENT: usize = 0;
/// Cell slots a recycled row owns; the panel never fits more.
const MAX_COLUMNS: usize = 12;

fn section_title(section: usize, i18n: i18n::I18n) -> &'static str {
    match section {
        RECENT => tr(i18n, "Recently used", "最近使用"),
        1 => tr(i18n, "Smileys and people", "表情符号与人物"),
        2 => tr(i18n, "Animals and nature", "动物与自然"),
        3 => tr(i18n, "Food and drink", "食物与饮料"),
        4 => tr(i18n, "Travel and places", "旅行与地点"),
        5 => tr(i18n, "Activities", "活动"),
        6 => tr(i18n, "Objects", "物品"),
        7 => tr(i18n, "Symbols", "符号"),
        _ => tr(i18n, "Flags", "旗帜"),
    }
}

/// The groups each section (1..) lists, in order.
const SECTIONS: [&[Group]; 8] = [
    &[Group::SmileysAndEmotion, Group::PeopleAndBody],
    &[Group::AnimalsAndNature],
    &[Group::FoodAndDrink],
    &[Group::TravelAndPlaces],
    &[Group::Activities],
    &[Group::Objects],
    &[Group::Symbols],
    &[Group::Flags],
];

/// One line of the list: a section title, or a row of emoji.
#[derive(Clone, PartialEq)]
enum Row {
    Title(usize),
    Cells(usize, Vec<&'static Emoji>),
}

impl Row {
    fn section(&self) -> usize {
        match self {
            Self::Title(section) | Self::Cells(section, _) => *section,
        }
    }
}

fn base(emoji: &'static Emoji) -> &'static Emoji {
    emoji.with_skin_tone(SkinTone::Default).unwrap_or(emoji)
}

#[derive(Clone, Copy)]
struct Preferences {
    recent: Signal<Vec<String>>,
    /// The tone chosen per emoji, by its default-tone character.
    skins: Signal<HashMap<String, usize>>,
    loaded: Signal<bool>,
    failed: Signal<bool>,
    retry: Signal<u64>,
    writes: Stored<watch::Sender<Option<String>>>,
}
impl Preferences {
    fn mount(settings: SettingsClient, key: String) -> Self {
        let preferences = Self {
            recent: signal(Vec::new()),
            skins: signal(HashMap::new()),
            loaded: signal(false),
            failed: signal(false),
            retry: signal(0),
            writes: stored(writer(settings.clone(), key.clone())),
        };
        let settings = stored(settings);
        let key = stored(key);
        effect(move || {
            preferences.retry.get();
            let settings = settings.with(|settings| settings.clone());
            let key = key.with(|key| key.clone());
            preferences.failed.set(false);
            spawn(async move {
                match runtime::run(async move { settings.setting(key).await }).await {
                    Ok(value) => {
                        preferences.apply(value);
                        preferences.loaded.set(true);
                    }
                    Err(_) => preferences.failed.set(true),
                }
            });
        });
        preferences
    }

    fn apply(self, stored: Option<String>) {
        let Some(value) = stored.and_then(|raw| serde_json::from_str::<Value>(&raw).ok()) else {
            return;
        };
        self.recent.set(
            value["recent"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .take(RECENT_LIMIT)
                .map(str::to_owned)
                .collect(),
        );
        self.skins.set(
            value["skins"]
                .as_object()
                .into_iter()
                .flatten()
                .filter_map(|(key, tone)| {
                    let tone = tone.as_u64().filter(|v| (*v as usize) < TONES.len())?;
                    Some((key.clone(), tone as usize))
                })
                .collect(),
        );
    }

    fn remember(self, emoji: String) {
        self.recent.update(|recent| {
            recent.retain(|value| value != &emoji);
            recent.insert(0, emoji);
            recent.truncate(RECENT_LIMIT);
        });
        self.save();
    }

    /// What `emoji` shows: the tone chosen for it, if any.
    fn shown(self, emoji: &'static Emoji) -> &'static Emoji {
        let base = base(emoji);
        self.skins.with(|skins| {
            skins
                .get(base.as_str())
                .and_then(|tone| base.with_skin_tone(TONES[*tone]))
                .unwrap_or(base)
        })
    }

    /// Never overwrites preferences that could not be read.
    fn save(self) {
        if !self.loaded.get_untracked() {
            return;
        }
        let value = serde_json::json!({
            "recent": self.recent.get_untracked(),
            "skins": self.skins.get_untracked(),
        })
        .to_string();
        self.writes.with(|writes| writes.send_replace(Some(value)));
    }
}

/// One ordered writer keeps only the newest value, so quick picks never race.
/// Disposing the panel closes the channel after its last value is written.
fn writer(settings: SettingsClient, key: String) -> watch::Sender<Option<String>> {
    let (sender, mut values) = watch::channel(None);
    drop(runtime::commit(async move {
        while values.changed().await.is_ok() {
            let value = values.borrow_and_update().clone();
            if let Err(error) = settings.set_setting(key.clone(), value).await {
                log::warn!("Could not save emoji preferences: {error}");
            }
        }
        Ok(())
    }));
    sender
}

pub(super) fn panel(insert: Rc<dyn Fn(String)>, account_id: String) -> impl IntoElement {
    let i18n = i18n::current();
    let settings = crate::settings_client();
    let preferences =
        Preferences::mount(settings, format!("desktop_emoji_preferences_{account_id}"));
    let query = signal(String::new());
    let searching = signal(false);
    // The emoji whose skin tones the top bar offers.
    let picking = signal_with(None::<&'static Emoji>, SignalEquals::Never);
    let active = signal(1usize);
    let history = element_ref();
    // A glyph of `icon_lg` with `space_sm` around it.
    let cell = ui::icon_lg() + ui::space_lg();
    // The list's real width, less a gutter for its scroll bar, decides the columns.
    let width = signal(columns(cell.0) as f32 * cell.0 + ui::space_md().0);
    let rows = computed(move || {
        let per_row =
            (((width.get() - ui::space_md().0) / cell.0).floor() as usize).clamp(1, MAX_COLUMNS);
        let query = query.get().to_lowercase();
        let mut rows = Vec::new();
        let mut push = |section: usize, title: bool, items: Vec<&'static Emoji>| {
            if title && !items.is_empty() {
                rows.push(Row::Title(section));
            }
            rows.extend(
                items
                    .chunks(per_row)
                    .map(|chunk| Row::Cells(section, chunk.to_vec())),
            );
        };
        if !query.is_empty() {
            let found = emojis::iter()
                .filter(|e| {
                    e.name().to_lowercase().contains(&query)
                        || e.shortcodes().any(|code| code.contains(&query))
                })
                .map(|e| preferences.shown(e))
                .collect();
            push(1, false, found);
            return rows;
        }
        let recent = preferences.recent.with(|recent| {
            recent
                .iter()
                .filter_map(|character| emojis::get(character))
                .collect()
        });
        push(RECENT, true, recent);
        for (index, groups) in SECTIONS.iter().enumerate() {
            let items = groups
                .iter()
                .flat_map(|group| group.emojis())
                .map(|e| preferences.shown(e))
                .collect();
            push(index + 1, true, items);
        }
        rows
    });
    let jump = move |section: usize| {
        let at = rows.with(|rows| {
            rows.iter()
                .position(|row| matches!(row, Row::Title(s) if *s == section))
        });
        if let Some(at) = at {
            history.scroll_to_index(at as u32, percentage(0.0), Points::ZERO, false);
        }
        active.set(section);
    };
    let insert = stored(insert);
    let pick = move |emoji: &'static Emoji| {
        insert.with(|insert| insert(emoji.as_str().to_owned()));
        preferences.remember(emoji.as_str().to_owned());
    };
    // Long press or right click offers the tones of an emoji that has them.
    let offer = move |emoji: &'static Emoji| {
        if emoji.skin_tones().is_some() {
            picking.set(Some(emoji));
        }
    };
    // A row owns fixed slots and only rebinds them when recycled, so scrolling
    // rebuilds no elements.
    let emoji_cell = move |emoji: Memo<Option<&'static Emoji>>| {
        view()
            .on_long_press(move || {
                if let Some(emoji) = emoji.get_untracked() {
                    offer(emoji)
                }
            })
            .on_context_menu(move |_: &ContextMenuEvent| {
                if let Some(emoji) = emoji.get_untracked() {
                    offer(emoji)
                }
            })
            .style_when(move || emoji.get().is_none(), |style| style.hidden())
            .child(
                ui::command()
                    .size(cell)
                    .px_1()
                    .py_1()
                    .accessibility_label(move || {
                        Some(emoji.get().map(Emoji::name).unwrap_or_default().into())
                    })
                    .child(
                        text(move || {
                            emoji
                                .get()
                                .map(|e| e.as_str().to_owned())
                                .unwrap_or_default()
                        })
                        .text_size(ui::icon_lg()),
                    )
                    .on_press(move || {
                        if let Some(emoji) = emoji.get_untracked() {
                            pick(emoji)
                        }
                    }),
            )
    };
    let tabs_bar = move || {
        view()
            .flex_row()
            .items_center()
            .children(
                [
                    (RECENT, TabLabel::Icon(ui::Icon::Clock)),
                    (1, TabLabel::Text("😀")),
                    (2, TabLabel::Text("🐻")),
                    (3, TabLabel::Text("🍔")),
                    (4, TabLabel::Text("🚙")),
                    (5, TabLabel::Text("⚽")),
                    (6, TabLabel::Text("💡")),
                    (7, TabLabel::Text("❤️")),
                    (8, TabLabel::Text("🏳️")),
                ]
                .into_iter()
                .map(move |(section, label)| {
                    panel_tab()
                        .flex_1()
                        .min_w_0()
                        .accessibility_label(move || Some(section_title(section, i18n).into()))
                        .selected(move || active.get() == section)
                        .on_press(move || jump(section))
                        .child(label.element())
                        .style_when(
                            move || section == RECENT && preferences.recent.with(Vec::is_empty),
                            |style| style.hidden(),
                        )
                }),
            )
            .child(
                panel_tab()
                    .flex_1()
                    .min_w_0()
                    .accessibility_label(move || Some(tr(i18n, "Search emoji", "搜索表情").into()))
                    .on_press(move || searching.set(true))
                    .child(ui::icon(ui::Icon::Search)),
            )
            .into_element()
    };
    let search_bar = move || {
        view()
            .flex_row()
            .items_center()
            .gap_1()
            .child(
                ui::field()
                    .flex_1()
                    .auto_focus(true)
                    .label(move || tr(i18n, "Search emoji", "搜索表情"))
                    .placeholder(move || tr(i18n, "Search emoji…", "搜索表情…").to_owned())
                    .value(move || query.get())
                    .on_change(move |value: &str| query.set(value.to_owned())),
            )
            .child(
                panel_tab()
                    .accessibility_label(move || Some(tr(i18n, "Close", "关闭").into()))
                    .on_press(move || {
                        query.set(String::new());
                        searching.set(false);
                    })
                    .child(ui::icon(ui::Icon::Close)),
            )
            .into_element()
    };
    let tone_bar = move |emoji: &'static Emoji| {
        let default = base(emoji);
        view()
            .flex_row()
            .items_center()
            .children(
                TONES
                    .into_iter()
                    .enumerate()
                    .filter_map(move |(index, tone)| {
                        let variant = default.with_skin_tone(tone)?;
                        Some(
                            panel_tab()
                                .flex_1()
                                .min_w_0()
                                .accessibility_label(move || Some(variant.name().into()))
                                .selected(move || preferences.shown(default) == variant)
                                .child(text(variant.as_str()))
                                .on_press(move || {
                                    preferences.skins.update(|skins| {
                                        skins.insert(default.as_str().to_owned(), index);
                                    });
                                    pick(variant);
                                    picking.set(None);
                                }),
                        )
                    }),
            )
            .child(
                panel_tab()
                    .accessibility_label(move || Some(tr(i18n, "Close", "关闭").into()))
                    .on_press(move || picking.set(None))
                    .child(ui::icon(ui::Icon::Close)),
            )
            .into_element()
    };
    view()
        .flex_1()
        .min_h_0()
        .gap_2()
        .child(dynamic(move || {
            let picked = picking.get();
            let search = searching.get();
            untracked(|| match (picked, search) {
                (Some(emoji), _) => tone_bar(emoji),
                (None, true) => search_bar(),
                (None, false) => tabs_bar(),
            })
        }))
        .child(when(preferences.failed, move || {
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    ui::error_text(move || {
                        tr(
                            i18n,
                            "Couldn’t load recent emoji.",
                            "无法加载最近使用的表情。",
                        )
                    })
                    .flex_1(),
                )
                .child(ui::action(
                    move || i18n.text(Key::Retry),
                    move || preferences.retry.update(|value| *value += 1),
                ))
        }))
        .child(
            virtual_list(
                move || rows.with(Vec::len),
                move |at| {
                    let row = computed(move || rows.with(|rows| rows.get(at.index()).cloned()));
                    let title = computed(move || match row.get() {
                        Some(Row::Title(section)) => Some(section),
                        _ => None,
                    });
                    view()
                        .flex_row()
                        .child(
                            text(move || {
                                title
                                    .get()
                                    .map(|section| section_title(section, i18n))
                                    .unwrap_or_default()
                            })
                            .text_role(ui::TextRole::Body)
                            .text_color(ui::fg_secondary)
                            .px_1()
                            .pt_2()
                            .pb_1()
                            .style_when(move || title.get().is_none(), |style| style.hidden()),
                        )
                        .children((0..MAX_COLUMNS).map(|slot| {
                            emoji_cell(computed(move || match row.get() {
                                Some(Row::Cells(_, cells)) => cells.get(slot).copied(),
                                _ => None,
                            }))
                        }))
                },
            )
            .keys(move || {
                rows.with(|rows| {
                    rows.iter()
                        .map(|row| match row {
                            Row::Title(section) => format!("t{section}"),
                            Row::Cells(section, cells) => format!("{section}{}", cells[0].as_str()),
                        })
                        .collect::<Vec<_>>()
                })
            })
            .element_ref(&history)
            .on_layout(move |bounds: &Bounds<Points>| width.set(bounds.size.width.0))
            .on_viewable_items_change(move |event: &ViewableItems| {
                if query.with(String::is_empty)
                    && let Some((first, _)) = event.viewable
                    && let Some(section) = rows.with(|rows| rows.get(first).map(Row::section))
                {
                    active.set(section);
                }
            })
            .estimated_item_size(cell)
            .flex_1()
            .min_h_0()
            .style_when(move || rows.with(Vec::is_empty), |style| style.hidden()),
        )
        .child(when(
            move || rows.with(Vec::is_empty),
            move || ui::placeholder(move || tr(i18n, "No emoji found", "未找到表情")),
        ))
}
