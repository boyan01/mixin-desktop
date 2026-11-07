use futures::StreamExt as _;
use mixin_desktop_api::{AccountClient, ConversationUnseenCount};
use nsh_base::theme;
use nsh_core::{animation::Easing, element::AnimatedValue, prelude::*};
use std::sync::Arc;

use crate::{
    i18n::{self, I18n, Key},
    runtime, ui,
};

mod circle_editor;
mod circles;
mod preference;

pub(crate) use circles::{Circles, membership_item as circle_membership_item};

#[derive(Clone, Copy)]
pub(crate) struct Navigation {
    pub category: Signal<&'static str>,
    pub circle: Signal<Option<String>>,
    pub collapsed: Signal<bool>,
    pub settings: Signal<bool>,
    pub list_reset: Signal<u64>,
    drawer_open: Signal<bool>,
}

impl Navigation {
    pub fn new() -> Self {
        Self {
            category: signal("chats"),
            circle: signal(None),
            collapsed: signal(false),
            settings: signal(false),
            list_reset: signal(0),
            drawer_open: signal(false),
        }
    }
    pub fn drawer(self) -> bool {
        window().width.0 <= 384.0
    }
    pub fn compact(self) -> bool {
        !self.drawer() && (self.collapsed.get() || window().width.0 < 496.0)
    }
    pub fn width(self) -> Points {
        if self.drawer() {
            px(0.0)
        } else if self.compact() {
            px(64.0)
        } else {
            px(176.0)
        }
    }
    pub fn open_drawer(self) {
        if self.drawer() {
            self.drawer_open.set(true);
        }
    }
    pub fn close_drawer(self) {
        self.drawer_open.set(false);
    }
    pub fn select(self, category: &'static str, circle: Option<String>) {
        batch(|| {
            self.category.set(category);
            self.circle.set(circle);
            self.settings.set(false);
            self.list_reset
                .update(|revision| *revision = revision.wrapping_add(1));
            self.close_drawer();
        });
    }
}

pub(crate) fn screen(
    account: Arc<AccountClient>,
    nav: Navigation,
    circles: Circles,
) -> impl IntoElement {
    let profile = signal_with(account.profile(), SignalEquals::Never);
    let preference = preference::Preference::new(nav);
    let counts = signal(Vec::<ConversationUnseenCount>::new());
    let client = account.clone();
    spawn(async move {
        let mut stream = Box::pin(runtime::observe(client.profile_changes()));
        while let Some(value) = stream.next().await {
            profile.set(value);
        }
    });
    spawn(async move {
        let mut stream = Box::pin(runtime::observe(account.unseen_count_changes()));
        while let Some(value) = stream.next().await {
            counts.set(value);
        }
    });
    let labels = Labels::new(nav);
    effect(move || {
        if !nav.drawer() {
            nav.close_drawer();
        }
    });
    view()
        .w(move || nav.width())
        .flex_shrink_0()
        .child(dynamic(move || {
            let drawer = nav.drawer();
            // Resizing out of drawer mode disposes the native modal immediately.
            // The account subscriptions above remain owned by this session.
            untracked(|| {
                if drawer {
                    drawer_surface(nav, profile, counts, circles, preference, labels).into_element()
                } else {
                    content(nav, profile, counts, circles, preference, labels).into_element()
                }
            })
        }))
        .child(circles.surface())
}

fn drawer_surface(
    nav: Navigation,
    profile: Signal<mixin_desktop_api::AccountProfile>,
    counts: Signal<Vec<ConversationUnseenCount>>,
    circles: circles::Circles,
    preference: preference::Preference,
    labels: Labels,
) -> impl IntoElement {
    let presence = crate::motion::Presence::panel(move || nav.drawer_open.get());
    when(
        move || presence.mounted(),
        move || {
            nsh_base::dialog()
                .animation(nsh_core::ModalAnimation::None)
                .close_on_escape(false)
                .close_on_backdrop_press(false)
                .on_cancel(move || nav.close_drawer())
                .flex_1()
                .child(
                    view()
                        .flex_1()
                        .flex_row()
                        .child(
                            presence.backdrop(
                                view()
                                    .absolute()
                                    .inset_0()
                                    .bg(move || theme().foreground.opacity(0.2))
                                    .on_press(move || nav.close_drawer()),
                            ),
                        )
                        .child(presence.surface(
                            content(nav, profile, counts, circles, preference, labels).w(px(176.0)),
                        )),
                )
        },
    )
}

