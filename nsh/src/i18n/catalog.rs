use std::{collections::BTreeMap, sync::LazyLock};

use super::language::Language;

type Messages = BTreeMap<String, String>;

// Preserve the reference resources. NSH-only copy lives in a separate catalog.
static CATALOGS: LazyLock<BTreeMap<&'static str, Messages>> = LazyLock::new(|| {
    [
        ("en", include_str!("../../assets/l10n/intl_en.arb")),
        ("es", include_str!("../../assets/l10n/intl_es.arb")),
        ("id", include_str!("../../assets/l10n/intl_in.arb")),
        ("ja", include_str!("../../assets/l10n/intl_ja.arb")),
        ("ms", include_str!("../../assets/l10n/intl_ms.arb")),
        ("ru", include_str!("../../assets/l10n/intl_ru.arb")),
        ("zh-CN", include_str!("../../assets/l10n/intl_zh.arb")),
        ("zh-TW", include_str!("../../assets/l10n/intl_zh-TW.arb")),
        ("zh-HK", include_str!("../../assets/l10n/intl_zh-HK.arb")),
    ]
    .into_iter()
    .map(|(tag, source)| {
        let values: BTreeMap<String, serde_json::Value> =
            serde_json::from_str(source).expect("bundled Flutter ARB is valid JSON");
        let messages = values
            .into_iter()
            .filter_map(|(key, value)| {
                if key.starts_with('@') {
                    return None;
                }
                value.as_str().map(|value| (key, value.to_owned()))
            })
            .collect();
        (tag, messages)
    })
    .collect()
});

static SUPPLEMENTAL: LazyLock<BTreeMap<String, Messages>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../assets/l10n/supplemental.json"))
        .expect("bundled supplemental translations are valid JSON")
});

fn plain(value: &str) -> bool {
    !value.contains(['{', '}'])
}

fn lookup(language: Language, english: &'static str) -> Option<&'static str> {
    if language == Language::English {
        return Some(english);
    }
    // Aliases are reviewed semantic matches, never fuzzy English matching.
    let key = super::aliases::key(english).or_else(|| {
        let mut matches = CATALOGS["en"]
            .iter()
            .filter(|(_, value)| value.as_str() == english && plain(value));
        let (key, _) = matches.next()?;
        matches.next().is_none().then_some(key.as_str())
    });
    key.and_then(|key| CATALOGS[language.value()].get(key))
        .map(String::as_str)
        .or_else(|| {
            SUPPLEMENTAL
                .get(english)
                .and_then(|messages| messages.get(language.value()))
                .map(String::as_str)
        })
}

pub(super) fn translation(language: Language, english: &'static str) -> Option<&'static str> {
    lookup(language, english).filter(|value| plain(value))
}

pub(super) fn format(
    language: Language,
    english: &'static str,
    chinese: &'static str,
    arguments: &[(&str, &str)],
) -> String {
    let template = lookup(language, english).unwrap_or(if language == Language::Chinese {
        chinese
    } else {
        english
    });
    let mut output = String::with_capacity(template.len());
    let mut copied = 0;
    // Match only the caller's explicit names in the original template. Values
    // are appended verbatim, so a name containing braces is never expanded.
    for (start, _) in template.match_indices('{') {
        if start < copied {
            continue;
        }
        if let Some((name, value)) = arguments.iter().find(|(name, _)| {
            template[start + 1..].starts_with(name)
                && template.as_bytes().get(start + name.len() + 1) == Some(&b'}')
        }) {
            output.push_str(&template[copied..start]);
            output.push_str(value);
            copied = start + name.len() + 2;
        }
    }
    output.push_str(&template[copied..]);
    output
}

pub(super) fn members(language: Language, count: i64) -> String {
    // This reference key uses only arg0, not ICU plural syntax.
    if let Some(template) = CATALOGS[language.value()].get("participantsCount") {
        template.replace("{arg0}", &count.to_string())
    } else {
        format(
            language,
            "{count} members",
            "{count} 位成员",
            &[("count", &count.to_string())],
        )
    }
}
