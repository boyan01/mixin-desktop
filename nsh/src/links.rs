//! Session-owned routing for message, search, webview, and native URL entry points.
mod parse;
mod preview;
mod recipient;
mod send;
mod view;

use mixin_desktop_api::{AccountClient, CodeResult, WebShareContent};
use nsh_core::prelude::*;
use std::{rc::Rc, sync::Arc};
use url::Url;

use crate::{navigation::ChatNavigation, runtime};
use parse::Target;

#[derive(Clone)]
pub(crate) struct AppLink {
    pub app_id: Option<String>,
    /// Only an unclassified sender may resolve to a plain website.
    pub allow_generic: bool,
    pub url: String,
    pub source_conversation: Option<String>,
    pub app_card_data: Option<serde_json::Value>,
}

#[derive(Clone)]
enum Panel {
    Loading,
    Error,
    Profile(String),
    Code(String, Box<CodeResult>),
    Snapshot(crate::snapshot::Reference),
    Send(parse::Send),
    App(AppLink),
    Unsupported(String),
}

type AppOpener = Rc<dyn Fn(AppLink)>;

#[derive(Clone, Copy)]
pub(crate) struct LinkNavigation {
    account: Stored<Arc<AccountClient>>,
    owner: Owner,
    navigation: ChatNavigation,
    panel: Signal<Option<Panel>>,
    request: Signal<u64>,
    app_opener: Stored<Option<AppOpener>>,
    failed: Signal<bool>,
}

fn tr(en: &'static str, zh: &'static str) -> &'static str {
    crate::i18n::tr(en, zh)
}

impl LinkNavigation {
    pub(crate) fn new(account: Arc<AccountClient>) -> Self {
        let state = Self {
            account: stored(account),
            owner: Owner::current(),
            navigation: use_context::<ChatNavigation>()
                .expect("Home provides navigation before link routing"),
            panel: signal_with(None, SignalEquals::Never),
            request: signal(0),
            app_opener: stored(None),
            failed: signal(false),
        };
        effect(move || {
            if state.failed.get() {
                state.failed.set(false);
                state.panel.set(Some(Panel::Error));
            }
        });
        state
    }

    pub(crate) fn on_open_app(self, open: impl Fn(AppLink) + 'static) {
        self.app_opener.replace(Some(Rc::new(open)));
    }

    fn client(self) -> Arc<AccountClient> {
        self.account.with(|account| account.clone())
    }

    fn close(self) {
        self.request.update(|request| *request += 1);
        self.panel.set(None);
    }

    pub(crate) fn open(self, raw: String, source_conversation: Option<String>) {
        self.navigation.cancel_pending();
        self.failed.set(false);
        self.request.update(|request| *request += 1);
        let ticket = self.request.get_untracked();
        let target = match parse::parse(&raw, source_conversation.clone()) {
            Ok(target) => target,
            Err(()) => {
                self.panel.set(Some(Panel::Error));
                return;
            }
        };
        self.panel.set(Some(Panel::Loading));
        self.owner.run(|| {
            spawn(async move {
                match target {
                    Target::User(id) => self.show(ticket, Panel::Profile(id)),
                    Target::Snapshot(reference) => self.show(ticket, Panel::Snapshot(reference)),
                    Target::Send(send) => self.show(ticket, Panel::Send(send)),
                    Target::Unsupported(raw) => self.show(ticket, Panel::Unsupported(raw)),
                    Target::External(url) => {
                        let opened = open_url(url.to_string()).await;
                        if self.current(ticket) {
                            self.panel.set((!opened).then_some(Panel::Error));
                        }
                    }
                    Target::Code(code) => {
                        let client = self.client();
                        let value = code.clone();
                        let result =
                            runtime::run(
                                async move { client.conversation().resolve_code(value).await },
                            )
                            .await;
                        match result {
                            Ok(value) if value.kind == "user" => {
                                if let Some(id) = value.user_id {
                                    self.show(ticket, Panel::Profile(id));
                                } else {
                                    self.show(ticket, Panel::Error);
                                }
                            }
                            Ok(value) => self.show(ticket, Panel::Code(code, Box::new(value))),
                            Err(_) => self.show(ticket, Panel::Error),
                        }
                    }
                    Target::Conversation { id, user, message } => {
                        let client = self.client();
                        let result = runtime::run(async move {
                            client
                                .conversation()
                                .resolve_conversation_link(id, user)
                                .await
                        })
                        .await;
                        match result {
                            Ok(row) if self.current(ticket) => {
                                self.panel.set(None);
                                if let Some(message) = message {
                                    let failed = self.failure_signal();
                                    self.navigation.open_conversation(
                                        self.client(),
                                        row.conversation_id,
                                        Some(message),
                                        failed,
                                    );
                                } else {
                                    self.navigation.select(row);
                                }
                            }
                            Ok(_) => {}
                            Err(_) => self.show(ticket, Panel::Error),
                        }
                    }
                    Target::Message(id) => {
                        let client = self.client();
                        let result = runtime::run(async move {
                            client.message().message_items_by_ids(vec![id]).await
                        })
                        .await;
                        match result {
                            Ok(mut rows) if self.current(ticket) && !rows.is_empty() => {
                                let message = rows.remove(0);
                                self.panel.set(None);
                                self.navigation.open_conversation(
                                    self.client(),
                                    message.conversation_id,
                                    Some(message.message_id),
                                    self.failure_signal(),
                                );
                            }
                            Ok(_) if !self.current(ticket) => {}
                            _ => self.show(ticket, Panel::Error),
                        }
                    }
                    Target::App(id, url) => {
                        self.open_app(ticket, id, url, source_conversation).await
                    }
                }
            });
        });
    }

