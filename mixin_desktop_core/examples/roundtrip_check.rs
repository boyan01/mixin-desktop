//! Explicit live acceptance tool. It mutates only two caller-selected mixtest accounts.
//! Run with MIXIN_DESKTOP_DATA_DIR=<isolated desktop dir> and --allow-mixtest-mutations.
//! The peer uses its existing primary session, preserving its linked desktop session.
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::{anyhow, bail, Context, Result};
use base64ct::{Base64, Encoding};
use mixin_desktop_core::{
    core::model::auth::AuthService,
    db::{
        app::{AppDatabase, Auth},
        SignalDatabase,
    },
    runtime::{desktop::DesktopRuntime, model::MessageListView, AccountRuntime},
};
use sdk::{Client, Credential, KeyStore};
use serde_json::{json, Value};

const DEADLINE: Duration = Duration::from_secs(90);

#[path = "roundtrip_check/cancel_retry.rs"]
mod cancel_retry;
#[path = "roundtrip_check/export_files.rs"]
mod export_files;

#[tokio::main]
async fn main() -> Result<()> {
    if !std::env::args().any(|arg| arg == "--allow-mixtest-mutations") {
        bail!("pass --allow-mixtest-mutations after authorizing test-account sends");
    }
    let directory = PathBuf::from(
        std::env::var_os("MIXIN_DESKTOP_DATA_DIR")
            .filter(|path| !path.is_empty())
            .ok_or_else(|| anyhow!("isolated MIXIN_DESKTOP_DATA_DIR is required"))?,
    );
    if !directory.is_absolute() || !directory.join("app.db").is_file() {
        bail!("use an existing isolated desktop login directory");
    }
    let peer_name = std::env::args()
        .skip_while(|arg| arg != "--peer")
        .nth(1)
        .ok_or_else(|| anyhow!("--peer <mixtest-name> is required"))?;
    if !peer_name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    {
        bail!("invalid mixtest name");
    }
    let desktop = DesktopRuntime::open().await?;
    let secondary = desktop
        .restore_account()
        .await?
        .ok_or_else(|| anyhow!("desktop session missing"))?;
    if !secondary
        .account()
        .full_name
        .as_deref()
        .unwrap_or_default()
        .starts_with("mixtest-")
    {
        secondary.shutdown().await;
        bail!("desktop must be a mixtest account");
    }
    let result = run(&directory, &peer_name, &secondary).await;
    tokio::time::timeout(Duration::from_secs(15), secondary.shutdown()).await?;
    result
}

async fn run(directory: &Path, peer_name: &str, secondary: &Arc<AccountRuntime>) -> Result<()> {
    let home = std::env::var_os("MIXTEST_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").expect("home directory")).join(".mixin/mixtest")
        });
    let peer: Value = serde_json::from_slice(&std::fs::read(
        home.join("accounts").join(format!("{peer_name}.json")),
    )?)?;
    let field = |name: &str| {
        peer.get(name)
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow!("mixtest account is missing {name}"))
    };
    if field("name")? != peer_name || !field("full_name")?.starts_with("mixtest-") {
        bail!("counterpart must be a mixtest account");
    }
    let user_id = field("user_id")?.to_string();
    if user_id == secondary.account_id() {
        bail!("two distinct test accounts are required");
    }
    let private_key = Base64::decode_vec(field("session_seed")?)?;
    let client = Client::new_with_user_agent(
        Credential::KeyStore(KeyStore {
            app_id: user_id.clone(),
            session_id: field("session_id")?.to_string(),
            session_private_key: base16ct::lower::encode_string(&private_key),
            server_public_key: String::new(),
            scp: "FULL".into(),
        }),
        Some(mixin_desktop_core::core::user_agent::generate_user_agent()),
    );
    let account = client.account_api.get_me().await?;
    if account.user_id != user_id
        || account.identity_number != field("identity_number")?
        || !account
            .full_name
            .as_deref()
            .unwrap_or_default()
            .starts_with("mixtest-")
    {
        bail!("primary credential does not match the selected test account");
    }
    let signal = SignalDatabase::connect(account.identity_number.clone())
        .await
        .map_err(|error| anyhow!("open peer signal database: {error}"))?;
    if let Some(identity) = signal.identity_dao.get_local_identity().await? {
        if identity.public_key != Base64::decode_vec(field("identity_public")?)?
            || identity.private_key.as_deref()
                != Some(Base64::decode_vec(field("identity_private")?)?.as_slice())
        {
            bail!("isolated primary Signal identity does not match the selected mixtest account");
        }
    } else {
        let registration = peer
            .get("registration_id")
            .and_then(Value::as_u64)
            .ok_or_else(|| anyhow!("test identity registration is missing"))?;
        signal
            .init(
                u32::try_from(registration)?,
                Some(&Base64::decode_vec(field("identity_private")?)?),
            )
            .await?;
    }
    drop(signal);
    let app = Arc::new(AppDatabase::connect_at(directory.join("roundtrip-peer-app.db")).await?);
    let auth_service = Arc::new(AuthService::new(app.clone()));
    let primary = Arc::new(
        AccountRuntime::start(
            Auth {
                user_id,
                private_key,
                primary_session_id: None,
                account,
            },
            auth_service,
            app.setting_dao.clone(),
        )
        .await?,
    );
    let result = check_exchange(directory, &primary, secondary).await;
    tokio::time::timeout(Duration::from_secs(15), primary.shutdown()).await?;
    result
}

