# Flutter localization resources

These nine ARB files are unmodified copies of `MixinNetwork/flutter-app` at
`7fa2d1ba5ab65f8a7b4418e685b4148ff81c09e9`, from `lib/l10n/intl_*.arb`.
They remain the primary reference translations. `supplemental.json` contains
explicit translations for NSH-only wording and missing reference locale entries.
The source checkout was `/Users/yangbin/workspace/mixin/flutter-app`.

| Reference resource | Messages | Messages with placeholders / ICU |
| --- | ---: | ---: |
| `intl_en.arb` | 500 | 54 |
| `intl_es.arb` | 427 | 51 |
| `intl_in.arb` | 230 | 26 |
| `intl_ja.arb` | 402 | 50 |
| `intl_ms.arb` | 235 | 28 |
| `intl_ru.arb` | 402 | 50 |
| `intl_zh-HK.arb` | 497 | 54 |
| `intl_zh-TW.arb` | 497 | 54 |
| `intl_zh.arb` | 498 | 54 |

The picker exposes all nine reference locales plus the system preference.
The Indonesian catalog retains the legacy filename `intl_in.arb`; the public
locale tag is `id`, and both persisted `in` and `id` are accepted. Simplified
Chinese accepts the existing `zh-CN` preference and `zh`/`zh-Hans` aliases.
System Chinese matching distinguishes Taiwan / Hant and Hong Kong / Macao.
Unsupported system locales resolve to English.

Static copy uses exact, unique English values or the explicitly reviewed
aliases in `nsh/src/i18n/aliases.rs`. Ambiguous English values require an alias.
No fuzzy or case-insensitive matching is used. A locale's original ARB entry
has priority. If that entry is absent, lookup uses the exact English key in
`supplemental.json`. If both are absent, presentation explicitly falls back to
the caller's English wording (or its existing simplified Chinese value for
`zh-CN`). English presentation retains the caller's wording even for aliases.
Traditional Chinese never falls back automatically to simplified Chinese.

The supplemental catalog provides 552 exact English keys, including 33 named
parameter templates. It covers the initial 299 NSH-only phrases, count/date
templates, native bot chrome, crop controls, message and system states,
inscription details and text previews, disappearing-message units, shared-media
empty states, attachment progress controls, and missing reference locale entries.
Only missing reference entries are supplemented; none of the nine original
ARB resources is overwritten.
Traditional Chinese supplemental values were initially converted with OpenCC
`s2twp` / `s2hk`, then checked against the reference regional terminology.
The conversion is a one-time authoring step, with no runtime dependency.
These translations still require native-speaker and visual review; catalog
coverage does not establish linguistic or layout acceptance.

Static lookup rejects braces, so unexpanded arguments and ICU messages cannot
appear in labels. `I18n::format` renders the explicit named templates and
matches only the argument names supplied by the caller. Values are appended
verbatim and are never parsed or expanded again. It does not interpret ICU.
Count labels use neutral forms in languages whose plurals require more than
substitution. The original `participantsCount` reference key remains a separate
count helper using its documented `{arg0}` argument. Other parameterized ARB
messages remain preserved until a caller supplies their complete arguments.

Language changes update the presentation signal immediately and persist via
`SettingsClient`. Failed writes restore the previous selection. The setting
is loaded at application startup; choosing System clears the durable override.
Static UI text needs a reactive text closure or a presentation-only remount
that depends on `I18n::locale_tag()`. Account services, navigation, draft state,
and session owners must survive that remount. Embedded bot contexts and native
bot labels consume the same locale selection.

The 2026-11-07 source audit covers 652 fixed English/Chinese copy pairs
(637 distinct English keys) in `nsh/src`. This includes 647 adjacent literal
pairs in `Key` declarations, translation/format calls and paired options, plus
five singular-duration branches that reuse their plural branch’s Chinese
template. Every pair resolves in all eight non-English locales through an ARB
entry or the supplemental catalog. A temporary Rust harness compiled the actual
catalog, aliases and language modules, checked all static pairs, and rendered
all 34 catalog templates in all nine locales. The initial 299-phrase gap is
closed. This audit does not classify runtime server/user content as translatable
copy. Brand names, file units, protocol identifiers and unpaired English
accessibility labels are deliberate exclusions. See `UNMAPPED.md` for the audit
boundary.
