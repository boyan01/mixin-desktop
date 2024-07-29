use std::collections::HashSet;

use lazy_static::lazy_static;

pub const SIGNAL_KEY: &str = "SIGNAL_KEY";
pub const SIGNAL_TEXT: &str = "SIGNAL_TEXT";
pub const SIGNAL_IMAGE: &str = "SIGNAL_IMAGE";
pub const SIGNAL_VIDEO: &str = "SIGNAL_VIDEO";
pub const SIGNAL_STICKER: &str = "SIGNAL_STICKER";
pub const SIGNAL_DATA: &str = "SIGNAL_DATA";
pub const SIGNAL_CONTACT: &str = "SIGNAL_CONTACT";
pub const SIGNAL_AUDIO: &str = "SIGNAL_AUDIO";
pub const SIGNAL_LIVE: &str = "SIGNAL_LIVE";
pub const SIGNAL_POST: &str = "SIGNAL_POST";
pub const SIGNAL_LOCATION: &str = "SIGNAL_LOCATION";
pub const SIGNAL_TRANSCRIPT: &str = "SIGNAL_TRANSCRIPT";
pub const PLAIN_TEXT: &str = "PLAIN_TEXT";
pub const PLAIN_IMAGE: &str = "PLAIN_IMAGE";
pub const PLAIN_VIDEO: &str = "PLAIN_VIDEO";
pub const PLAIN_DATA: &str = "PLAIN_DATA";
pub const PLAIN_STICKER: &str = "PLAIN_STICKER";
pub const PLAIN_CONTACT: &str = "PLAIN_CONTACT";
pub const PLAIN_AUDIO: &str = "PLAIN_AUDIO";
pub const PLAIN_LIVE: &str = "PLAIN_LIVE";
pub const PLAIN_POST: &str = "PLAIN_POST";
pub const PLAIN_JSON: &str = "PLAIN_JSON";
pub const PLAIN_LOCATION: &str = "PLAIN_LOCATION";
pub const PLAIN_TRANSCRIPT: &str = "PLAIN_TRANSCRIPT";
pub const MESSAGE_RECALL: &str = "MESSAGE_RECALL";
pub const MESSAGE_PIN: &str = "MESSAGE_PIN";
pub const STRANGER: &str = "STRANGER";
pub const SECRET: &str = "SECRET";
pub const SYSTEM_CONVERSATION: &str = "SYSTEM_CONVERSATION";
pub const SYSTEM_USER: &str = "SYSTEM_USER";
pub const SYSTEM_CIRCLE: &str = "SYSTEM_CIRCLE";
pub const SYSTEM_SESSION: &str = "SYSTEM_SESSION";
pub const SYSTEM_ACCOUNT_SNAPSHOT: &str = "SYSTEM_ACCOUNT_SNAPSHOT";
pub const SYSTEM_SAFE_SNAPSHOT: &str = "SYSTEM_SAFE_SNAPSHOT";
pub const SYSTEM_SAFE_INSCRIPTION: &str = "SYSTEM_SAFE_INSCRIPTION";
pub const APP_BUTTON_GROUP: &str = "APP_BUTTON_GROUP";
pub const APP_CARD: &str = "APP_CARD";
pub const WEBRTC_AUDIO_OFFER: &str = "WEBRTC_AUDIO_OFFER";
pub const WEBRTC_AUDIO_ANSWER: &str = "WEBRTC_AUDIO_ANSWER";
pub const WEBRTC_ICE_CANDIDATE: &str = "WEBRTC_ICE_CANDIDATE";
pub const WEBRTC_AUDIO_CANCEL: &str = "WEBRTC_AUDIO_CANCEL";
pub const WEBRTC_AUDIO_DECLINE: &str = "WEBRTC_AUDIO_DECLINE";
pub const WEBRTC_AUDIO_END: &str = "WEBRTC_AUDIO_END";
pub const WEBRTC_AUDIO_BUSY: &str = "WEBRTC_AUDIO_BUSY";
pub const WEBRTC_AUDIO_FAILED: &str = "WEBRTC_AUDIO_FAILED";
pub const KRAKEN_INVITE: &str = "KRAKEN_INVITE";
pub const KRAKEN_PUBLISH: &str = "KRAKEN_PUBLISH";
pub const KRAKEN_SUBSCRIBE: &str = "KRAKEN_SUBSCRIBE";
pub const KRAKEN_ANSWER: &str = "KRAKEN_ANSWER";
pub const KRAKEN_TRICKLE: &str = "KRAKEN_TRICKLE";
pub const KRAKEN_END: &str = "KRAKEN_END";
pub const KRAKEN_CANCEL: &str = "KRAKEN_CANCEL";
pub const KRAKEN_DECLINE: &str = "KRAKEN_DECLINE";
pub const KRAKEN_LIST: &str = "KRAKEN_LIST";
pub const KRAKEN_RESTART: &str = "KRAKEN_RESTART";
pub const ENCRYPTED_TEXT: &str = "ENCRYPTED_TEXT";
pub const ENCRYPTED_IMAGE: &str = "ENCRYPTED_IMAGE";
pub const ENCRYPTED_VIDEO: &str = "ENCRYPTED_VIDEO";
pub const ENCRYPTED_STICKER: &str = "ENCRYPTED_STICKER";
pub const ENCRYPTED_DATA: &str = "ENCRYPTED_DATA";
pub const ENCRYPTED_CONTACT: &str = "ENCRYPTED_CONTACT";
pub const ENCRYPTED_AUDIO: &str = "ENCRYPTED_AUDIO";
pub const ENCRYPTED_LIVE: &str = "ENCRYPTED_LIVE";
pub const ENCRYPTED_POST: &str = "ENCRYPTED_POST";
pub const ENCRYPTED_LOCATION: &str = "ENCRYPTED_LOCATION";
pub const ENCRYPTED_TRANSCRIPT: &str = "ENCRYPTED_TRANSCRIPT";