async fn connected(account: &AccountRuntime) -> Result<()> {
    let mut changes = account.subscribe_connection_status();
    tokio::time::timeout(DEADLINE, async {
        while !*changes.borrow_and_update() {
            changes.changed().await?;
        }
        Ok::<_, anyhow::Error>(())
    })
    .await
    .context("Blaze connection deadline")?
}

async fn message_matching(
    account: &AccountRuntime,
    id: &str,
    condition: impl Fn(&MessageListView) -> bool,
) -> Result<MessageListView> {
    let mut changes = account.subscribe_conversation_changes();
    tokio::time::timeout(DEADLINE, async {
        loop {
            let message = account
                .message_access()
                .message_items_by_ids(vec![id.to_string()])
                .await?
                .into_iter()
                .find(|message| condition(message));
            if let Some(message) = message {
                return Ok::<_, anyhow::Error>(message);
            }
            if let Err(tokio::sync::broadcast::error::RecvError::Closed) = changes.recv().await {
                bail!("change stream closed");
            }
        }
    })
    .await
    .context("message state deadline")?
}

async fn check_exchange(
    directory: &Path,
    primary: &AccountRuntime,
    secondary: &AccountRuntime,
) -> Result<()> {
    tokio::try_join!(connected(primary), connected(secondary))?;
    if std::env::args().any(|argument| argument == "--cancel-retry-image") {
        return cancel_retry::run(directory, primary, secondary).await;
    }
    if let Some(evidence) = std::env::args()
        .skip_while(|argument| argument != "--export-files")
        .nth(1)
    {
        return export_files::run(directory, Path::new(&evidence), secondary).await;
    }
    if std::env::args().any(|argument| argument == "--transcript") {
        let path = std::env::args()
            .skip_while(|argument| argument != "--transcript")
            .nth(1)
            .ok_or_else(|| {
                anyhow!("--transcript <original-roundtrip-evidence.json> is required")
            })?;
        return transcript_exchange(directory, Path::new(&path), primary, secondary).await;
    }

    if let Some(path) = std::env::args().skip_while(|arg| arg != "--resume").nth(1) {
        return resume_exchange(Path::new(&path), primary, secondary).await;
    }
    let conversation = primary
        .conversation_access()
        .open_user_conversation(secondary.account_id().to_string())
        .await?;
    let secondary_conversation = secondary
        .conversation_access()
        .open_user_conversation(primary.account_id().to_string())
        .await?;
    if conversation != secondary_conversation {
        bail!("direct conversation IDs differ");
    }
    let run = uuid::Uuid::new_v4().to_string();
    let marker = format!("NSH migration acceptance {run}");
    let first_content = format!("{marker} primary to desktop");
    let first = primary
        .message_access()
        .send_text(conversation.clone(), first_content.clone(), None, false)
        .await?;
    message_matching(secondary, &first, |message| {
        message.category == "SIGNAL_TEXT" && message.content == first_content
    })
    .await?;
    secondary
        .message_access()
        .mark_conversation_read(conversation.clone())
        .await?;
    message_matching(primary, &first, |message| message.status == "READ").await?;
    println!("PASS: primary-to-desktop encrypted text decrypted and read receipt persisted");
    let second_content = format!("{marker} desktop to primary");
    let second = secondary
        .message_access()
        .send_text(conversation.clone(), second_content.clone(), None, false)
        .await?;
    message_matching(primary, &second, |message| {
        message.category == "SIGNAL_TEXT" && message.content == second_content
    })
    .await?;
    primary
        .message_access()
        .mark_conversation_read(conversation.clone())
        .await?;
    message_matching(secondary, &second, |message| message.status == "READ").await?;
    println!("PASS: desktop-to-primary encrypted text decrypted and read receipt persisted");

    let assets = directory.join("roundtrip-fixtures");
    std::fs::create_dir_all(&assets)?;
    let image_path = assets.join(format!("NSH migration acceptance {run}.png"));
    image::RgbImage::from_fn(32, 32, |x, y| image::Rgb([x as u8 * 7, y as u8 * 7, 128]))
        .save(&image_path)?;
    let image_bytes = std::fs::read(&image_path)?;
    let image = secondary
        .message_access()
        .send_attachment_receipt(
            conversation.clone(),
            image_path.to_string_lossy().into_owned(),
            "IMAGE".into(),
            "image/png".into(),
            None,
            Some(32),
            Some(32),
            None,
            None,
            Some(marker.clone()),
            None,
            false,
        )
        .await?;
    if let Some(error) = image.upload_error {
        bail!("image upload failed after acceptance: {error}");
    }
    check_received_file(primary, &image.message_id, "SIGNAL_IMAGE", &image_bytes).await?;
    println!("PASS: encrypted PNG upload/download preserved source bytes");

    let voice_path = assets.join(format!("NSH migration acceptance {run}.ogg"));
    let encoded = tokio::process::Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=0.5",
            "-ac",
            "1",
            "-ar",
            "48000",
            "-c:a",
            "libopus",
        ])
        .arg(&voice_path)
        .output()
        .await?;
    if !encoded.status.success() {
        bail!("could not create the test Opus fixture");
    }
    let voice_bytes = std::fs::read(&voice_path)?;
    let waveform = (0..64)
        .map(|index| if index % 2 == 0 { 128 } else { 255 })
        .collect();
    let voice = secondary
        .message_access()
        .send_audio_receipt(
            conversation.clone(),
            voice_path.to_string_lossy().into_owned(),
            500,
            waveform,
            None,
        )
        .await?;
    if let Some(error) = voice.upload_error {
        bail!("audio upload failed after acceptance: {error}");
    }
    check_received_file(primary, &voice.message_id, "SIGNAL_AUDIO", &voice_bytes).await?;
    println!("PASS: encrypted Ogg/Opus upload/download preserved source bytes");
    primary
        .message_access()
        .mark_conversation_read(conversation.clone())
        .await?;
    message_matching(secondary, &image.message_id, |message| {
        message.status == "READ"
    })
    .await?;
    message_matching(secondary, &voice.message_id, |message| {
        message.status == "READ"
    })
    .await?;
    let report = json!({"run":run,"conversation_id":conversation,"primary":primary.account_id(),"secondary":secondary.account_id(),
        "text_ids":[first,second],"image_id":image.message_id,"voice_id":voice.message_id,
        "image_bytes":image_bytes.len(),"voice_bytes":voice_bytes.len(),"result":"passed"});
    let report_path = directory.join("roundtrip-results.json");
    use std::io::Write as _;
    #[cfg(unix)]
    use std::os::unix::fs::OpenOptionsExt as _;
    let mut options = std::fs::OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(&report_path)?;
    file.write_all(serde_json::to_string_pretty(&report)?.as_bytes())?;
    println!("PASS: all four messages have durable READ receipts; private result manifest written");
    Ok(())
}

