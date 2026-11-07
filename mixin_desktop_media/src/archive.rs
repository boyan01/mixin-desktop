//! Stream selected local files into a temporary ZIP without changing originals.
use crate::{MediaError, MediaResult, PreparedAttachment};
use std::{
    collections::HashSet,
    fs::File,
    io,
    path::{Path, PathBuf},
};
use zip::{write::SimpleFileOptions, ZipWriter};

pub async fn prepare(
    items: Vec<PreparedAttachment>,
    password: Option<String>,
) -> MediaResult<PreparedAttachment> {
    tokio::task::spawn_blocking(move || create(items, password.as_deref()))
        .await
        .map_err(|error| MediaError::InvalidAttachment(error.to_string()))?
}

fn create(
    items: Vec<PreparedAttachment>,
    password: Option<&str>,
) -> MediaResult<PreparedAttachment> {
    if items.is_empty() {
        return Err(MediaError::InvalidAttachment(
            "Choose files to archive".into(),
        ));
    }
    let temporary = tempfile::Builder::new()
        .prefix("mixin-archive-")
        .suffix(".zip")
        .tempfile()?;
    let (output, temporary) = temporary.into_parts();
    let mut archive = ZipWriter::new(output);
    let mut names = HashSet::new();
    let mut entries = 0_usize;
    for item in &items {
        let source = PathBuf::from(&item.path);
        if !source.is_absolute() {
            return Err(MediaError::InvalidAttachment(
                "Archive sources must be absolute local paths".into(),
            ));
        }
        let filename = item.name.clone();
        validate_name(&filename)?;
        let mut name = filename.clone();
        let mut suffix = 2;
        while !names.insert(name.clone()) {
            let filename = Path::new(&filename);
            let stem = filename.file_stem().unwrap_or_default().to_string_lossy();
            let extension = filename
                .extension()
                .map(|ext| format!(".{}", ext.to_string_lossy()))
                .unwrap_or_default();
            name = format!("{stem} ({suffix}){extension}");
            suffix += 1;
        }
        append(
            &mut archive,
            &source,
            &name,
            0,
            &mut entries,
            password.filter(|value| !value.is_empty()),
        )?;
    }
    let output = archive.finish().map_err(zip_error)?;
    let size = output.metadata()?.len();
    drop(output);
    let mut item = PreparedAttachment::default();
    item.path = temporary.to_string_lossy().into_owned();
    item.name = "Archive.zip".into();
    item.kind = "DATA".into();
    item.mime_type = "application/zip".into();
    item.size = size;
    item.retain_temporary(temporary);
    Ok(item)
}

fn append(
    archive: &mut ZipWriter<File>,
    source: &Path,
    name: &str,
    depth: usize,
    entries: &mut usize,
    password: Option<&str>,
) -> MediaResult<()> {
    if depth > 64 || *entries >= 10_000 {
        return Err(MediaError::InvalidAttachment(
            "An archive supports up to 10,000 entries and 64 directory levels".into(),
        ));
    }
    let metadata = source.symlink_metadata()?;
    if metadata.file_type().is_symlink() || !(metadata.is_file() || metadata.is_dir()) {
        return Err(MediaError::InvalidAttachment(
            "Choose regular files and folders; symbolic links and special files cannot be archived"
                .into(),
        ));
    }
    *entries += 1;
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(if metadata.is_dir() { 0o755 } else { 0o644 })
        .large_file(metadata.len() > u32::MAX as u64);
    if metadata.is_dir() {
        archive
            .add_directory(format!("{name}/"), options)
            .map_err(zip_error)?;
        let mut children = source
            .read_dir()?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()?;
        children.sort();
        for child in children {
            let basename = child
                .file_name()
                .ok_or_else(|| MediaError::InvalidAttachment("File has no name".into()))?
                .to_string_lossy();
            validate_name(&basename)?;
            append(
                archive,
                &child,
                &format!("{name}/{basename}"),
                depth + 1,
                entries,
                password,
            )?;
        }
    } else {
        let options = match password {
            Some(password) => options.with_aes_encryption(zip::AesMode::Aes256, password),
            None => options,
        };
        archive.start_file(name, options).map_err(zip_error)?;
        io::copy(&mut File::open(source)?, archive)?;
    }
    Ok(())
}

fn validate_name(name: &str) -> MediaResult<()> {
    let drive_prefix = name.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
        && name.as_bytes().get(1) == Some(&b':');
    if name.is_empty()
        || matches!(name, "." | "..")
        || name.contains(['/', '\\'])
        || name.chars().any(char::is_control)
        || Path::new(name).is_absolute()
        || drive_prefix
    {
        return Err(MediaError::InvalidAttachment(
            "Archive entries must have a local file name".into(),
        ));
    }
    Ok(())
}

