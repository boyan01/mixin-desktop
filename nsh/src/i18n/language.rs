/// The catalog names follow Flutter's ARB files; public tags use BCP 47.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Language {
    #[default]
    System,
    English,
    Spanish,
    Indonesian,
    Japanese,
    Malay,
    Russian,
    Chinese,
    ChineseTaiwan,
    ChineseHongKong,
}

impl Language {
    pub(super) const SUPPORTED: [Self; 9] = [
        Self::English,
        Self::Spanish,
        Self::Indonesian,
        Self::Japanese,
        Self::Malay,
        Self::Russian,
        Self::Chinese,
        Self::ChineseTaiwan,
        Self::ChineseHongKong,
    ];

    pub(super) fn parse(value: &str) -> Self {
        if value == "system" {
            return Self::System;
        }
        Self::resolve(value).unwrap_or_default()
    }

    pub(super) fn system(value: &str) -> Self {
        Self::resolve(value).unwrap_or(Self::English)
    }

    fn resolve(value: &str) -> Option<Self> {
        let normalized = value.replace('_', "-").to_ascii_lowercase();
        let mut parts = normalized.split('-');
        match parts.next()? {
            "en" => Some(Self::English),
            "es" => Some(Self::Spanish),
            "id" | "in" => Some(Self::Indonesian),
            "ja" => Some(Self::Japanese),
            "ms" => Some(Self::Malay),
            "ru" => Some(Self::Russian),
            "zh" => {
                let parts: Vec<_> = parts.collect();
                Some(if parts.contains(&"hk") || parts.contains(&"mo") {
                    Self::ChineseHongKong
                } else if parts.contains(&"tw") || parts.contains(&"hant") {
                    Self::ChineseTaiwan
                } else {
                    Self::Chinese
                })
            }
            _ => None,
        }
    }

    pub(super) fn value(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::English => "en",
            Self::Spanish => "es",
            Self::Indonesian => "id",
            Self::Japanese => "ja",
            Self::Malay => "ms",
            Self::Russian => "ru",
            Self::Chinese => "zh-CN",
            Self::ChineseTaiwan => "zh-TW",
            Self::ChineseHongKong => "zh-HK",
        }
    }

    pub(super) fn title(self) -> &'static str {
        match self {
            Self::System => "System language",
            Self::English => "English",
            Self::Spanish => "Español",
            Self::Indonesian => "Bahasa Indonesia",
            Self::Japanese => "日本語",
            Self::Malay => "Bahasa Melayu",
            Self::Russian => "Русский",
            Self::Chinese => "简体中文",
            Self::ChineseTaiwan => "繁體中文（台灣）",
            Self::ChineseHongKong => "繁體中文（香港）",
        }
    }
}