async fn check_received_file(
    account: &AccountRuntime,
    id: &str,
    category: &str,
    expected: &[u8],
) -> Result<()> {
    let received = message_matching(account, id, |message| message.category == category).await?;
    if received.media_status == "CANCELED" {
        let result = account
            .attachment_access()
            .download_attachment(id.to_string())
            .await;
        if let Err(error) = result {
            if !error
                .to_string()
                .starts_with("attachment download is already active:")
            {
                return Err(error);
            }
        }
    }
    let received = message_matching(account, id, |message| {
        matches!(message.media_status.as_str(), "DONE" | "READ")
    })
    .await?;
    let path = received
        .media_url
        .ok_or_else(|| anyhow!("downloaded attachment has no path"))?;
    let actual = std::fs::read(path)?;
    if actual != expected {
        bail!("downloaded attachment differs from original bytes");
    }
    Ok(())
}

/// Continues the accepted message IDs after a product fix; never sends a duplicate.
async fn resume_exchange(
    evidence: &Path,
    primary: &AccountRuntime,
    secondary: &AccountRuntime,
) -> Result<()> {
    let report: Value = serde_json::from_slice(&std::fs::read(evidence)?)?;
    let accounts = report
        .get("accounts")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("resume evidence missing accounts"))?;
    let secondary_rows = accounts
        .iter()
        .find(|account| {
            account.get("identity_number").and_then(Value::as_str)
                == Some(secondary.account().identity_number.as_str())
        })
        .and_then(|account| account.get("messages"))
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("resume evidence does not match the desktop account"))?;
    let audio = secondary_rows
        .iter()
        .find(|row| row.get("category").and_then(Value::as_str) == Some("SIGNAL_AUDIO"))
        .ok_or_else(|| anyhow!("resume evidence has no accepted audio"))?;
    let id = audio
        .get("message_id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("resume audio ID missing"))?;
    let conversation = audio
        .get("conversation_id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("resume conversation missing"))?;
    if sdk::generate_conversation_id(primary.account_id(), secondary.account_id()).to_string()
        != conversation
    {
        bail!("resume conversation does not belong to the two selected test accounts");
    }
    let source = report
        .get("source_files")
        .and_then(Value::as_array)
        .and_then(|files| {
            files.iter().find_map(|file| {
                file.get("path")
                    .and_then(Value::as_str)
                    .filter(|path| path.ends_with(".ogg"))
            })
        })
        .ok_or_else(|| anyhow!("resume audio source missing"))?;
    check_received_file(primary, id, "SIGNAL_AUDIO", &std::fs::read(source)?).await?;
    let audio = message_matching(primary, id, |_| true).await?;
    if audio.media_duration != "500" || audio.media_mime_type.as_deref() != Some("audio/ogg") {
        bail!("received audio metadata differs from the accepted fixture");
    }
    let waveform = audio
        .media_waveform
        .as_deref()
        .map(Base64::decode_vec)
        .transpose()?
        .unwrap_or_default();
    let expected_waveform: Vec<u8> = (0..64)
        .map(|index| if index % 2 == 0 { 128 } else { 255 })
        .collect();
    if waveform != expected_waveform {
        bail!("received audio waveform differs from the accepted fixture");
    }
    primary
        .message_access()
        .mark_conversation_read(conversation.to_string())
        .await?;
    for row in secondary_rows {
        if let Some(id) = row.get("message_id").and_then(Value::as_str) {
            message_matching(secondary, id, |message| message.status == "READ").await?;
        }
    }
    println!("PASS: resumed original Ogg/Opus message delivered, bytes/duration/MIME/waveform preserved, all original messages READ");
    Ok(())
}

/// Sends only the three previously accepted fixtures, bundled and then forwarded back.
/// No new recipients, plain bots, text, images, or audio are created by this opt-in path.
async fn transcript_exchange(
    directory: &Path,
    evidence: &Path,
    primary: &AccountRuntime,
    secondary: &AccountRuntime,
) -> Result<()> {
    let evidence: Value = serde_json::from_slice(&std::fs::read(evidence)?)?;
    let evidence_directory = evidence
        .get("data_directory")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("transcript evidence missing isolated data directory"))?;
    if std::fs::canonicalize(evidence_directory)? != std::fs::canonicalize(directory)? {
        bail!("transcript evidence belongs to another isolated data directory");
    }
    let accounts = evidence
        .get("accounts")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("transcript evidence missing accepted accounts"))?;
    for account in [primary, secondary] {
        if !accounts.iter().any(|row| {
            row.get("identity_number").and_then(Value::as_str)
                == Some(account.account().identity_number.as_str())
        }) {
            bail!("transcript evidence does not cover both selected mixtest accounts");
        }
    }
    let rows = accounts
        .iter()
        .find(|row| {
            row.get("identity_number").and_then(Value::as_str)
                == Some(secondary.account().identity_number.as_str())
        })
        .and_then(|row| row.get("messages"))
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("transcript evidence has no original desktop messages"))?;
    let mut ids = Vec::new();
    for category in ["SIGNAL_TEXT", "SIGNAL_IMAGE", "SIGNAL_AUDIO"] {
        ids.push(
            rows.iter()
                .find(|row| row.get("category").and_then(Value::as_str) == Some(category))
                .and_then(|row| row.get("message_id"))
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("transcript evidence missing {category}"))?
                .to_string(),
        );
    }
    let conversation =
        sdk::generate_conversation_id(primary.account_id(), secondary.account_id()).to_string();
    let mut sources = secondary
        .message_access()
        .message_items_by_ids(ids.clone())
        .await?;
    if sources.len() != 3
        || sources
            .iter()
            .any(|row| row.conversation_id != conversation || row.status != "READ")
    {
        bail!(
            "original fixtures are missing, unread, or outside the selected mixtest conversation"
        );
    }
    let text = sources
        .iter()
        .find(|row| row.category == "SIGNAL_TEXT")
        .ok_or_else(|| anyhow!("accepted marker text missing"))?;
    if !text.content.starts_with("NSH migration acceptance ") {
        bail!("original text lacks the authorized acceptance marker");
    }
    let marker = text
        .content
        .split_whitespace()
        .take(4)
        .collect::<Vec<_>>()
        .join(" ");
    let image = sources
        .iter()
        .find(|row| row.category == "SIGNAL_IMAGE")
        .ok_or_else(|| anyhow!("accepted image missing"))?;
    if image.caption.as_deref() != Some(marker.as_str()) {
        bail!("original image does not have the accepted marker caption");
    }
    let source_files = evidence
        .get("source_files")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("transcript evidence has no accepted fixture paths"))?;
    let mut expected_files = std::collections::HashMap::new();
    for (category, suffix) in [("SIGNAL_IMAGE", ".png"), ("SIGNAL_AUDIO", ".ogg")] {
        let path = source_files
            .iter()
            .filter_map(|row| row.get("path").and_then(Value::as_str))
            .find(|path| path.ends_with(suffix))
            .ok_or_else(|| anyhow!("accepted fixture missing {suffix}"))?;
        let path = std::fs::canonicalize(path)?;
        if !path.starts_with(std::fs::canonicalize(directory.join("roundtrip-fixtures"))?)
            || !path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(&marker))
        {
            bail!("fixture path is outside the isolated accepted fixture directory");
        }
        let source = sources
            .iter()
            .find(|row| row.category == category)
            .ok_or_else(|| anyhow!("accepted fixture missing source row"))?;
        let bytes = std::fs::read(path)?;
        if source
            .media_url
            .as_deref()
            .map(std::fs::read)
            .transpose()?
            .as_deref()
            != Some(bytes.as_slice())
        {
            bail!("original downloaded fixture bytes differ from the accepted source");
        }
        expected_files.insert(source.message_id.clone(), bytes);
    }
    if std::env::args().any(|argument| argument == "--forward-batch") {
        return batch_forward_exchange(
            directory,
            primary,
            secondary,
            &conversation,
            &marker,
            &sources,
            &expected_files,
        )
        .await;
    }
    sources.sort_by(|left, right| {
        left.created_at_micros
            .cmp(&right.created_at_micros)
            .then_with(|| left.message_id.cmp(&right.message_id))
    });
    let report_path = directory.join("roundtrip-transcript-results.json");
    let previous: Option<Value> = if report_path.exists() {
        let previous: Value = serde_json::from_slice(&std::fs::read(&report_path)?)?;
        if previous.get("marker").and_then(Value::as_str) != Some(marker.as_str())
            || previous.get("conversation_id").and_then(Value::as_str)
                != Some(conversation.as_str())
        {
            bail!("existing transcript acceptance report belongs to different fixtures");
        }
        Some(previous)
    } else {
        None
    };
    let combined = if let Some(id) = previous
        .as_ref()
        .and_then(|row| row.get("combined_id"))
        .and_then(Value::as_str)
    {
        id.to_owned()
    } else {
        let id = secondary
            .message_access()
            .combine_forward_messages(conversation.clone(), ids)
            .await?;
        write_transcript_report(
            &report_path,
            &json!({"result":"combined_accepted", "marker":marker,
            "conversation_id":conversation, "combined_id":id}),
        )?;
        id
    };
    check_received_transcript(primary, &combined, &sources, &expected_files).await?;
    primary
        .message_access()
        .mark_conversation_read(conversation.clone())
        .await?;
    message_matching(secondary, &combined, |row| row.status == "READ").await?;
    println!("PASS: combined encrypted transcript preserved original order, sender/time, text, PNG, Opus bytes and metadata");
    let forwarded = if let Some(id) = previous
        .as_ref()
        .and_then(|row| row.get("forwarded_id"))
        .and_then(Value::as_str)
    {
        id.to_owned()
    } else {
        let id = primary
            .message_access()
            .forward_messages(conversation.clone(), vec![combined.clone()])
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("transcript forward returned no message ID"))?;
        write_transcript_report(
            &report_path,
            &json!({"result":"forwarded_accepted", "marker":marker,
            "conversation_id":conversation, "combined_id":combined, "forwarded_id":id}),
        )?;
        id
    };
    check_received_transcript(secondary, &forwarded, &sources, &expected_files).await?;
    secondary
        .message_access()
        .mark_conversation_read(conversation.clone())
        .await?;
    message_matching(primary, &forwarded, |row| row.status == "READ").await?;
    write_transcript_report(
        &report_path,
        &json!({"result":"passed", "marker":marker,
        "conversation_id":conversation, "combined_id":combined, "forwarded_id":forwarded,
        "checks":["combined_SIGNAL_transcript", "forwarded_SIGNAL_transcript", "original_sender_time_order",
            "marker_text", "PNG_Opus_byte_identity", "audio_metadata_waveform", "READ_receipts"],
        "cross_prefix":"hermetic_tests_only"}),
    )?;
    println!("PASS: transcript forwarded back with attachment bytes/metadata preserved and both READ receipts persisted");
    Ok(())
}