pub trait MessageCategory {
    fn is_plain(&self) -> bool;
    fn is_system(&self) -> bool;
    fn is_pin(&self) -> bool;
    fn is_encrypted(&self) -> bool;
    fn is_signal(&self) -> bool;
    fn is_call(&self) -> bool;
    fn is_kraken(&self) -> bool;
    fn is_recall(&self) -> bool;
    fn is_fts_message(&self) -> bool;
    fn is_text(&self) -> bool;
    fn is_live(&self) -> bool;
    fn is_image(&self) -> bool;
    fn is_video(&self) -> bool;
    fn is_sticker(&self) -> bool;
    fn is_post(&self) -> bool;
    fn is_audio(&self) -> bool;
    fn is_data(&self) -> bool;
    fn is_location(&self) -> bool;
    fn is_contact(&self) -> bool;
    fn is_transcript(&self) -> bool;
    fn is_app_card(&self) -> bool;
    fn is_app_button_group(&self) -> bool;
    fn is_media(&self) -> bool;
    fn is_attachment(&self) -> bool;
    fn is_group_call(&self) -> bool;
    fn is_call_message(&self) -> bool;
    fn can_recall(&self) -> bool;
    fn is_illegal_message_category(&self) -> bool;
    fn can_reply(&self) -> bool;
}

impl MessageCategory for String {
    fn is_plain(&self) -> bool {
        self.starts_with("PLAIN_")
    }

    fn is_system(&self) -> bool {
        self.starts_with("SYSTEM_")
    }

    fn is_pin(&self) -> bool {
        self == MESSAGE_PIN
    }

    fn is_encrypted(&self) -> bool {
        self.starts_with("ENCRYPTED_")
    }

    fn is_signal(&self) -> bool {
        self.starts_with("SIGNAL_")
    }

    fn is_call(&self) -> bool {
        self.starts_with("WEBRTC") || self.starts_with("KRAKEN")
    }

    fn is_kraken(&self) -> bool {
        self.starts_with("KRAKEN")
    }

    fn is_recall(&self) -> bool {
        self == MESSAGE_RECALL
    }

    fn is_fts_message(&self) -> bool {
        self.ends_with("TEXT")
            || self.ends_with("DATA")
            || self.ends_with("POST")
            || self.ends_with("TRANSCRIPT")
    }

    fn is_text(&self) -> bool {
        self.ends_with("_TEXT")
    }

    fn is_live(&self) -> bool {
        self.ends_with("_LIVE")
    }

    fn is_image(&self) -> bool {
        self.ends_with("_IMAGE")
    }
    fn is_video(&self) -> bool {
        self.ends_with("_VIDEO")
    }

    fn is_sticker(&self) -> bool {
        self.ends_with("_STICKER")
    }

    fn is_post(&self) -> bool {
        self.ends_with("_POST")
    }

    fn is_audio(&self) -> bool {
        self.ends_with("_AUDIO")
    }

    fn is_data(&self) -> bool {
        self.ends_with("_DATA")
    }

    fn is_location(&self) -> bool {
        self.ends_with("_LOCATION")
    }

    fn is_contact(&self) -> bool {
        self.ends_with("_CONTACT")
    }

    fn is_transcript(&self) -> bool {
        self.ends_with("_TRANSCRIPT")
    }

    fn is_app_card(&self) -> bool {
        self == APP_CARD
    }

    fn is_app_button_group(&self) -> bool {
        self == APP_BUTTON_GROUP
    }

    fn is_media(&self) -> bool {
        self.is_data() || self.is_image() || self.is_video()
    }

    fn is_attachment(&self) -> bool {
        self.is_data() || self.is_image() || self.is_video() || self.is_audio()
    }

