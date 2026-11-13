use super::{Command, Handle};
use crate::i18n::tr;
use nsh_core::{
    desktop::MenuRole,
    element::desktop::{MenuItem, set_application_menu},
    prelude::*,
};

fn quit_item(shell: Handle) -> MenuItem {
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        MenuItem::new(tr("Quit Mixin", "退出 Mixin"))
            .shortcut("mod+q")
            .on_select(move || shell.quit())
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        let _ = shell;
        MenuItem::role(MenuRole::Quit).label(tr("Quit Mixin", "退出 Mixin"))
    }
}

pub(super) fn mount(shell: Handle) {
    effect(move || {
        let signed = shell.signed_in.get() && !shell.blocked.get();
        let chat = signed && shell.has_chat.get();
        let action = move |en: &'static str, zh: &'static str, command: Command, enabled: bool| {
            MenuItem::new(tr(en, zh))
                .disabled(!enabled)
                .on_select(move || shell.dispatch(command))
        };
        let help = |en, zh, url: &'static str| {
            MenuItem::new(tr(en, zh)).on_select(move || {
                spawn(async move {
                    let _ = open_url(url).await;
                });
            })
        };
        let mut file = vec![
            action("New conversation…", "新建聊天…", Command::NewChat, signed).shortcut("mod+n"),
            action("New group…", "新建群组…", Command::NewGroup, signed).shortcut("mod+shift+n"),
            action("New circle…", "新建圈子…", Command::NewCircle, signed),
        ];
        #[cfg(debug_assertions)]
        file.push(action(
            "Chat backup and restore…",
            "聊天备份与恢复…",
            Command::Transfer,
            signed,
        ));
        file.extend([MenuItem::separator(), close_item(shell)]);
        set_application_menu(vec![
            MenuItem::submenu(
                "Mixin",
                [
                    MenuItem::role(MenuRole::About)
                        .label(tr("About Mixin Messenger", "关于 Mixin Messenger")),
                    MenuItem::separator(),
                    action("Settings…", "设置…", Command::Preferences, signed).shortcut("mod+,"),
                    action(
                        "Lock",
                        "锁定",
                        Command::Lock,
                        shell.signed_in.get() && shell.can_lock.get(),
                    )
                    .shortcut("mod+shift+l"),
                    MenuItem::separator(),
                    action("Quick search…", "快速搜索…", Command::QuickSearch, signed)
                        .shortcut("mod+k"),
                    MenuItem::new(tr("Hide Mixin", "隐藏 Mixin"))
                        .shortcut("mod+h")
                        .on_select(move || shell.hide()),
                    MenuItem::new(tr("Show Mixin", "显示 Mixin")).on_select(move || shell.show()),
                    MenuItem::separator(),
                    quit_item(shell),
                ],
            ),
            MenuItem::submenu(tr("File", "文件"), file),
            MenuItem::submenu(
                tr("Edit", "编辑"),
                [
                    MenuItem::role(MenuRole::Undo),
                    MenuItem::role(MenuRole::Redo),
                    MenuItem::separator(),
                    MenuItem::role(MenuRole::Cut),
                    MenuItem::role(MenuRole::Copy),
                    MenuItem::role(MenuRole::Paste),
                    MenuItem::role(MenuRole::SelectAll),
                ],
            ),
            MenuItem::submenu(
                tr("Conversation", "聊天"),
                [
                    action(
                        if shell.muted.get() {
                            "Unmute"
                        } else {
                            "Mute…"
                        },
                        if shell.muted.get() {
                            "取消静音"
                        } else {
                            "静音…"
                        },
                        Command::Mute,
                        chat,
                    ),
                    action("Search…", "搜索…", Command::SearchConversation, chat),
                    action(
                        "Delete chat…",
                        "删除聊天…",
                        Command::DeleteConversation,
                        chat,
                    ),
                    action(
                        if shell.pinned.get() { "Unpin" } else { "Pin" },
                        if shell.pinned.get() {
                            "取消置顶"
                        } else {
                            "置顶"
                        },
                        Command::Pin,
                        chat,
                    ),
                    action(
                        "Toggle chat info",
                        "显示或隐藏聊天信息",
                        Command::ToggleInfo,
                        chat,
                    ),
                ],
            ),
            MenuItem::submenu(
                tr("Window", "窗口"),
                [
                    MenuItem::role(MenuRole::Minimize),
                    MenuItem::role(MenuRole::Zoom),
                    MenuItem::separator(),
                    action(
                        "Previous conversation",
                        "上一个聊天",
                        Command::PreviousChat,
                        signed,
                    )
                    .shortcut("mod+up")
                    .shortcut_when(nsh_core::keymap::When::NotEditing),
                    action("Next conversation", "下一个聊天", Command::NextChat, signed)
                        .shortcut("mod+down")
                        .shortcut_when(nsh_core::keymap::When::NotEditing),
                    MenuItem::separator(),
                    MenuItem::new(tr("Show Mixin", "显示 Mixin"))
                        .shortcut("mod+o")
                        .on_select(move || shell.show()),
                ],
            ),
            MenuItem::submenu(
                tr("Help", "帮助"),
                [
                    help("Help center", "帮助中心", "https://support.mixin.one/"),
                    help(
                        "Terms of service",
                        "服务条款",
                        "https://mixin.one/pages/terms",
                    ),
                    help(
                        "Privacy policy",
                        "隐私政策",
                        "https://mixin.one/pages/privacy",
                    ),
                ],
            ),
        ]);
    });
}

fn close_item(shell: Handle) -> MenuItem {
    #[cfg(target_os = "macos")]
    {
        let _ = shell;
        // AppKit routes performClose: to the key window, including auxiliary
        // windows. The main window's keep-alive policy retains its draft.
        MenuItem::role(MenuRole::Close).label(tr("Close window", "关闭窗口"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        MenuItem::new(tr("Close window", "关闭窗口"))
            .shortcut("mod+w")
            .on_select(move || shell.hide())
    }
}
