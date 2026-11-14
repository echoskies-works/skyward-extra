//! Which language the panel speaks.

/// The two the menu is written in.
///
/// Two, not the UI's three: the panel has no font for traditional characters
/// beyond what GB2312 happens to share, and a menu that switches languages
/// on the same screen as fastboot has enough to do without a third column.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Lang {
    En,
    /// Chinese, in the simplified characters GB2312 covers.
    #[default]
    Zh,
}

impl Lang {
    /// One thing written in both languages, in the order [`Lang`] declares.
    pub fn pick<T>(self, [en, zh]: [T; 2]) -> T {
        match self {
            Lang::En => en,
            Lang::Zh => zh,
        }
    }

    /// The language to print without being told.
    ///
    /// `AETHER_LANG` first, because that is the value Aether's own settings
    /// can hand down and it survives a shell that resets the environment;
    /// then `LC_ALL` and `LANG` the way a shell sets them; then Chinese.
    ///
    /// Chinese as the fallback is a choice, not an oversight: this panel is
    /// for a phone used in China, and the English is there for the reader who
    /// cannot use the Chinese, not the other way round. A value the phone
    /// does carry — `C`, `POSIX`, an empty string — says nothing about which
    /// language to print, so it is skipped rather than obeyed.
    pub fn from_env() -> Lang {
        for name in ["AETHER_LANG", "LC_ALL", "LANG"] {
            let Ok(value) = std::env::var(name) else { continue };
            let value = value.to_ascii_lowercase();
            if value.starts_with("zh") {
                return Lang::Zh;
            }
            if value.starts_with("en") {
                return Lang::En;
            }
        }
        Lang::Zh
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picking_reads_the_language_it_was_asked_for() {
        assert_eq!(Lang::En.pick(["no", "否"]), "no");
        assert_eq!(Lang::Zh.pick(["no", "否"]), "否");
    }

    #[test]
    fn the_default_is_chinese() {
        // Whatever this machine's shell says, the fallback the panel uses when
        // nothing tells it anything has to be the language of the people the
        // phone is for.
        assert_eq!(Lang::default(), Lang::Zh);
    }
}
