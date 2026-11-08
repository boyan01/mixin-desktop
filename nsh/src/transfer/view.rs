use nsh_core::prelude::*;

use super::{Command, Direction, Handle, Status};
use crate::{
    i18n::{self, I18n, Key},
    ui::parts,
    ui::{self, Typography as _},
};

pub(crate) fn screen(handle: Handle) -> impl IntoElement {
    let i18n = i18n::current();
    scroll().flex_1().min_h_0().child(
        parts::page_body()
            .child(parts::note(move || {
                i18n.tr(
                    "Keep both devices on the same network and open Mixin on your phone. Confirm the transfer on the other device.",
                    "请将两台设备连接到同一网络，并打开手机上的 Mixin。在另一台设备上确认同步请求。",
                )
            }))
            .child(
                ui::card().children(
                    [Direction::Restore, Direction::Backup]
                        .into_iter()
                        .map(move |direction| transfer_row(handle, direction)),
                ),
            ),
    )
}

fn transfer_row(handle: Handle, direction: Direction) -> impl IntoElement {
    let i18n = i18n::current();
    let job = handle.job(direction);
    let icon = match direction {
        Direction::Restore => ui::Icon::Download,
        Direction::Backup => ui::Icon::Upload,
    };
    view()
        .child(
            ui::settings_row(Some(icon), move || title(i18n, direction))
                .on_press(move || {
                    handle.send(
                        direction,
                        match direction {
                            Direction::Restore => Command::PullToRemote,
                            Direction::Backup => Command::PushToRemote,
                        },
                    )
                })
                .disabled(move || {
                    handle.restore_busy.get()
                        || handle.backup_busy.get()
                        || handle.restore.get().status.active()
                        || handle.backup.get().status.active()
                }),
        )
        .child(when(
            move || job.get().status != Status::Idle,
            move || {
                view()
                    .px(ui::space_lg())
                    .pb(ui::space_md())
                    .gap(ui::space_sm())
                    .child(status(handle, direction))
                    .child(when(
                        move || job.get().status.active(),
                        move || {
                            ui::action(
                                move || i18n.tr("Show transfer", "查看同步进度"),
                                move || handle.visible.set(Some(direction)),
                            )
                        },
                    ))
            },
        ))
}

pub(crate) fn surface(handle: Handle) -> impl IntoElement {
    let current = computed(move || {
        if let Some(id) = handle.restore_request.get() {
            Some((Some(id), Direction::Restore))
        } else if let Some(id) = handle.backup_request.get() {
            Some((Some(id), Direction::Backup))
        } else {
            handle.visible.get().map(|direction| (None, direction))
        }
    });
    let shown = signal(None::<(Option<u64>, Direction)>);
    effect(move || {
        if let Some(current) = current.get() {
            shown.set(Some(current));
        }
    });
    let presence = crate::motion::Presence::modal(move || current.with(Option::is_some));
    when(
        move || presence.mounted(),
        move || {
            cleanup(move || {
                let _ = shown.try_set(None);
            });
            dynamic(move || {
                let shown = shown.get();
                untracked(|| match shown {
                    Some((Some(id), direction)) => {
                        approval(handle, direction, id, presence).into_element()
                    }
                    Some((None, direction)) => progress(handle, direction, presence).into_element(),
                    None => view().hidden().into_element(),
                })
            })
        },
    )
}

fn title(i18n: I18n, direction: Direction) -> &'static str {
    match direction {
        Direction::Restore => i18n.tr("Sync from phone", "从手机同步"),
        Direction::Backup => i18n.tr("Sync to phone", "同步到手机"),
    }
}

