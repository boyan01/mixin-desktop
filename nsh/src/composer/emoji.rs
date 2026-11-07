use mixin_desktop_api::SettingsClient;
use nsh_base::theme;
use nsh_core::prelude::*;
use serde_json::Value;
use std::{rc::Rc, sync::OnceLock};

use super::tr;
use crate::{
    i18n::{self, Key},
    runtime, ui,
};

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
    busy: Signal<bool>,
    failed: Signal<bool>,
    settings: Stored<SettingsClient>,
    loaded: Signal<bool>,
    key: Stored<String>,
}
impl Preferences {
    fn load(self) {
        let settings = self.settings.with(|settings| settings.clone());
        let key = self.key.with(|key| key.clone());
        self.busy.set(true);
        self.failed.set(false);
        spawn(async move {
            match runtime::run(async move { settings.setting(key).await }).await {
                Ok(value) => {
                    if let Some(value) =
                        value.and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
                    {
                        self.recent.set(
                            value["recent"]
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter_map(Value::as_str)
                                .take(64)
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
                    self.loaded.set(true);
                }
                Err(_) => self.failed.set(true),
            }
            self.busy.set(false);
        });
    }
    fn save(self) {
        let client = self.settings.with(|s| s.clone());
        let key = self.key.with(|key| key.clone());
        let value = serde_json::json!({"recent": self.recent.get_untracked(), "skin": self.skin.get_untracked()}).to_string();
        self.busy.set(true);
        self.failed.set(false);
        spawn(async move {
            self.failed.set(
                runtime::commit(async move { client.set_setting(key, Some(value)).await })
                    .await
                    .is_err(),
            );
            self.busy.set(false);
        });
    }
}

pub(super) fn panel(insert: Rc<dyn Fn(String)>, account_id: String) -> impl IntoElement {
    let i18n = i18n::current();
    let settings = use_context::<Stored<Option<SettingsClient>>>()
        .expect("App provides settings")
        .with(|s| s.clone())
        .expect("Desktop settings initialized");
    let preferences = Preferences {
        recent: signal(Vec::new()),
        skin: signal(0),
        busy: signal(true),
        failed: signal(false),
        settings: stored(settings),
        loaded: signal(false),
        key: stored(format!("desktop_emoji_preferences_{account_id}")),
    };
    preferences.load();
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
                .gap_2()
                .child(
                    text(move || {
                        tr(
                            i18n,
                            "Couldn’t save emoji preferences.",
                            "无法保存表情偏好。",
                        )
                    })
                    .text_xs()
                    .text_color(move || theme().destructive),
                )
                .child(
                    ui::action(
                        move || i18n.text(Key::Retry),
                        move || {
                            if preferences.loaded.get_untracked() {
                                preferences.save();
                            } else {
                                preferences.load();
                            }
                        },
                    )
                    .disabled(preferences.busy),
                )
        }))
        .child(
            virtual_list(
                move || filtered.with(Vec::len),
                move |at| {
                    ui::command()
                        .size_10()
                        .px_1()
                        .py_1()
                        .disabled(move || preferences.busy.get() || !preferences.loaded.get())
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
                            .text_2xl(),
                        )
                        .on_press(move || {
                            let selected = filtered.with(|rows| {
                                rows.get(at.index()).map(|(_, character)| character.clone())
                            });
                            if let Some(selected) = selected {
                                insert.with(|insert| insert(selected.clone()));
                                preferences.recent.update(|recent| {
                                    recent.retain(|s| s != &selected);
                                    recent.insert(0, selected);
                                    recent.truncate(64);
                                });
                                preferences.save();
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
            .columns(move || {
                (((window().width.0 * 0.95).min(464.0) - 24.0) / 40.0)
                    .floor()
                    .max(1.0) as u32
            })
            .estimated_item_size(px(40.0))
            .flex_1()
            .min_h_0()
            .style_when(move || filtered.with(Vec::is_empty), |style| style.hidden()),
        )
        .child(when(
            move || preferences.loaded.get() && filtered.with(Vec::is_empty),
            move || {
                view()
                    .flex_1()
                    .min_h_0()
                    .items_center()
                    .justify_center()
                    .child(
                        text(move || {
                            if query.with(String::is_empty) && group.get() == "recent" {
                                tr(i18n, "No recent emoji", "暂无最近使用的表情")
                            } else {
                                tr(i18n, "No emoji found", "未找到表情")
                            }
                        })
                        .text_sm()
                        .text_color(move || theme().muted_foreground),
                    )
            },
        ))
        .child(
            scroll().horizontal().child(
                view().flex_row().gap_1().children(
                    [
                        ("recent", "◷", "Recently used", "最近使用"),
                        ("smileysEmotion", "😀", "Smileys and people", "表情与人物"),
                        ("animalsNature", "🐻", "Animals and nature", "动物与自然"),
                        ("foodDrink", "🍔", "Food and drink", "食物与饮料"),
                        ("travelPlaces", "🚙", "Travel and places", "旅行与地点"),
                        ("activities", "⚽", "Activities", "活动"),
                        ("objects", "💡", "Objects", "物品"),
                        ("symbols", "❤️", "Symbols", "符号"),
                        ("flags", "🏳️", "Flags", "旗帜"),
                    ]
                    .into_iter()
                    .map(move |(value, title, en, zh)| {
                        ui::command()
                            .size_8()
                            .px_1()
                            .py_1()
                            .accessibility_label(move || Some(tr(i18n, en, zh).into()))
                            .selected(move || group.get() == value)
                            .on_press(move || group.set(value))
                            .selected_style(|style| {
                                style
                                    .bg(move || theme().accent)
                                    .outline(px(1.0))
                                    .outline_color(move || theme().primary)
                            })
                            .child(text(title))
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
                    ui::command()
                        .size_8()
                        .px_1()
                        .py_1()
                        .disabled(move || preferences.busy.get() || !preferences.loaded.get())
                        .accessibility_label(move || Some(tr(i18n, en, zh).into()))
                        .selected(move || preferences.skin.get() == index)
                        .selected_style(|style| {
                            style
                                .bg(move || theme().accent)
                                .outline(px(1.0))
                                .outline_color(move || theme().primary)
                        })
                        .child(text(title))
                        .on_press(move || {
                            preferences.skin.set(index);
                            preferences.save();
                        })
                }),
            ),
        )
}
