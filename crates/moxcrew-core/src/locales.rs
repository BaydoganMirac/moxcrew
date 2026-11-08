#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DesktopLocale {
    pub code: &'static str,
    pub native_label: &'static str,
    pub catalog: Option<&'static str>,
}

pub const DEFAULT_DESKTOP_LOCALE: &str = "tr";
pub const TURKISH_DESKTOP_LOCALE: DesktopLocale = DesktopLocale {
    code: "tr",
    native_label: "Türkçe",
    catalog: Some("tr"),
};
pub const ENGLISH_DESKTOP_LOCALE: DesktopLocale = DesktopLocale {
    code: "en",
    native_label: "English",
    catalog: None,
};

/// The single authority for locale codes accepted by the native product.
/// Adding a locale also requires a reviewed desktop catalog and layout evidence.
pub const DESKTOP_LOCALES: &[DesktopLocale] = &[TURKISH_DESKTOP_LOCALE, ENGLISH_DESKTOP_LOCALE];

pub fn desktop_locale(code: &str) -> Option<DesktopLocale> {
    DESKTOP_LOCALES
        .iter()
        .copied()
        .find(|locale| locale.code == code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn desktop_locale_registry_is_unique_and_has_one_source_fallback() {
        let mut codes = HashSet::new();
        assert!(DESKTOP_LOCALES.iter().all(|locale| {
            !locale.code.is_empty()
                && !locale.native_label.trim().is_empty()
                && codes.insert(locale.code)
        }));
        assert_eq!(
            DESKTOP_LOCALES
                .iter()
                .filter(|locale| locale.catalog.is_none())
                .count(),
            1
        );
        assert!(desktop_locale(DEFAULT_DESKTOP_LOCALE).is_some());
    }
}
