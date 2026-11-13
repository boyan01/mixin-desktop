//! Files shipped by the official NSH packager. These asset URIs do not map
//! to the main bundle's resource root. The macOS notification adapter installs
//! its sound in the application's Library/Sounds before notification delivery.
use nsh_core::element::{AssetFile, AssetKind};

pub(crate) const FILES: &[AssetFile] = &[
    AssetFile {
        uri: "asset:/@mixin_desktop_ui/Emoji-LICENSE.txt",
        source: concat!(env!("CARGO_MANIFEST_DIR"), "/assets/emoji.LICENSE"),
        kind: AssetKind::File,
    },
    AssetFile {
        uri: "asset:/@mixin_desktop_ui/Media-Third-Party-Notices.md",
        source: concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../mixin_desktop_media/THIRD-PARTY-NOTICES.md"
        ),
        kind: AssetKind::File,
    },
    AssetFile {
        uri: "asset:/@mixin_desktop_ui/mixin.caf",
        source: concat!(env!("CARGO_MANIFEST_DIR"), "/assets/mixin.caf"),
        kind: AssetKind::File,
    },
];
