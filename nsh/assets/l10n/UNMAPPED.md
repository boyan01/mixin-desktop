# NSH copy coverage audit

The initial 2026-11-06 audit recorded 299 NSH-only phrases without an original
Flutter ARB match. All 299 now have explicit translations in `supplemental.json`
for es, id, ja, ms, ru, zh-CN, zh-TW and zh-HK; the exact English key supplies
the English copy. The original ARB translations remain unchanged and take
priority.

The current source scan covers 652 fixed English/Chinese copy pairs across
`nsh/src`, representing 637 distinct English keys. It includes 647 adjacent
literal pairs in `Key` declarations, `tr` / `label` / `format` calls and paired
options, plus five singular-duration branches that reuse the plural branch’s
Chinese template. Zero pairs lack a resolved translation in any of the eight
non-English locales. Coverage includes avatar crop controls, sender loading
states, unknown/encrypted/system messages, inscription metadata and bounded
text-preview states, disappearing-message units, shared-media empty states,
attachment progress controls, and missing reference Indonesian and Malay entries.

The supplemental catalog contains 553 exact English keys. Its 34 named
parameter templates have the same placeholder names in every locale. A temporary
Rust harness imports the actual language, alias and catalog modules. It checked
617 static source pairs in all nine locales, rendered all 34 catalog templates,
checked that every argument appeared and no placeholders remained, and verified
that argument values containing braces are preserved verbatim. The five
singular-duration branches were audited explicitly because they do not repeat
the Chinese literal beside the English string.

Additional catalog entries cover native bot chrome and dialogs: Back, More,
Close, Refresh, Copy link, Open in browser, Webpage, OK, Cancel and the download
failure message. Their native presentation wiring is owned by the bot host.

The audit covers application-authored fixed copy and these explicit templates.
It does not claim that arbitrary runtime strings, server-provided content or
user names/messages are translated. Brand names, binary units and protocol
identifiers retain their original spelling. Unpaired English-only accessibility
labels are allowed by the NSH design guide. Date/time formatting is owned by the
presentation layer and requires a separate locale-aware formatter.

Catalog coverage is a source-level result. Native-speaker review, translated
text expansion, live language switching and visual layout remain unverified.
Unknown future copy has an explicit English fallback; it is not silently
substituted with simplified Chinese for Taiwan or Hong Kong.
