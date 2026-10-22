//! Small, explicit UI language values. Detection runs at startup or on request,
//! never while drawing a frame and never through a global mutable locale.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Language {
    // Keep deterministic constructors and existing renderer fixtures in French.
    // Automatic detection falls back to English, independently of this default.
    #[default]
    French,
    English,
}

impl Language {
    pub const fn code(self) -> &'static str {
        match self {
            Self::French => "fr",
            Self::English => "en",
        }
    }

    pub const fn text<'a>(self, french: &'a str, english: &'a str) -> &'a str {
        match self {
            Self::French => french,
            Self::English => english,
        }
    }

    /// Accept platform locale IDs such as fr-CA and fr_FR.UTF-8, but never
    /// mistake an unrelated language whose name happens to start with "fr".
    pub fn from_locale(locale: &str) -> Option<Self> {
        let language = locale.trim().split(['-', '_', '.', '@']).next()?;
        if language.eq_ignore_ascii_case("fr") {
            Some(Self::French)
        } else if language.eq_ignore_ascii_case("en") {
            Some(Self::English)
        } else {
            None
        }
    }

    pub fn detect_system() -> Self {
        platform_language()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LanguagePreference {
    #[default]
    Auto,
    Specific(Language),
}

impl LanguagePreference {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "auto" => Some(Self::Auto),
            "fr" => Some(Self::Specific(Language::French)),
            "en" => Some(Self::Specific(Language::English)),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Specific(language) => language.code(),
        }
    }

    pub fn resolve(self) -> Language {
        match self {
            Self::Auto => Language::detect_system(),
            Self::Specific(language) => language,
        }
    }
}

#[cfg(target_os = "windows")]
fn platform_language() -> Language {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetUserDefaultUILanguage() -> u16;
    }
    // Use the user's display language, not the regional date/number format.
    language_from_windows_id(unsafe { GetUserDefaultUILanguage() })
}

#[cfg(any(target_os = "windows", test))]
fn language_from_windows_id(language_id: u16) -> Language {
    // PRIMARYLANGID discards the region (France, Canada, Belgium, etc.).
    match language_id & 0x03ff {
        0x0c => Language::French,
        _ => Language::English,
    }
}

#[cfg(target_os = "macos")]
fn platform_language() -> Language {
    use std::ffi::{CStr, c_char, c_void};

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFLocaleCopyPreferredLanguages() -> *const c_void;
        fn CFArrayGetCount(array: *const c_void) -> isize;
        fn CFArrayGetValueAtIndex(array: *const c_void, index: isize) -> *const c_void;
        fn CFStringGetCString(
            string: *const c_void,
            buffer: *mut c_char,
            capacity: isize,
            encoding: u32,
        ) -> u8;
        fn CFRelease(value: *const c_void);
    }

    // CoreFoundation returns an owned array of canonical language IDs. A
    // fixed buffer and bounded scan avoid a dependency or locale catalogue.
    unsafe {
        let preferred = CFLocaleCopyPreferredLanguages();
        if preferred.is_null() {
            return Language::English;
        }
        let mut result = Language::English;
        for index in 0..CFArrayGetCount(preferred).clamp(0, 32) {
            let language = CFArrayGetValueAtIndex(preferred, index);
            let mut buffer = [0u8; 96];
            if !language.is_null()
                && CFStringGetCString(
                    language,
                    buffer.as_mut_ptr().cast(),
                    buffer.len() as isize,
                    0x08000100, // kCFStringEncodingUTF8
                ) != 0
            {
                let name = CStr::from_ptr(buffer.as_ptr().cast());
                if let Some(supported) = name.to_str().ok().and_then(Language::from_locale) {
                    result = supported;
                    break;
                }
            }
        }
        CFRelease(preferred);
        result
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn platform_language() -> Language {
    let all = std::env::var("LC_ALL").ok();
    let messages = std::env::var("LC_MESSAGES").ok();
    let lang = std::env::var("LANG").ok();
    language_from_environment(all.as_deref(), messages.as_deref(), lang.as_deref())
}

#[cfg(any(not(any(target_os = "windows", target_os = "macos")), test))]
fn language_from_environment(
    all: Option<&str>,
    messages: Option<&str>,
    lang: Option<&str>,
) -> Language {
    // The first non-empty variable wins even when it names an unsupported
    // language. Falling through to LANG in that case would violate LC_ALL.
    [all, messages, lang]
        .into_iter()
        .flatten()
        .find(|value| !value.trim().is_empty())
        .and_then(Language::from_locale)
        .unwrap_or(Language::English)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_regions_encodings_and_case_resolve_without_prefix_matches() {
        for locale in ["fr", "fr-FR", "fr_CA.UTF-8", "fr_BE@euro", " FR-ch "] {
            assert_eq!(Language::from_locale(locale), Some(Language::French));
        }
        for locale in ["en", "en-GB", "en_US.UTF-8", "EN-au"] {
            assert_eq!(Language::from_locale(locale), Some(Language::English));
        }
        for locale in ["", "C", "C.UTF-8", "POSIX", "de_DE", "french", "france"] {
            assert_eq!(Language::from_locale(locale), None);
        }
    }

    #[test]
    fn linux_message_locale_precedence_and_fallback_are_deterministic() {
        assert_eq!(
            language_from_environment(Some("en_US"), Some("fr_FR"), Some("fr_CA")),
            Language::English
        );
        assert_eq!(
            language_from_environment(Some(""), Some("fr_CA.UTF-8"), Some("en_GB")),
            Language::French
        );
        assert_eq!(
            language_from_environment(None, None, Some("fr_FR")),
            Language::French
        );
        assert_eq!(
            language_from_environment(Some("de_DE"), Some("fr_FR"), Some("fr_CA")),
            Language::English
        );
        assert_eq!(
            language_from_environment(Some("C"), None, Some("fr_FR")),
            Language::English
        );
        assert_eq!(
            language_from_environment(None, Some(""), None),
            Language::English
        );
    }

    #[test]
    fn windows_display_language_regions_and_unsupported_values() {
        for id in [0x040c, 0x080c, 0x0c0c, 0x100c] {
            assert_eq!(language_from_windows_id(id), Language::French);
        }
        for id in [0, 0x0409, 0x0809, 0x0407, 0x1400] {
            assert_eq!(language_from_windows_id(id), Language::English);
        }
    }

    #[test]
    fn explicit_preferences_resolve_independently_of_the_host() {
        for language in [Language::French, Language::English] {
            let preference = LanguagePreference::Specific(language);
            assert_eq!(preference.resolve(), language);
            assert_eq!(
                LanguagePreference::parse(preference.as_str()),
                Some(preference)
            );
        }
        assert_eq!(
            LanguagePreference::parse(" auto\r\n"),
            Some(LanguagePreference::Auto)
        );
        for invalid in ["", "FR", "fr-FR", "automatic", "en\0", "fr en"] {
            assert_eq!(LanguagePreference::parse(invalid), None);
        }
    }
}
