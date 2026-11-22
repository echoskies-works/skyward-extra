//! Which language the panel speaks.

/// The two the menu is written in.
///
/// Two, not the UI's fifteen: the panel has no font for traditional characters
/// beyond what GB2312 happens to share, and a menu that switches languages
/// on the same screen as fastboot has enough to do without fifteen columns.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Lang {
    /// What every phone that has not asked for Chinese reads.
    #[default]
    En,
    /// Chinese, in the simplified characters GB2312 covers.
    Zh,
}

/// Where the pages keep the language they were left speaking.
///
/// The same file, and the same name, as the system's own settings: the panel
/// is up before the system and outlives it, so this file is the one thing the
/// two share. It is read as text here, rather than through the settings crate,
/// because this program's dependency list is a decision about how likely it is
/// to still run on a phone whose system is the thing that broke.
const SETTINGS: &str = "/metadata/skyward/settings.json";

impl Lang {
    /// One thing written in both languages, in the order [`Lang`] declares.
    pub fn pick<T>(self, [en, zh]: [T; 2]) -> T {
        match self {
            Lang::En => en,
            Lang::Zh => zh,
        }
    }

    /// The language a locale names.
    ///
    /// Simplified Chinese is the only language this panel writes. Every other
    /// one — and the traditional characters, which the font would draw as a
    /// row of boxes — reads the English, so the panel is never unreadable.
    ///
    /// The rule is the pages' own: `zh` with no script and a place that writes
    /// the traditional characters says traditional, `zh` with no script and no
    /// place says simplified. Spelled out again rather than shared, because
    /// the UI crate is what this program must not depend on.
    pub fn for_locale(tag: &str) -> Lang {
        let tag = tag.replace('_', "-").to_ascii_lowercase();
        let mut parts = tag.split('-');
        if parts.next() != Some("zh") {
            return Lang::En;
        }
        for part in parts {
            match part {
                // The traditional characters, or a place that writes them.
                "hant" | "tw" | "hk" | "mo" => return Lang::En,
                "hans" | "cn" | "sg" | "my" => return Lang::Zh,
                _ => {}
            }
        }
        // Chinese that says nothing about the characters: simplified.
        Lang::Zh
    }
}

/// The language the panel speaks without being told.
///
/// `AETHER_LANG` first, because a shell can say so outright — nothing on the
/// phone sets it, and it is what lets the panel be looked at in either
/// language from a terminal without a rebuild. Otherwise the settings file
/// answers: the language chosen on the language page, and failing that the one
/// the pages last spoke.
///
/// Nothing to read, or nothing in it about a language, is a phone that has not
/// asked for Chinese, and English is what it reads. A first start, a system
/// that never came up far enough to mount `/metadata`, a settings file wiped
/// by a factory reset — all of them land here, and all of them are English.
pub fn chosen() -> Lang {
    if let Ok(tag) = std::env::var("AETHER_LANG")
        && !tag.is_empty()
    {
        return Lang::for_locale(&tag);
    }
    of_settings(&std::fs::read_to_string(SETTINGS).unwrap_or_default())
}

/// The language the settings file's text names.
///
/// The chosen language wins over the spoken one, the way the pages give the
/// choice its way when they come back up.
fn of_settings(text: &str) -> Lang {
    let tag = field(text, "language").or_else(|| field(text, "spoken")).unwrap_or_default();
    Lang::for_locale(tag)
}

/// The string `key` holds in the settings file.
///
/// The file is JSON, and this reads the one shape it is written in: a quoted
/// key, a colon, a quoted value, one such pair per line. That is enough for
/// the two fields the panel wants, and a JSON parser would be a dependency
/// this program will not take. A key the file does not hold, or one holding
/// something that is not a string, reads as nothing.
fn field<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    for line in text.lines() {
        let Some(rest) = line.trim_start().strip_prefix('"') else { continue };
        let Some((name, rest)) = rest.split_once('"') else { continue };
        if name != key {
            continue;
        }
        let Some(rest) = rest.trim_start().strip_prefix(':') else { continue };
        let Some(rest) = rest.trim_start().strip_prefix('"') else { continue };
        return rest.split('"').next();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picking_reads_the_language_it_was_asked_for() {
        assert_eq!(Lang::En.pick(["no", "否"]), "no");
        assert_eq!(Lang::Zh.pick(["no", "否"]), "否");
    }

    /// Chinese is the one language the panel writes, and only in the
    /// characters its font can draw. Everything else reads the English.
    #[test]
    fn only_simplified_chinese_is_chinese() {
        for tag in ["zh", "zh-Hans", "zh-CN", "zh_CN", "zh-Hans-CN", "zh-SG", "ZH-cn"] {
            assert_eq!(Lang::for_locale(tag), Lang::Zh, "{tag}");
        }
        // The traditional characters would come out as boxes, and a menu of
        // boxes is worse than a menu in the other language.
        for tag in ["zh-Hant", "zh-TW", "zh-HK", "zh-MO", "zh-Hant-TW", "zh-hant"] {
            assert_eq!(Lang::for_locale(tag), Lang::En, "{tag}");
        }
        for tag in ["en", "en-US", "fr-FR", "ja-JP", "pt-BR", "in-ID", ""] {
            assert_eq!(Lang::for_locale(tag), Lang::En, "{tag}");
        }
    }

    /// Nothing said is English: a first start, a system that never mounted
    /// `/metadata`, a settings file a reset emptied.
    #[test]
    fn nothing_said_is_english() {
        assert_eq!(Lang::default(), Lang::En);
        assert_eq!(of_settings(""), Lang::En);
        assert_eq!(of_settings("{\n  \"dark\": false\n}\n"), Lang::En);
    }

    #[test]
    fn the_language_is_read_out_of_the_settings_file() {
        let decided = "{\n  \"language\": \"zh-Hans\",\n  \"spoken\": \"zh-Hans\",\n  \"dark\": false\n}\n";
        assert_eq!(of_settings(decided), Lang::Zh);
        // A language chosen on the page wins over the one last spoken.
        let both = "{\n  \"language\": \"en-US\",\n  \"spoken\": \"zh-Hans\"\n}\n";
        assert_eq!(of_settings(both), Lang::En);
        // Nothing chosen: the pages spoke what the system named.
        let never_chosen = "{\n  \"spoken\": \"zh-Hans\"\n}\n";
        assert_eq!(of_settings(never_chosen), Lang::Zh);
        let french = "{\n  \"spoken\": \"fr-FR\"\n}\n";
        assert_eq!(of_settings(french), Lang::En);
    }

    /// The reader must not be fooled by a value that looks like a key, or by
    /// a key that is only part of another.
    #[test]
    fn only_a_whole_key_reads() {
        let text = "{\n  \"spoken_language\": \"zh-Hans\",\n  \"note\": \"\\\"language\\\": \\\"zh-Hans\\\"\"\n}\n";
        assert_eq!(field(text, "language"), None);
        assert_eq!(of_settings(text), Lang::En);
        // A key holding something other than a string is nothing.
        assert_eq!(field("{\n  \"language\": null\n}\n", "language"), None);
    }
}