fn approval(
    handle: Handle,
    direction: Direction,
    request_id: u64,
    presence: crate::motion::Presence,
) -> impl IntoElement {
    let i18n = i18n::current();
    let reject = move || {
        if !presence.interactive() {
            return;
        }
        handle.send_request(
            direction,
            match direction {
                Direction::Restore => Command::CancelRestoreRequest,
                Direction::Backup => Command::CancelBackupRequest,
            },
            request_id,
        )
    };
    // Only the buttons answer: a stray press outside must not refuse the request.
    ui::dialog(
        presence,
        || {},
        ui::dialog_card(ui::DialogWidth::Regular)
            .child(ui::dialog_title(move || title(i18n, direction)))
            .child(
                text(move || match direction {
                    Direction::Restore => i18n.tr(
                        "Allow your phone to send chat history to this device?",
                        "允许手机向此设备同步聊天记录吗？",
                    ),
                    Direction::Backup => i18n.tr(
                        "Allow this device to send chat history to your phone?",
                        "允许此设备向手机同步聊天记录吗？",
                    ),
                })
                .text_role(ui::TextRole::Body),
            )
            .child(
                parts::dialog_actions()
                    .child(
                        ui::action(move || i18n.text(Key::Cancel), reject)
                            .disabled(handle.busy(direction)),
                    )
                    .child(
                        ui::primary()
                            .disabled(handle.busy(direction))
                            .child(text(move || i18n.tr("Allow", "允许")))
                            .on_press(move || {
                                if !presence.interactive() {
                                    return;
                                }
                                handle.send_request(
                                    direction,
                                    match direction {
                                        Direction::Restore => Command::ConfirmRestore,
                                        Direction::Backup => Command::ConfirmBackup,
                                    },
                                    request_id,
                                )
                            }),
                    ),
            )
            .child(when(
                move || handle.response_error(direction).get() == Some(request_id),
                move || {
                    ui::error_text(move || {
                        i18n.tr(
                            "Couldn’t respond to the transfer request. Try again.",
                            "无法回应同步请求，请重试。",
                        )
                    })
                },
            )),
    )
}

fn progress(
    handle: Handle,
    direction: Direction,
    presence: crate::motion::Presence,
) -> impl IntoElement {
    let i18n = i18n::current();
    let job = handle.job(direction);
    ui::dialog(
        presence,
        move || {
            if !job.get_untracked().status.active() {
                handle.visible.set(None);
            }
        },
        ui::dialog_card(ui::DialogWidth::Regular)
            .child(ui::dialog_title(move || title(i18n, direction)))
            .child(status(handle, direction))
            .child(when(
                move || job.get().status.active(),
                move || {
                    view()
                        .gap(ui::space_lg())
                        .child(
                            text(move || {
                                i18n.tr(
                                    "Keep both devices connected until the transfer finishes.",
                                    "同步完成前请保持两台设备联网。",
                                )
                            })
                            .text_role(ui::TextRole::Body)
                            .text_color(ui::fg_secondary),
                        )
                        .child(
                            ui::action(
                                move || i18n.text(Key::Cancel),
                                move || handle.cancel(direction),
                            )
                            .disabled(handle.busy(direction)),
                        )
                },
            ))
            .child(when(
                move || !job.get().status.active(),
                move || {
                    ui::action(
                        move || i18n.tr("Done", "完成"),
                        move || handle.visible.set(None),
                    )
                },
            )),
    )
}

fn status(handle: Handle, direction: Direction) -> impl IntoElement {
    let i18n = i18n::current();
    let job = handle.job(direction);
    view()
        .gap(ui::space_sm())
        .child(
            text(move || match job.get().status {
                Status::Idle => "",
                Status::Waiting => i18n.tr("Waiting for the other device…", "等待另一台设备连接…"),
                Status::Transferring => i18n.tr("Transferring chats…", "正在同步聊天记录…"),
                Status::Completed => i18n.tr("Transfer completed", "同步完成"),
                Status::Failed => i18n.tr(
                    "Transfer failed. Check both devices and try again.",
                    "同步失败，请检查两台设备后重试。",
                ),
                Status::VersionMismatch => i18n.tr(
                    "The devices use different transfer versions. Update both apps and try again.",
                    "两台设备的同步协议版本不同，请更新应用后重试。",
                ),
                Status::Cancelled => i18n.tr("Transfer cancelled", "同步已取消"),
            })
            .text_role(ui::TextRole::Body),
        )
        .child(when(
            move || job.get().status == Status::Transferring,
            move || {
                text(move || {
                    let current = job.get();
                    format!(
                        "{:.1}% · {:.1} KB/s",
                        current.progress,
                        current.speed / 1024.0
                    )
                })
                .text_role(ui::TextRole::Body)
                .text_color(ui::fg_secondary)
            },
        ))
}