pub(crate) fn trigger(nav: Navigation) -> impl IntoElement {
    let i18n = i18n::current();
    when(
        move || nav.drawer(),
        move || {
            ui::icon_button()
                .on_press(move || nav.open_drawer())
                .accessibility_label(move || Some(i18n.tr("Open navigation", "打开导航").into()))
                .child(ui::icon(ui::Icon::Menu))
        },
    )
}

// Labels share one native animation; changing the rail width never animates layout.
#[derive(Clone, Copy)]
pub(super) struct Labels {
    visible: Signal<bool>,
    opacity: AnimatedValue,
}

impl Labels {
    fn new(nav: Navigation) -> Self {
        let labels = Self {
            visible: signal(!nav.compact()),
            opacity: animated_value(if nav.compact() { 0.0 } else { 1.0 }),
        };
        let generation = stored(0_u64);
        let compact = computed(move || nav.compact());
        effect(move || {
            let compact = compact.get();
            let reduced = reduce_motion();
            untracked(|| {
                let change = generation.get().wrapping_add(1);
                generation.replace(change);
                if !compact {
                    labels.visible.set(true);
                }
                let end = labels.opacity.timing(
                    if compact { 0.0 } else { 1.0 },
                    if reduced {
                        80.0
                    } else if compact {
                        90.0
                    } else {
                        130.0
                    },
                    Easing::EaseOut,
                );
                if compact {
                    spawn(async move {
                        if end.await && generation.get() == change {
                            labels.visible.set(false);
                        }
                    });
                }
            });
        });
        labels
    }
}

fn content(
    nav: Navigation,
    profile: Signal<mixin_desktop_api::AccountProfile>,
    counts: Signal<Vec<ConversationUnseenCount>>,
    circles: circles::Circles,
    preference: preference::Preference,
    labels: Labels,
) -> View {
    let i18n = i18n::current();
    view()
        .w(move || if nav.drawer() { px(176.0) } else { nav.width() })
        .h_full()
        .flex_shrink_0()
        .overflow_hidden()
        .bg(move || theme().sidebar)
        .border_r_1()
        .border_color(move || theme().border)
        .px_3()
        .gap_1()
        .child(view().h_16().window_drag(true))
        .child(
            ui::command()
                .justify_start()
                .gap_3()
                .px_2()
                .py_3()
                .min_h_16()
                .selected(nav.settings)
                .selected_style(|s| s.bg(move || theme().accent))
                .transition_colors()
                .duration(|| crate::motion::duration(160.0))
                .ease_out()
                .on_press(move || {
                    batch(|| {
                        nav.settings.set(true);
                        nav.close_drawer();
                    })
                })
                .accessibility_label(move || Some(i18n.text(Key::Settings).into()))
                .child(
                    ui::avatar_view(
                        move || profile.with(|p| p.full_name.clone().into()),
                        move || profile.with(|p| p.avatar_url.clone()),
                    )
                    .size_6(),
                )
                .child(
                    view()
                        .flex_1()
                        .min_w_0()
                        .gap_1()
                        .bind_opacity(&labels.opacity)
                        .style_when(move || !labels.visible.get(), |s| s.hidden())
                        .child(
                            text(move || profile.with(|p| p.full_name.clone()))
                                .text_sm()
                                .font_weight(FontWeight::MEDIUM)
                                .number_of_lines(1),
                        )
                        .child(
                            text(move || profile.with(|p| p.identity_number.clone()))
                                .text_xs()
                                .text_color(move || theme().muted_foreground),
                        ),
                ),
        )
        .child(view().h_4())
        .child(category(
            i18n,
            nav,
            ui::Icon::Chat,
            Key::AllChats,
            "chats",
            counts,
            labels,
        ))
        .child(divider())
        .child(category(
            i18n,
            nav,
            ui::Icon::Contacts,
            Key::Contacts,
            "contacts",
            counts,
            labels,
        ))
        .child(category(
            i18n,
            nav,
            ui::Icon::Groups,
            Key::Groups,
            "groups",
            counts,
            labels,
        ))
        .child(category(
            i18n,
            nav,
            ui::Icon::Bot,
            Key::Bots,
            "bots",
            counts,
            labels,
        ))
        .child(category(
            i18n,
            nav,
            ui::Icon::Strangers,
            Key::Strangers,
            "strangers",
            counts,
            labels,
        ))
        .child(circles.list(counts, labels))
        .child(
            ui::command()
                .justify_start()
                .px_2()
                .gap_3()
                .min_h_12()
                .accessibility_label(move || {
                    Some(
                        i18n.text(if nav.compact() {
                            Key::Expand
                        } else {
                            Key::Collapse
                        })
                        .into(),
                    )
                })
                .disabled(move || preference.disabled())
                .style_when(move || window().width.0 < 496.0, |style| style.hidden())
                .on_press(move || preference.toggle())
                .child(dynamic(move || {
                    ui::icon(if nav.compact() {
                        ui::Icon::Expand
                    } else {
                        ui::Icon::Collapse
                    })
                }))
                .child(
                    text(move || i18n.text(Key::Collapse))
                        .text_sm()
                        .bind_opacity(&labels.opacity)
                        .style_when(move || !labels.visible.get(), |s| s.hidden()),
                ),
        )
        .child(preference.feedback())
        .child(circles.feedback())
}

