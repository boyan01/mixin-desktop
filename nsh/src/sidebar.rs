use futures::StreamExt as _;
use mixin_desktop_api::{AccountClient, AccountProfile, ConversationUnseenCount};
use nsh_core::{animation::Easing, element::AnimatedValue, prelude::*};
use std::sync::Arc;

use crate::{
    i18n::{self, Key},
    runtime,
    ui::{self, Typography as _},
};

mod circle_editor;
mod circles;
mod preference;

pub(crate) use circles::{Circles, add_to_circle_item, remove_from_circle_item};

use crate::navigation::Navigation;

pub(crate) fn screen(
    account: Arc<AccountClient>,
    nav: Navigation,
    circles: Circles,
) -> impl IntoElement {
    let profile = signal_with(account.profile(), SignalEquals::Never);
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
    let rail = Rail {
        nav,
        profile,
        counts,
        labels: Labels::new(nav),
        services: Some(Services {
            circles,
            preference: preference::Preference::new(nav),
        }),
    };
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
                    rail.drawer().into_element()
                } else {
                    rail.content().into_element()
                }
            })
        }))
        .child(circles.surface())
}

pub(crate) fn trigger(nav: Navigation) -> impl IntoElement {
    let i18n = i18n::current();
    when(
        move || nav.drawer(),
        move || {
            ui::round_button()
                .size_8()
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
            // Runs again when Reduce Motion changes.
            let _ = reduce_motion();
            untracked(|| {
                let change = generation.get().wrapping_add(1);
                generation.replace(change);
                if !compact {
                    labels.visible.set(true);
                }
                let end = labels.opacity.timing(
                    if compact { 0.0 } else { 1.0 },
                    crate::motion::duration(if compact {
                        crate::motion::RAIL_LABELS_EXIT_MS
                    } else {
                        crate::motion::RAIL_LABELS_ENTER_MS
                    }),
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

    fn bind(self, content: View) -> View {
        content
            .bind_opacity(&self.opacity)
            .style_when(move || !self.visible.get(), |style| style.hidden())
    }

    fn bind_text(self, label: Text) -> Text {
        label
            .text_role(ui::TextRole::Body)
            .flex_1()
            .number_of_lines(1)
            .bind_opacity(&self.opacity)
            .style_when(move || !self.visible.get(), |style| style.hidden())
    }
}

/// The account's circles and persisted Collapse state.
#[derive(Clone, Copy)]
struct Services {
    circles: Circles,
    preference: preference::Preference,
}

/// Without services the rail lists no circles and Collapse is not persisted.
#[derive(Clone, Copy)]
pub(crate) struct Rail {
    nav: Navigation,
    profile: Signal<AccountProfile>,
    counts: Signal<Vec<ConversationUnseenCount>>,
    labels: Labels,
    services: Option<Services>,
}

impl Rail {
    /// A rail over the given sources, without account services, for the preview.
    #[cfg(debug_assertions)]
    pub(crate) fn detached(
        nav: Navigation,
        profile: Signal<AccountProfile>,
        counts: Signal<Vec<ConversationUnseenCount>>,
    ) -> Self {
        Self {
            nav,
            profile,
            counts,
            labels: Labels::new(nav),
            services: None,
        }
    }

    fn drawer(self) -> impl IntoElement {
        let nav = self.nav;
        let presence = crate::motion::Presence::panel(move || nav.drawer_open());
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
                                presence.backdrop(ui::scrim().on_press(move || nav.close_drawer())),
                            )
                            .child(presence.surface(self.content())),
                    )
            },
        )
    }

    pub(crate) fn content(self) -> View {
        self.content_with_feedback(view().hidden())
    }

    pub(crate) fn content_with_feedback(self, feedback: impl IntoElement) -> View {
        let nav = self.nav;
        view()
            .w(move || {
                if nav.drawer() {
                    ui::navigation_width()
                } else {
                    nav.width()
                }
            })
            .h_full()
            .flex_shrink_0()
            .overflow_hidden()
            .bg(ui::surface_rail)
            .px_3()
            .pb_2()
            .gap_1()
            .child(ui::title_bar())
            .child(self.account_item())
            .child(view().h_2())
            .child(self.category(ui::Icon::Chat, Key::AllChats, "chats"))
            .child(ui::divider())
            .child(self.category(ui::Icon::Contacts, Key::Contacts, "contacts"))
            .child(self.category(ui::Icon::Groups, Key::Groups, "groups"))
            .child(self.category(ui::Icon::Bot, Key::Bots, "bots"))
            .child(self.category(ui::Icon::Strangers, Key::Strangers, "strangers"))
            .child(match self.services {
                Some(services) => services
                    .circles
                    .list(self.counts, self.labels)
                    .into_element(),
                None => view().flex_1().min_h_0().into_element(),
            })
            .child(self.collapse_item())
            .child(self.services.map(|services| {
                view()
                    .child(services.preference.feedback())
                    .child(services.circles.feedback())
            }))
            .child(feedback)
    }

    fn account_item(self) -> impl IntoElement {
        let i18n = i18n::current();
        let Self {
            nav,
            profile,
            labels,
            ..
        } = self;
        // Inset so its avatar centers on the rows' icon column, also when collapsed.
        ui::card().child(
            ui::card_row()
                .min_h_11()
                .px(ui::space_xs())
                .py(ui::space_xs())
                .selected(nav.settings)
                .on_press(move || {
                    batch(|| {
                        nav.settings.set(true);
                        nav.close_drawer();
                    })
                })
                .accessibility_label(move || Some(i18n.text(Key::Settings).into()))
                .child(ui::avatar_view(
                    move || profile.with(|p| p.user_id.clone()),
                    move || profile.with(|p| p.full_name.clone().into()),
                    move || profile.with(|p| p.avatar_url.clone()),
                    ui::AvatarSize::Compact,
                ))
                .child(
                    labels
                        .bind(view().flex_1().min_w_0())
                        .child(
                            text(move || profile.with(|p| p.full_name.clone()))
                                .text_role(ui::TextRole::Body)
                                .text_emphasis()
                                .number_of_lines(1),
                        )
                        .child(
                            text(move || profile.with(|p| p.identity_number.clone()))
                                .text_role(ui::TextRole::Caption)
                                .number_of_lines(1)
                                .text_color(ui::fg_secondary),
                        ),
                ),
        )
    }

    fn category(self, icon: ui::Icon, label: Key, value: &'static str) -> impl IntoElement {
        let i18n = i18n::current();
        let Self {
            nav,
            counts,
            labels,
            ..
        } = self;
        let unseen = computed(move || {
            counts.with(|counts| {
                counts
                    .iter()
                    .find(|count| count.category == value && count.circle_id.is_none())
                    .map_or((0, 0), |count| (count.count, count.muted_count))
            })
        });
        let count = move || unseen.get().0;
        let muted = move || unseen.get().1;
        let item = ui::nav_row()
            .accessibility_label(move || Some(i18n.text(label).into()))
            .selected(move || !nav.settings.get() && nav.category.get() == value)
            .on_press(move || nav.select(value, None))
            .child(navigation_icon(ui::icon(icon), nav, count, muted))
            .child(labels.bind_text(text(move || i18n.text(label))))
            .child(when(
                move || count() > 0 && !nav.compact(),
                move || ui::count_pill(move || count().to_string(), || false),
            ));
        view()
            .w_full()
            .tooltip(
                ui::hint(move || {
                    if count() > 0 {
                        format!("{} ({})", i18n.text(label), count())
                    } else {
                        i18n.text(label).into()
                    }
                })
                .disabled(move || !nav.compact()),
            )
            .child(item.w_full())
    }

    fn collapse_item(self) -> impl IntoElement {
        let i18n = i18n::current();
        let Self {
            nav,
            services,
            labels,
            ..
        } = self;
        let preference = services.map(|services| services.preference);
        ui::nav_row()
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
            .disabled(move || preference.is_some_and(|preference| preference.disabled()))
            .style_when(move || nav.forced_compact(), |style| style.hidden())
            .on_press(move || {
                if let Some(preference) = preference {
                    preference.toggle();
                } else {
                    nav.animate_width();
                    nav.collapsed.update(|collapsed| *collapsed = !*collapsed);
                }
            })
            .child(dynamic(move || {
                ui::icon(if nav.compact() {
                    ui::Icon::Expand
                } else {
                    ui::Icon::Collapse
                })
            }))
            .child(labels.bind_text(text(move || i18n.text(Key::Collapse))))
    }
}

fn navigation_icon(
    icon: Svg,
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
        .child(icon)
        .child(when(
            move || nav.compact() && count() > 0,
            move || {
                view()
                    .absolute()
                    .top_0()
                    .right_0()
                    .size_1p5()
                    .rounded(ui::radius_full())
                    .bg(move || {
                        if count() == muted() {
                            ui::fg_secondary()
                        } else {
                            ui::danger()
                        }
                    })
            },
        ))
}

#[cfg(debug_assertions)]
pub(crate) use {circle_editor::preview as preview_circle_editor, circles::preview_rename};

#[cfg(debug_assertions)]
pub(crate) use preference::preference_error;