    fn is_group_call(&self) -> bool {
        lazy_static! {
            static ref group_call_categories: HashSet<&'static str> =
                HashSet::from([KRAKEN_END, KRAKEN_DECLINE, KRAKEN_CANCEL, KRAKEN_INVITE,]);
        }
        group_call_categories.contains(&self.as_str())
    }

    fn is_call_message(&self) -> bool {
        lazy_static! {
            static ref call_message_categories: HashSet<&'static str> = HashSet::from([
                WEBRTC_AUDIO_CANCEL,
                WEBRTC_AUDIO_DECLINE,
                WEBRTC_AUDIO_END,
                WEBRTC_AUDIO_BUSY,
                WEBRTC_AUDIO_FAILED,
            ]);
        }
        call_message_categories.contains(&self.as_str())
    }

    fn can_recall(&self) -> bool {
        lazy_static! {
            static ref recall_categories: HashSet<&'static str> = HashSet::from([
                ENCRYPTED_TEXT,
                ENCRYPTED_IMAGE,
                ENCRYPTED_VIDEO,
                ENCRYPTED_STICKER,
                ENCRYPTED_DATA,
                ENCRYPTED_CONTACT,
                ENCRYPTED_AUDIO,
                ENCRYPTED_LIVE,
                ENCRYPTED_POST,
                ENCRYPTED_LOCATION,
                ENCRYPTED_TRANSCRIPT,
                SIGNAL_TEXT,
                SIGNAL_IMAGE,
                SIGNAL_VIDEO,
                SIGNAL_STICKER,
                SIGNAL_DATA,
                SIGNAL_CONTACT,
                SIGNAL_AUDIO,
                SIGNAL_LIVE,
                SIGNAL_POST,
                SIGNAL_LOCATION,
                SIGNAL_TRANSCRIPT,
                PLAIN_TEXT,
                PLAIN_IMAGE,
                PLAIN_VIDEO,
                PLAIN_STICKER,
                PLAIN_DATA,
                PLAIN_CONTACT,
                PLAIN_AUDIO,
                PLAIN_LIVE,
                PLAIN_POST,
                PLAIN_LOCATION,
                PLAIN_TRANSCRIPT,
                APP_CARD,
            ]);
        }
        recall_categories.contains(&self.as_str())
    }

    fn is_illegal_message_category(&self) -> bool {
        lazy_static! {
            static ref illegal_categories: HashSet<&'static str> = HashSet::from([
                SIGNAL_KEY,
                SIGNAL_TEXT,
                SIGNAL_IMAGE,
                SIGNAL_VIDEO,
                SIGNAL_STICKER,
                SIGNAL_DATA,
                SIGNAL_CONTACT,
                SIGNAL_AUDIO,
                SIGNAL_LIVE,
                SIGNAL_POST,
                SIGNAL_LOCATION,
                SIGNAL_TRANSCRIPT,
                PLAIN_TEXT,
                PLAIN_IMAGE,
                PLAIN_VIDEO,
                PLAIN_DATA,
                PLAIN_STICKER,
                PLAIN_CONTACT,
                PLAIN_AUDIO,
                PLAIN_LIVE,
                PLAIN_POST,
                PLAIN_JSON,
                PLAIN_LOCATION,
                PLAIN_TRANSCRIPT,
                MESSAGE_RECALL,
                STRANGER,
                SECRET,
                SYSTEM_CONVERSATION,
                SYSTEM_USER,
                SYSTEM_CIRCLE,
                SYSTEM_SESSION,
                SYSTEM_ACCOUNT_SNAPSHOT,
                SYSTEM_SAFE_SNAPSHOT,
                SYSTEM_SAFE_INSCRIPTION,
                APP_BUTTON_GROUP,
                APP_CARD,
                ENCRYPTED_TEXT,
                ENCRYPTED_IMAGE,
                ENCRYPTED_VIDEO,
                ENCRYPTED_STICKER,
                ENCRYPTED_DATA,
                ENCRYPTED_CONTACT,
                ENCRYPTED_AUDIO,
                ENCRYPTED_LIVE,
                ENCRYPTED_POST,
                ENCRYPTED_LOCATION,
                ENCRYPTED_TRANSCRIPT,
                MESSAGE_PIN,
            ]);
        }
        !illegal_categories.contains(&self.as_str())
    }

    fn can_reply(&self) -> bool {
        self.is_text()
            || self.is_image()
            || self.is_video()
            || self.is_live()
            || self.is_data()
            || self.is_post()
            || self.is_location()
            || self.is_audio()
            || self.is_sticker()
            || self.is_contact()
            || self.is_transcript()
            || self == APP_CARD
            || self == APP_BUTTON_GROUP
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        assert!("MESSAGE_PIN".to_string().is_pin());
        _ = "ABC".to_string().can_recall();
        _ = "DEFG".to_string().can_recall();
    }
}