    /// Opens the existing recipient picker and send confirmation for a native page action.
    pub(crate) fn share_web_page(
        self,
        url: String,
        title: String,
        app_id: Option<String>,
        shareable: bool,
    ) {
        self.navigation.cancel_pending();
        self.request.update(|request| *request += 1);
        let ticket = self.request.get_untracked();
        self.panel.set(Some(Panel::Loading));
        let client = self.client();
        self.owner.run(|| {
            spawn(async move {
                let result = runtime::run(async move {
                    client
                        .bot()
                        .prepare_web_share(app_id, url, title, shareable)
                        .await
                })
                .await;
                let payload = match result {
                    Ok(WebShareContent::Text(url)) => Some(parse::Payload::Text(url)),
                    Ok(WebShareContent::AppCard(card)) => {
                        serde_json::from_str(&card).ok().map(parse::Payload::Card)
                    }
                    Err(_) => None,
                };
                match payload {
                    Some(payload) => self.show(
                        ticket,
                        Panel::Send(parse::Send {
                            recipient: parse::Recipient::Choose,
                            payload,
                        }),
                    ),
                    None => self.show(ticket, Panel::Error),
                }
            })
        });
    }

    fn current(self, ticket: u64) -> bool {
        self.request.get_untracked() == ticket
    }
    fn show(self, ticket: u64, panel: Panel) {
        if self.current(ticket) {
            self.panel.set(Some(panel));
        }
    }

    fn failure_signal(self) -> Signal<bool> {
        self.failed
    }

    async fn open_app(
        self,
        ticket: u64,
        id: String,
        url: Url,
        source_conversation: Option<String>,
    ) {
        let client = self.client();
        let app_id = id.clone();
        let result = runtime::run(async move {
            let profile = client
                .user()
                .refresh_user_profile(app_id.clone())
                .await?
                .ok_or(mixin_desktop_api::ClientError::NotFound)?;
            if !profile.is_bot {
                return Err(mixin_desktop_api::ClientError::InvalidArgument(
                    "The link does not identify a bot".into(),
                ));
            }
            client.user().bot_home_uri(app_id).await
        })
        .await;
        let Some(raw_home) = result.ok().flatten() else {
            self.show(ticket, Panel::Error);
            return;
        };
        let Ok(mut home) = Url::parse(&raw_home) else {
            self.show(ticket, Panel::Error);
            return;
        };
        if !matches!(home.scheme(), "http" | "https") {
            self.show(ticket, Panel::Error);
            return;
        }
        let mut parameters = home
            .query_pairs()
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect::<std::collections::BTreeMap<_, _>>();
        parameters.extend(
            url.query_pairs()
                .filter(|(key, _)| key != "action")
                .map(|(key, value)| (key.into_owned(), value.into_owned())),
        );
        home.query_pairs_mut().clear().extend_pairs(parameters);
        let link = AppLink {
            app_id: Some(id),
            allow_generic: false,
            url: home.into(),
            source_conversation,
            app_card_data: None,
        };
        if self.current(ticket) {
            if let Some(open) = self.app_opener.with(|open| open.clone()) {
                self.panel.set(None);
                open(link);
            } else {
                self.panel.set(Some(Panel::App(link)));
            }
        }
    }

    pub(crate) fn surface(self) -> impl IntoElement {
        view::surface(self)
    }
}