fn zip_error(error: zip::result::ZipError) -> MediaError {
    MediaError::InvalidAttachment(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read as _;

    #[test]
    fn archives_keep_duplicate_basenames_and_original_files_until_the_send_guard_drops() {
        let source = tempfile::tempdir().unwrap();
        for folder in ["first", "second"] {
            std::fs::create_dir(source.path().join(folder)).unwrap();
            std::fs::write(source.path().join(folder).join("file.txt"), folder).unwrap();
        }
        let paths = ["first", "second"].map(|folder| {
            source
                .path()
                .join(folder)
                .join("file.txt")
                .to_string_lossy()
                .into_owned()
        });
        let item = create(
            paths
                .iter()
                .map(|path| {
                    let mut item = PreparedAttachment::default();
                    item.path = path.clone();
                    item.name = "file.txt".into();
                    item
                })
                .collect(),
            None,
        )
        .unwrap();
        let path = PathBuf::from(&item.path);
        {
            let mut archive = zip::ZipArchive::new(File::open(&path).unwrap()).unwrap();
            assert_eq!(archive.len(), 2);
            for (entry, expected) in [("file.txt", "first"), ("file (2).txt", "second")] {
                let mut content = String::new();
                archive
                    .by_name(entry)
                    .unwrap()
                    .read_to_string(&mut content)
                    .unwrap();
                assert_eq!(content, expected);
            }
        }
        drop(item);
        assert!(!path.exists());
        assert_eq!(std::fs::read_to_string(&paths[0]).unwrap(), "first");
    }

    #[test]
    fn password_archives_require_the_correct_password_and_preserve_entry_names_and_originals() {
        for password in ["acceptance-test", "密码-🔐"] {
            let source = tempfile::tempdir().unwrap();
            let mut items = Vec::new();
            let expected = [
                ("small.txt", b"small".as_slice()),
                (
                    "original.txt",
                    b"A named original file whose content is longer than twenty bytes".as_slice(),
                ),
            ];
            for (index, (name, bytes)) in expected.iter().enumerate() {
                let path = source.path().join(format!("random-prepared-{index}"));
                std::fs::write(&path, bytes).unwrap();
                let mut item = PreparedAttachment::default();
                item.path = path.to_string_lossy().into_owned();
                item.name = (*name).into();
                items.push(item);
            }
            let item = create(items, Some(password)).unwrap();
            let path = PathBuf::from(&item.path);
            let mut zip = zip::ZipArchive::new(File::open(&path).unwrap()).unwrap();
            assert_eq!(zip.len(), expected.len());
            for (name, bytes) in expected {
                assert!(zip.by_name(name).is_err(), "passwordless reads must fail");
                let wrong_rejected = match zip.by_name_decrypt(name, b"incorrect-password") {
                    Err(_) => true,
                    // AES has a short password verifier; a rare collision must
                    // still be rejected by the content authentication tag.
                    Ok(mut entry) => entry.read_to_end(&mut Vec::new()).is_err(),
                };
                assert!(wrong_rejected, "wrong password must fail authentication");
                let mut entry = zip.by_name_decrypt(name, password.as_bytes()).unwrap();
                assert!(entry.encrypted());
                let mut actual = Vec::new();
                entry.read_to_end(&mut actual).unwrap();
                assert_eq!(actual, bytes);
            }
            drop(zip);
            drop(item);
            assert!(!path.exists());
            assert_eq!(std::fs::read_dir(source.path()).unwrap().count(), 2);
        }
    }

    #[test]
    fn empty_password_keeps_an_unencrypted_archive_and_owned_input_leases_are_respected() {
        let temporary = tempfile::NamedTempFile::new().unwrap().into_temp_path();
        std::fs::write(&temporary, b"prepared clipboard payload").unwrap();
        let original_path = temporary.to_path_buf();
        let mut source = PreparedAttachment::default();
        source.path = temporary.to_string_lossy().into_owned();
        source.name = "Pasted image.png".into();
        source.retain_temporary(temporary);
        let item = create(vec![source.clone()], Some("")).unwrap();
        let mut zip = zip::ZipArchive::new(File::open(&item.path).unwrap()).unwrap();
        let mut entry = zip.by_name("Pasted image.png").unwrap();
        assert!(!entry.encrypted());
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"prepared clipboard payload");
        drop(entry);
        drop(zip);
        drop(source);
        assert!(!original_path.exists());
        assert!(Path::new(&item.path).exists());
    }

    #[test]
    fn archive_entry_names_cannot_escape_the_selected_leaf_name() {
        let source = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(source.path(), b"original").unwrap();
        for name in [
            "",
            ".",
            "..",
            "../outside.txt",
            "/absolute.txt",
            "folder/file.txt",
            "folder\\file.txt",
            "bad\0name",
            "C:drive.txt",
        ] {
            let mut item = PreparedAttachment::default();
            item.path = source.path().to_string_lossy().into_owned();
            item.name = name.into();
            assert!(create(vec![item], None).is_err(), "{name:?}");
        }
        assert_eq!(std::fs::read(source.path()).unwrap(), b"original");
    }
}
