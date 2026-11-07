//! Locale-aware dates using the reference app's date skeleton ordering.
use chrono::{Datelike as _, Locale, NaiveDate};

pub(super) fn full(date: NaiveDate, tag: &str) -> String {
    date.format_localized(date_pattern(tag, true, false), locale(tag))
        .to_string()
}

pub(super) fn short(date: NaiveDate, today: NaiveDate, tag: &str) -> String {
    if date.iso_week() == today.iso_week() {
        date.format_localized(weekday_pattern(tag), locale(tag))
            .to_string()
    } else {
        date.format_localized(
            date_pattern(tag, date.year() != today.year(), false),
            locale(tag),
        )
        .to_string()
    }
}

pub(super) fn day(date: NaiveDate, today: NaiveDate, tag: &str) -> String {
    date.format_localized(
        date_pattern(tag, date.year() != today.year(), true),
        locale(tag),
    )
    .to_string()
}

fn locale(tag: &str) -> Locale {
    match tag {
        "es" => Locale::es_ES,
        "id" => Locale::id_ID,
        "ja" => Locale::ja_JP,
        "ms" => Locale::ms_MY,
        "ru" => Locale::ru_RU,
        "zh-CN" => Locale::zh_CN,
        "zh-HK" => Locale::zh_HK,
        "zh-TW" => Locale::zh_TW,
        _ => Locale::en_US,
    }
}

fn weekday_pattern(tag: &str) -> &'static str {
    match tag {
        "zh-CN" => "周%a",
        "zh-HK" | "zh-TW" => "週%a",
        _ => "%a",
    }
}

fn date_pattern(tag: &str, year: bool, weekday: bool) -> &'static str {
    // Date order and punctuation match intl 0.20.3's MMMd/yMMMd and
    // MMMEd/yMMMEd skeletons. Chrono supplies month and weekday names.
    match (tag, year, weekday) {
        ("ja", false, false) | ("zh-CN" | "zh-HK" | "zh-TW", false, false) => "%-m月%-d日",
        ("ja", true, false) | ("zh-CN" | "zh-HK" | "zh-TW", true, false) => "%Y年%-m月%-d日",
        ("ja", false, true) => "%-m月%-d日(%a)",
        ("ja", true, true) => "%Y年%-m月%-d日(%a)",
        ("zh-CN", false, true) => "%-m月%-d日周%a",
        ("zh-CN", true, true) => "%Y年%-m月%-d日周%a",
        ("zh-HK" | "zh-TW", false, true) => "%-m月%-d日週%a",
        ("zh-HK" | "zh-TW", true, true) => "%Y年%-m月%-d日週%a",
        ("es" | "id" | "ms" | "ru", false, false) => "%-d %b",
        ("es" | "id" | "ms", true, false) => "%-d %b %Y",
        ("ru", true, false) => "%-d %b %Y\u{202f}г.",
        ("es" | "id" | "ms" | "ru", false, true) => "%a, %-d %b",
        ("es" | "id" | "ms", true, true) => "%a, %-d %b %Y",
        ("ru", true, true) => "%a, %-d %b %Y\u{202f}г.",
        (_, false, false) => "%b %-d",
        (_, true, false) => "%b %-d, %Y",
        (_, false, true) => "%a, %b %-d",
        (_, true, true) => "%a, %b %-d, %Y",
    }
}