fn category(
    i18n: I18n,
    nav: Navigation,
    icon: ui::Icon,
    label: Key,
    value: &'static str,
    counts: Signal<Vec<ConversationUnseenCount>>,
    labels: Labels,
) -> impl IntoElement {
    let count = move || {
        counts.with(|counts| {
            counts
                .iter()
                .find(|c| c.category == value && c.circle_id.is_none())
                .map_or(0, |c| c.count)
        })
    };
    let muted = move || {
        counts.with(|counts| {
            counts
                .iter()
                .find(|count| count.category == value && count.circle_id.is_none())
                .map_or(0, |count| count.muted_count)
        })
    };
    let item = ui::command()
        .min_h_11()
        .justify_start()
        .px_2()
        .gap_3()
        .accessibility_label(move || Some(i18n.text(label).into()))
        .selected(move || !nav.settings.get() && nav.category.get() == value)
        .selected_style(|s| s.bg(move || theme().accent))
        .transition_colors()
        .duration(|| crate::motion::duration(160.0))
        .ease_out()
        .on_press(move || nav.select(value, None))
        .child(navigation_icon(icon, nav, count, muted))
        .child(
            text(move || i18n.text(label))
                .text_sm()
                .flex_1()
                .number_of_lines(1)
                .bind_opacity(&labels.opacity)
                .style_when(move || !labels.visible.get(), |s| s.hidden()),
        )
        .child(when(
            move || count() > 0 && !nav.compact(),
            move || {
                view()
                    .min_w_6()
                    .h_5()
                    .px_1()
                    .rounded_full()
                    .items_center()
                    .justify_center()
                    .bg(move || theme().muted_foreground.opacity(0.7))
                    .child(
                        text(move || count().to_string())
                            .text_xs()
                            .text_color(move || theme().background),
                    )
            },
        ));
    view()
        .w_full()
        .tooltip(
            nsh_base::tooltip()
                .disabled(move || !nav.compact())
                .px_3()
                .py_2()
                .rounded_md()
                .bg(move || theme().popover)
                .text_color(move || theme().popover_foreground)
                .border_1()
                .border_color(move || theme().border)
                .child(
                    text(move || {
                        if count() > 0 {
                            format!("{} ({})", i18n.text(label), count())
                        } else {
                            i18n.text(label).into()
                        }
                    })
                    .text_sm(),
                ),
        )
        .child(item.w_full())
}

fn divider() -> View {
    view().h(px(1.0)).mx_2().my_3().bg(move || theme().border)
}

fn badge(count: impl Fn() -> i64 + Copy + 'static) -> View {
    view()
        .min_w_6()
        .h_5()
        .px_1()
        .rounded_full()
        .items_center()
        .justify_center()
        .bg(move || theme().muted_foreground)
        .child(
            text(move || count().to_string())
                .text_xs()
                .text_color(move || theme().background),
        )
}

fn navigation_icon(
    icon: ui::Icon,
    nav: Navigation,
    count: impl Fn() -> i64 + Copy + 'static,
    muted: impl Fn() -> i64 + Copy + 'static,
) -> View {
    view()
        .relative()
        .size_6()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .child(ui::icon(icon))
        .child(when(
            move || nav.compact() && count() > 0,
            move || {
                view()
                    .absolute()
                    .top_0()
                    .right_0()
                    .size(px(6.0))
                    .rounded_full()
                    .bg(move || {
                        if count() == muted() {
                            theme().muted_foreground
                        } else {
                            theme().color("unread").unwrap_or(theme().primary)
                        }
                    })
            },
        ))
}
