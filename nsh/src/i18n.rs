use mixin_desktop_api::SettingsClient;
use nsh_base::{SelectItem, theme};
use nsh_core::prelude::*;

use crate::{conversation::parts, runtime, ui};

const LANGUAGE_SETTING: &str = "desktop_ui_language";

mod aliases;
mod catalog;
mod language;

use language::Language;

#[derive(Clone, Copy)]
pub(crate) struct I18n {
    owner: Owner,
    language: Signal<Language>,
    settings: Stored<Option<SettingsClient>>,
    saving: Signal<bool>,
    failed: Signal<bool>,
}

impl I18n {
    pub(crate) fn new() -> Self {
        Self {
            owner: Owner::current(),
            language: signal(Language::System),
            settings: stored(None),
            saving: signal(false),
            failed: signal(false),
        }
    }
    pub(crate) async fn load(&self, settings: SettingsClient) {
        let read = settings.clone();
        match runtime::run(async move { read.setting(LANGUAGE_SETTING.into()).await }).await {
            Ok(value) => self
                .language
                .set(Language::parse(value.as_deref().unwrap_or("system"))),
            Err(_) => self.failed.set(true),
        }
        self.settings.replace(Some(settings));
    }
    fn resolved(self) -> Language {
        match self.language.get() {
            Language::System => Language::system(locale().as_str()),
            language => language,
        }
    }
    /// Reactive BCP 47 locale for presentation and embedded bot contexts.
    pub(crate) fn locale_tag(self) -> &'static str {
        self.resolved().value()
    }
    pub(crate) fn chinese(self) -> bool {
        self.locale_tag().starts_with("zh")
    }
    pub(crate) fn members(self, count: i64) -> String {
        catalog::members(self.resolved(), count)
    }
    pub(crate) fn format(
        self,
        en: &'static str,
        zh: &'static str,
        arguments: &[(&str, &str)],
    ) -> String {
        catalog::format(self.resolved(), en, zh, arguments)
    }
    pub(crate) fn tr(self, en: &'static str, zh: &'static str) -> &'static str {
        let language = self.resolved();
        catalog::translation(language, en).unwrap_or(if language == Language::Chinese {
            zh
        } else {
            en
        })
    }
    pub(crate) fn text(self, key: Key) -> &'static str {
        let (en, zh) = key.strings();
        self.tr(en, zh)
    }
    fn choose(self, language: Language) {
        if self.saving.get_untracked() || language == self.language.get_untracked() {
            return;
        }
        let Some(settings) = self.settings.with(|value| value.clone()) else {
            return;
        };
        let previous = self.language.get_untracked();
        self.language.set(language);
        self.saving.set(true);
        self.failed.set(false);
        self.owner.run(|| {
            spawn(async move {
                let value = (language != Language::System).then(|| language.value().to_owned());
                if runtime::commit(async move {
                    settings.set_setting(LANGUAGE_SETTING.into(), value).await
                })
                .await
                .is_err()
                {
                    self.language.set(previous);
                    self.failed.set(true);
                }
                self.saving.set(false);
            })
        });
    }
    pub(crate) fn picker(self) -> impl IntoElement {
        view()
            .gap_1()
            .child(
                parts::option_select()
                    .items(move || {
                        let mut items = vec![SelectItem::new("system", self.text(Key::System))];
                        items.extend(
                            Language::SUPPORTED.into_iter().map(|language| {
                                SelectItem::new(language.value(), language.title())
                            }),
                        );
                        items
                    })
                    .value(move || Some(SharedString::from(self.language.get().value())))
                    .disabled(move || self.saving.get())
                    .accessibility_label(move || Some(self.text(Key::Language).into()))
                    .on_change(move |next: &Option<SharedString>| {
                        if let Some(value) = next {
                            self.choose(Language::parse(value.as_str()));
                        }
                    })
                    .offset(px(4.0))
                    .trigger(
                        view()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .px_3()
                            .py_2()
                            .rounded(move || theme().radius)
                            .hover(|s| s.bg(move || theme().muted))
                            .focus(|s| s.outline(px(2.0)).outline_color(move || theme().ring))
                            .child(ui::icon(ui::Icon::Globe))
                            .child(
                                text(move || match self.language.get() {
                                    Language::System => self.text(Key::System),
                                    language => language.title(),
                                })
                                .text_sm(),
                            )
                            .child(ui::icon(ui::Icon::ChevronDown)),
                    ),
            )
            .child(when(
                move || self.failed.get(),
                move || {
                    text(move || self.text(Key::LanguageFailed))
                        .text_xs()
                        .text_color(move || theme().destructive)
                },
            ))
    }
}

/// Use inside reactive text bindings, or a locale-keyed presentation subtree.
pub(crate) fn tr(en: &'static str, zh: &'static str) -> &'static str {
    current().tr(en, zh)
}

pub(crate) fn current() -> I18n {
    use_context::<I18n>().expect("App provides translations")
}