async fn batch_forward_exchange(
    directory: &Path,
    primary: &AccountRuntime,
    secondary: &AccountRuntime,
    conversation: &str,
    marker: &str,
    sources: &[MessageListView],
    files: &std::collections::HashMap<String, Vec<u8>>,
) -> Result<()> {
    let run = std::env::args()
        .skip_while(|arg| arg != "--batch-run")
        .nth(1);
    if run.as_deref().is_some_and(|run| {
        run.is_empty()
            || run.len() > 40
            || !run
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
    }) {
        bail!("batch run name must be a short ASCII label");
    }
    let report_path = directory.join(match run {
        Some(run) => format!("roundtrip-forward-batch-{run}.json"),
        None => "roundtrip-forward-batch-results.json".to_owned(),
    });
    let source_ids = sources
        .iter()
        .map(|row| row.message_id.clone())
        .collect::<Vec<_>>();
    let ids: Vec<String> = if report_path.exists() {
        let report: Value = serde_json::from_slice(&std::fs::read(&report_path)?)?;
        if report["marker"].as_str() != Some(marker)
            || report["conversation_id"].as_str() != Some(conversation)
            || report["source_ids"] != json!(source_ids)
        {
            bail!("existing batch report belongs to different accepted fixtures");
        }
        serde_json::from_value(report["accepted_ids"].clone())?
    } else {
        let ids = secondary
            .message_access()
            .forward_messages(conversation.to_owned(), source_ids.clone())
            .await?;
        write_transcript_report(
            &report_path,
            &json!({"result":"accepted", "marker":marker,
            "conversation_id":conversation, "source_ids":source_ids, "accepted_ids":ids}),
        )?;
        ids
    };
    if ids.len() != sources.len() {
        bail!("batch did not accept all selected fixtures");
    }
    for (id, source) in ids.iter().zip(sources) {
        let received = message_matching(primary, id, |row| row.category == source.category).await?;
        if let Some(bytes) = files.get(&source.message_id) {
            check_received_file(primary, id, &source.category, bytes).await?;
            let downloaded =
                message_matching(primary, id, |row| row.media_status == "DONE").await?;
            if downloaded.media_mime_type != source.media_mime_type
                || downloaded.media_duration != source.media_duration
                || downloaded.media_waveform != source.media_waveform
                || downloaded.caption != source.caption
            {
                bail!("batch attachment changed accepted source metadata");
            }
        } else if received.content != source.content {
            bail!("batch text changed accepted marker content");
        }
    }
    primary
        .message_access()
        .mark_conversation_read(conversation.to_owned())
        .await?;
    for id in &ids {
        message_matching(secondary, id, |row| row.status == "READ").await?;
    }
    write_transcript_report(
        &report_path,
        &json!({"result":"passed", "marker":marker,
        "conversation_id":conversation, "source_ids":source_ids, "accepted_ids":ids,
        "checks":["mixed_batch_text_PNG_Opus", "original_content_bytes_metadata", "all_READ"]}),
    )?;
    println!("PASS: one mixed batch accepted all three marked fixtures, preserved text/PNG/Opus bytes and audio metadata, and reached READ");
    Ok(())
}

