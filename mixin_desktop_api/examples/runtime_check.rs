//! Explicit, read-only live check through the public API using an isolated mixtest account.
//! Set MIXIN_DESKTOP_DATA_DIR. Pass --login to authorize through mixtest's macos target.
use std::{
    error::Error,
    path::{Path, PathBuf},
    time::Duration,
};

use futures::StreamExt as _;
use mixin_desktop_api::DesktopClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let directory = PathBuf::from(
        std::env::var_os("MIXIN_DESKTOP_DATA_DIR")
            .filter(|path| !path.is_empty())
            .ok_or("MIXIN_DESKTOP_DATA_DIR is required")?,
    );
    if !directory.is_absolute() || directory == Path::new("/") {
        return Err("an absolute isolated data directory is required".into());
    }
    let desktop = DesktopClient::open().await?;
    let account = if std::env::args().any(|argument| argument == "--login") {
        let login = desktop.begin_login().await?;
        let status = tokio::process::Command::new("mixtest")
            .args([
                "link",
                "-auto",
                "-target",
                "macos",
                "-url",
                &login.auth_url(),
            ])
            .status()
            .await?;
        if !status.success() {
            login.cancel();
            return Err("mixtest authorization failed".into());
        }
        tokio::time::timeout(Duration::from_secs(60), login.wait()).await??
    } else {
        desktop.restore_account().await?
    };
    let result = async {
        let profile = account.profile();
        if !profile.full_name.starts_with("mixtest-") {
            return Err::<(), Box<dyn Error>>("live checks require a mixtest account".into());
        }
        let mut connection = Box::pin(account.connection_status());
        tokio::time::timeout(Duration::from_secs(30), async {
            while let Some(connected) = connection.next().await {
                if connected { return Ok::<_, Box<dyn Error>>(()); }
            }
            Err("connection stream ended".into())
        }).await??;
        let refreshed = account.refresh_profile().await?;
        assert_eq!(refreshed.user_id, profile.user_id);
        let conversations = account.conversation().conversations("chats".into(), None, String::new(), false, 100, 0).await?;
        let count = account.conversation().conversation_count("chats".into(), None, String::new(), false).await?;
        assert!(count >= conversations.len() as i64);
        let mut checked_messages = 0;
        for conversation in conversations.iter().take(3) {
            let messages = account.message().messages(conversation.conversation_id.clone(), None, None, 20).await?;
            assert!(messages.iter().all(|message| message.conversation_id == conversation.conversation_id));
            checked_messages += messages.len();
            if let Some(message) = messages.last() {
                let window = account.message().messages_around(conversation.conversation_id.clone(), message.message_id.clone(), 2, 2).await?;
                assert!(window.iter().any(|item| item.message_id == message.message_id));
            }
        }
        println!("PASS: mixtest profile, live Blaze connection, conversation count/page, message windows; {count} conversations, {checked_messages} messages inspected");
        Ok(())
    }.await;
    tokio::time::timeout(Duration::from_secs(15), account.shutdown()).await?;
    println!("PASS: bounded account shutdown");
    result
}