// Keeping each pair together makes a missing translation a compile error.
macro_rules! messages {
    ($($key:ident => ($en:literal, $zh:literal)),+ $(,)?) => {
        #[derive(Clone, Copy)]
        pub(crate) enum Key { $($key),+ }
        impl Key {
            fn strings(self) -> (&'static str, &'static str) {
                match self { $(Self::$key => ($en, $zh)),+ }
            }
        }
    };
}

messages! {
    Today => ("Today", "今天"),
    Yesterday => ("Yesterday", "昨天"),
    Creating => ("Creating…", "正在创建…"),
    PickConversation => ("Pick a conversation", "选择一个会话"),
    Language => ("Language", "语言"),
    System => ("System language", "跟随系统"),
    LanguageFailed => ("Couldn’t save the language. Try again.", "无法保存语言，请重试。"),
    Opening => ("Opening Mixin…", "正在打开 Mixin…"),
    OpenFailed => ("Couldn’t open Mixin", "无法打开 Mixin"),
    OpenFailedBody => ("Check your connection and try again.", "请检查网络连接后重试。"),
    ScanTitle => ("Scan to sign in", "扫码登录"),
    Preparing => ("Preparing your code…", "正在生成二维码…"),
    LoginFailed => ("Couldn’t create a sign-in code", "无法获取登录二维码"),
    Retry => ("Try again", "重试"),
    AllChats => ("All chats", "全部聊天"),
    Bots => ("Bots", "机器人"),
    Strangers => ("Strangers", "陌生人"),
    Collapse => ("Collapse", "折叠"),
    Expand => ("Expand", "展开"),
    Settings => ("Settings", "设置"),
    SearchContact => ("Search contact…", "搜索联系人…"),
    NewConversation => ("New conversation…", "新建会话…"),
    NewGroup => ("New group…", "新建群组…"),
    NewCircle => ("New circle…", "新建圈子…"),
    Name => ("Name", "名称"),
    Cancel => ("Cancel", "取消"),
    Create => ("Create", "创建"),
    SearchUserHint => ("Mixin ID or phone number", "Mixin ID 或手机号码"),
    SearchFailed => ("Couldn’t find this contact. Check the ID and try again.", "未找到联系人，请检查 ID 后重试。"),
    CommandFailed => ("Couldn’t complete this action. Try again.", "操作失败，请重试。"),
    NoContacts => ("No contacts found", "没有找到联系人"),
    Attachment => ("Send file…", "发送文件…"),
    Emoji => ("Stickers", "贴纸"),
    NoStickers => ("No stickers yet", "暂无贴纸"),
    Play => ("Play", "播放"),
    Record => ("Record a voice message", "录制语音消息"),
    Stop => ("Stop", "停止"),
    ChatInfo => ("Conversation info", "聊天信息"),
    SearchMessages => ("Search messages", "搜索消息"),
    EncryptedPlaceholder => ("End-to-end encrypted", "端对端加密"),
    ScanInstructions1 => ("Open Mixin on your phone.", "打开手机上的 Mixin。"),
    ScanInstructions2 => ("Scan the QR code and confirm sign-in.", "扫描二维码，并在手机上确认登录。"),
    Contacts => ("Contacts", "联系人"),
    Groups => ("Groups", "群组"),
    Search => ("Search", "搜索"),
    Unread => ("Unread", "未读"),
    NoChats => ("No conversations yet", "暂无会话"),
    NoChatsBody => ("Your conversations will appear here as they sync.", "同步后的会话会显示在这里。"),
    NoMatches => ("No conversations found", "未找到会话"),
    ClearFilters => ("Clear filters", "清除筛选"),
    Connecting => ("Reconnecting…", "正在重新连接…"),
    Reconnect => ("Reconnect", "重新连接"),
    LoadingChats => ("Loading conversations…", "正在加载会话…"),
    ChatsFailed => ("Couldn’t load conversations", "无法加载会话"),
    LoadingMessages => ("Loading messages…", "正在加载消息…"),
    MessagesFailed => ("Couldn’t load messages", "无法加载消息"),
    NoMessages => ("Start the conversation", "开始聊天"),
    NoMessagesBody => ("Send a message to say hello.", "发一条消息，打个招呼。"),
    MessagePlaceholder => ("Write a message…", "输入消息…"),
    Send => ("Send", "发送"),
    Sending => ("Sending…", "正在发送…"),
    SendFailed => ("Couldn’t send. Your message is still here. Try again.", "发送失败，消息已保留，请重试。"),
    Latest => ("Back to latest", "回到最新"),
    Photo => ("Photo", "图片"),
    Video => ("Video", "视频"),
    Audio => ("Voice message", "语音消息"),
    File => ("File", "文件"),
    Sticker => ("Sticker", "贴纸"),
    Contact => ("Contact card", "联系人名片"),
    Location => ("Location", "位置"),
    Transfer => ("Transfer", "转账"),
    Post => ("Post", "文章"),
    Transcript => ("Chat history", "聊天记录"),
    Message => ("Message", "消息"),
    Draft => ("Draft", "草稿"),
    Failed => ("Failed", "失败"),
    Sent => ("Sent", "已发送"),
    Read => ("Read", "已读"),
    Pinned => ("Pinned", "已置顶"),
    Back => ("Back", "返回"),
}
