use std::fmt::{Debug, Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("server error: {0}")]
    Server(Error),
    #[error(transparent)]
    Request(#[from] reqwest::Error),
    #[error("failed to serialize json: {0:?}")]
    JsonSerializeError(#[from] serde_json::Error),
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Error {
    pub status: i64,
    pub code: i64,
    pub description: String,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Error: status: {}, code: {}, description: {}",
            self.status, self.code, self.description
        )
    }
}

impl std::error::Error for Error {}

pub mod error_code {
    pub const BAD_REQUEST: i64 = 400;
    pub const AUTHENTICATION: i64 = 401;
    pub const FORBIDDEN: i64 = 403;
    pub const NOT_FOUND: i64 = 404;
    pub const TOO_MANY_REQUEST: i64 = 429;
    pub const SERVER: i64 = 500;
    pub const TIME_INACCURATE: i64 = 911;

    pub const TRANSACTION: i64 = 10001;
    pub const BAD_DATA: i64 = 10002;
    pub const PHONE_SMS_DELIVERY: i64 = 10003;
    pub const RECAPTCHA_IS_INVALID: i64 = 10004;
    pub const NEED_CAPTCHA: i64 = 10005;
    pub const OLD_VERSION: i64 = 10006;
    pub const PHONE_INVALID_FORMAT: i64 = 20110;
    pub const INSUFFICIENT_IDENTITY_NUMBER: i64 = 20111;
    pub const INVALID_INVITATION_CODE: i64 = 20112;
    pub const PHONE_VERIFICATION_CODE_INVALID: i64 = 20113;
    pub const PHONE_VERIFICATION_CODE_EXPIRED: i64 = 20114;
    pub const INVALID_QR_CODE: i64 = 20115;
    pub const GROUP_CHAT_FULL: i64 = 20116;
    pub const INSUFFICIENT_BALANCE: i64 = 20117;
    pub const INVALID_PIN_FORMAT: i64 = 20118;
    pub const PIN_INCORRECT: i64 = 20119;
    pub const TOO_SMALL: i64 = 20120;
    pub const USED_PHONE: i64 = 20122;
    pub const INSUFFICIENT_TRANSACTION_FEE: i64 = 20124;
    pub const TOO_MANY_STICKERS: i64 = 20126;
    pub const WITHDRAWAL_AMOUNT_SMALL: i64 = 20127;
    pub const INVALID_CODE_TOO_FREQUENT: i64 = 20129;
    pub const INVALID_EMERGENCY_CONTACT: i64 = 20130;
    pub const WITHDRAWAL_MEMO_FORMAT_INCORRECT: i64 = 20131;
    pub const FAVORITE_LIMIT: i64 = 20132;
    pub const CIRCLE_LIMIT: i64 = 20133;
    pub const WITHDRAWAL_FEE_TOO_SMALL: i64 = 20135;
    pub const CONVERSATION_CHECKSUM_INVALID_ERROR: i64 = 20140;
    pub const BLOCKCHAIN_ERROR: i64 = 30100;
    pub const INVALID_ADDRESS: i64 = 30102;
    pub const INSUFFICIENT_POOL: i64 = 30103;
}