async fn check_received_transcript(
    account: &AccountRuntime,
    id: &str,
    sources: &[MessageListView],
    expected_files: &std::collections::HashMap<String, Vec<u8>>,
) -> Result<()> {
    message_matching(account, id, |row| row.category == "SIGNAL_TRANSCRIPT").await?;
    let entries = account
        .message_access()
        .transcript_messages(id.to_string())
        .await?;
    if entries.len() != sources.len() {
        bail!("received transcript has a different number of entries");
    }
    for (entry, source) in entries.iter().zip(sources) {
        if entry.message_id != source.message_id
            || entry.sender_id != source.sender_id
            || entry.created_at_micros / 1000 != source.created_at_micros / 1000
            || entry.category != source.category
        {
            bail!("received transcript changed original order, sender, timestamp, or category");
        }
        if let Some(expected) = expected_files.get(&entry.message_id) {
            let mut changes = account.subscribe_conversation_changes();
            // A transcript may reuse the already downloaded original attachment.
            // Only CANCELED rows need an explicit download; observe any active download.
            if entry.media_status == "CANCELED" {
                account
                    .attachment_access()
                    .download_transcript_attachment(id.to_string(), entry.message_id.clone())
                    .await?;
            }
            let downloaded = tokio::time::timeout(DEADLINE, async {
                loop {
                    let row = account
                        .message_access()
                        .transcript_messages(id.to_string())
                        .await?
                        .into_iter()
                        .find(|row| row.message_id == entry.message_id)
                        .ok_or_else(|| anyhow!("downloaded transcript entry disappeared"))?;
                    if row.media_status == "DONE" {
                        return Ok::<_, anyhow::Error>(row);
                    }
                    if let Err(tokio::sync::broadcast::error::RecvError::Closed) =
                        changes.recv().await
                    {
                        bail!("change stream closed");
                    }
                }
            })
            .await
            .context("transcript attachment completion deadline")??;
            let path = downloaded
                .media_url
                .as_deref()
                .ok_or_else(|| anyhow!("transcript download missing local path"))?;
            if std::fs::read(path)? != *expected
                || downloaded.media_mime_type != source.media_mime_type
                || downloaded.media_duration != source.media_duration
                || downloaded.media_waveform != source.media_waveform
                || downloaded.media_width != source.media_width
                || downloaded.media_height != source.media_height
                || downloaded.caption != source.caption
            {
                bail!("transcript attachment bytes or presentation metadata changed");
            }
        } else if entry.content != source.content {
            bail!("transcript changed the authorized marker text");
        }
    }
    message_matching(account, id, |row| row.media_status == "DONE").await?;
    Ok(())
}

fn write_transcript_report(path: &Path, report: &Value) -> Result<()> {
    use std::io::Write as _;
    #[cfg(unix)]
    use std::os::unix::fs::OpenOptionsExt as _;
    let mut options = std::fs::OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    options
        .open(path)?
        .write_all(serde_json::to_string_pretty(report)?.as_bytes())?;
    Ok(())
}
