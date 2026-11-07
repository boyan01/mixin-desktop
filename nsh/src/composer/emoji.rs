use mixin_desktop_api::SettingsClient;
use nsh_core::prelude::*;
use serde_json::Value;
use std::{rc::Rc, sync::OnceLock};
use tokio::sync::watch;

use super::{
    stickers::{TabLabel, columns, panel_tab},
    tr,
};
use crate::{
    i18n::{self, Key},
    runtime, ui,
};

const RECENT_LIMIT: usize = 64;

struct Emoji {
    character: String,
    name: String,
    group: String,
    search: String,
    variants: Vec<String>,
}
fn dictionary() -> &'static [Emoji] {
    static DATA: OnceLock<Vec<Emoji>> = OnceLock::new();
    DATA.get_or_init(|| {
        let value: Value = serde_json::from_str(include_str!("../../assets/emoji.json"))
            .expect("bundled emoji dictionary");
        value["emoji"]
            .as_array()
            .expect("emoji list")
            .iter()
            .map(|v| {
                let string = |key| v[key].as_str().unwrap_or_default().to_owned();
                Emoji {
                    character: string("char"),
                    name: string("name"),
                    group: string("group"),
                    search: format!(
                        "{} {} {}",
                        string("name"),
                        string("short"),
                        v["keywords"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(" ")
                    )
                    .to_lowercase(),
                    variants: v["variants"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect(),
                }
            })
            .collect()
    })
}

#[derive(Clone, Copy)]
struct Preferences {
    recent: Signal<Vec<String>>,
    skin: Signal<usize>,
    loaded: Signal<bool>,
    failed: Signal<bool>,
    retry: Signal<u64>,
    writes: Stored<watch::Sender<Option<String>>>,
}
impl Preferences {
    fn mount(settings: SettingsClient, key: String) -> Self {
        let preferences = Self {
            recent: signal(Vec::new()),
            skin: signal(0),
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
        self.skin.set(
            value["skin"]
                .as_u64()
                .filter(|v| *v < 6)
                .unwrap_or_default() as usize,
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

    /// Never overwrites preferences that could not be read.
    fn save(self) {
        if !self.loaded.get_untracked() {
            return;
        }
        let value = serde_json::json!({
            "recent": self.recent.get_untracked(),
            "skin": self.skin.get_untracked(),
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
    let settings = use_context::<Stored<Option<SettingsClient>>>()
        .expect("App provides settings")
        .with(|s| s.clone())
        .expect("Desktop settings initialized");
    let preferences =
        Preferences::mount(settings, format!("desktop_emoji_preferences_{account_id}"));
    let query = signal(String::new());
    let group = signal("smileysEmotion");
    let filtered = computed(move || {
        let query = query.get().to_lowercase();
        let group = group.get();
        if group == "recent" && query.is_empty() {
            return preferences.recent.with(|recent| {
                recent
                    .iter()
                    .filter_map(|character| {
                        dictionary()
                            .iter()
                            .position(|e| {
                                &e.character == character || e.variants.contains(character)
                            })
                            .map(|index| (index, character.clone()))
                    })
                    .collect::<Vec<_>>()
            });
        }
        dictionary()
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                if !query.is_empty() {
                    e.search.contains(&query)
                } else if group == "smileysEmotion" {
                    e.group == group || e.group == "peopleBody"
                } else {
                    e.group == group
                }
            })
            .map(|(index, emoji)| {
                (
                    index,
                    emoji
                        .variants
                        .get(preferences.skin.get())
                        .unwrap_or(&emoji.character)
                        .clone(),
                )
            })
            .collect::<Vec<_>>()
    });
    let insert = stored(insert);
    // A glyph of `icon_lg` with `space_sm` around it.
    let cell = ui::icon_lg() + ui::space_lg();
    view()
        .flex_1()
        .min_h_0()
        .gap_2()
        .child(
            ui::field()
                .label(move || tr(i18n, "Search emoji", "搜索表情"))
                .placeholder(move || tr(i18n, "Search emoji…", "搜索表情…").to_owned())
                .auto_focus(true)
                .value(move || query.get())
                .on_change(move |value: &str| query.set(value.to_owned())),
        )
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
                move || filtered.with(Vec::len),
                move |at| {
                    ui::command()
                        .size(cell)
                        .px_1()
                        .py_1()
                        .accessibility_label(move || {
                            Some(
                                filtered
                                    .with(|indices| {
                                        indices
                                            .get(at.index())
                                            .map(|(i, _)| dictionary()[*i].name.clone())
                                            .unwrap_or_default()
                                    })
                                    .into(),
                            )
                        })
                        .child(
                            text(move || {
                                filtered.with(|rows| {
                                    rows.get(at.index())
                                        .map(|(_, character)| character.clone())
                                        .unwrap_or_default()
                                })
                            })
                            .text_size(ui::icon_lg()),
                        )
                        .on_press(move || {
                            let selected = filtered.with(|rows| {
                                rows.get(at.index()).map(|(_, character)| character.clone())
                            });
                            if let Some(selected) = selected {
                                insert.with(|insert| insert(selected.clone()));
                                preferences.remember(selected);
                            }
                        })
                },
            )
            .keys(move || {
                filtered.with(|indices| {
                    indices
                        .iter()
                        .map(|(_, character)| character.clone())
                        .collect::<Vec<_>>()
                })
            })
            .columns(move || columns(cell.0))
            .estimated_item_size(cell)
            .flex_1()
            .min_h_0()
            .style_when(move || filtered.with(Vec::is_empty), |style| style.hidden()),
        )
        .child(when(
            move || preferences.loaded.get() && filtered.with(Vec::is_empty),
            move || {
                ui::placeholder(move || {
                    if query.with(String::is_empty) && group.get() == "recent" {
                        tr(i18n, "No recent emoji", "暂无最近使用的表情")
                    } else {
                        tr(i18n, "No emoji found", "未找到表情")
                    }
                })
            },
        ))
        .child(
            scroll().horizontal().child(
                view().flex_row().gap_1().children(
                    [
                        (
                            "recent",
                            TabLabel::Icon(ui::Icon::Clock),
                            "Recently used",
                            "最近使用",
                        ),
                        (
                            "smileysEmotion",
                            TabLabel::Text("😀"),
                            "Smileys and people",
                            "表情与人物",
                        ),
                        (
                            "animalsNature",
                            TabLabel::Text("🐻"),
                            "Animals and nature",
                            "动物与自然",
                        ),
                        (
                            "foodDrink",
                            TabLabel::Text("🍔"),
                            "Food and drink",
                            "食物与饮料",
                        ),
                        (
                            "travelPlaces",
                            TabLabel::Text("🚙"),
                            "Travel and places",
                            "旅行与地点",
                        ),
                        ("activities", TabLabel::Text("⚽"), "Activities", "活动"),
                        ("objects", TabLabel::Text("💡"), "Objects", "物品"),
                        ("symbols", TabLabel::Text("❤️"), "Symbols", "符号"),
                        ("flags", TabLabel::Text("🏳️"), "Flags", "旗帜"),
                    ]
                    .into_iter()
                    .map(move |(value, title, en, zh)| {
                        panel_tab()
                            .accessibility_label(move || Some(tr(i18n, en, zh).into()))
                            .selected(move || group.get() == value)
                            .on_press(move || group.set(value))
                            .child(title.element())
                    }),
                ),
            ),
        )
        .child(
            view().flex_row().gap_1().children(
                [
                    ("🖐️", "Default skin tone", "默认肤色"),
                    ("🖐🏻", "Light skin tone", "浅肤色"),
                    ("🖐🏼", "Medium-light skin tone", "中浅肤色"),
                    ("🖐🏽", "Medium skin tone", "中等肤色"),
                    ("🖐🏾", "Medium-dark skin tone", "中深肤色"),
                    ("🖐🏿", "Dark skin tone", "深肤色"),
                ]
                .into_iter()
                .enumerate()
                .map(move |(index, (title, en, zh))| {
                    panel_tab()
                        .disabled(move || !preferences.loaded.get())
                        .accessibility_label(move || Some(tr(i18n, en, zh).into()))
                        .selected(move || preferences.skin.get() == index)
                        .child(text(title))
                        .on_press(move || {
                            preferences.skin.set(index);
                            preferences.save();
                        })
                }),
            ),
        )
}
