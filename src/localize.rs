//! Localization, backed by Fluent catalogs embedded at build time.

use i18n_embed::fluent::{FluentLanguageLoader, fluent_language_loader};
use i18n_embed::{DefaultLocalizer, LanguageLoader, Localizer};
use rust_embed::RustEmbed;
use std::sync::LazyLock;

#[derive(RustEmbed)]
#[folder = "i18n/"]
struct Localizations;

pub static LANGUAGE_LOADER: LazyLock<FluentLanguageLoader> = LazyLock::new(|| {
    let loader: FluentLanguageLoader = fluent_language_loader!();
    loader
        .load_fallback_language(&Localizations)
        .expect("i18n/en catalog is embedded at build time");
    loader
});

/// Looks up a message by id, with optional Fluent arguments.
#[macro_export]
macro_rules! fl {
    ($message_id:literal) => {{
        i18n_embed_fl::fl!($crate::localize::LANGUAGE_LOADER, $message_id)
    }};
    ($message_id:literal, $($args:expr),*) => {{
        i18n_embed_fl::fl!($crate::localize::LANGUAGE_LOADER, $message_id, $($args), *)
    }};
}

pub fn localizer() -> Box<dyn Localizer> {
    Box::from(DefaultLocalizer::new(&*LANGUAGE_LOADER, &Localizations))
}

/// Loads the catalog matching the desktop's language. Call once at startup:
/// `fl!` is used inside the render loop and must not do this per lookup.
pub fn init() {
    let requested = i18n_embed::DesktopLanguageRequester::requested_languages();
    if let Err(error) = localizer().select(&requested) {
        eprintln!("cosmic-mini-taskmanager: could not load language: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use i18n_embed::unic_langid::LanguageIdentifier;

    /// Message ids defined in a language's catalog, read straight from the
    /// embedded `.ftl` so this checks the shipped files rather than the loader.
    fn message_ids(lang: &str) -> Vec<String> {
        let path = format!("{lang}/cosmic_mini_taskmanager.ftl");
        let file = Localizations::get(&path).expect("catalog is embedded");
        let text = std::str::from_utf8(&file.data).expect("catalog is UTF-8");

        text.lines()
            .filter(|line| !line.starts_with([' ', '#', '.', '*', '[']))
            .filter_map(|line| line.split_once(" ="))
            .map(|(id, _)| id.trim().to_string())
            .collect()
    }

    /// One test, because selecting a language mutates the process-wide loader:
    /// split across `#[test]` functions these would race each other.
    #[test]
    fn catalogs_resolve_in_every_shipped_language() {
        let localizer = localizer();

        // Fallback (en) is loaded eagerly.
        assert_eq!(fl!("app-title"), "Task Manager");
        assert_eq!(fl!("btn-stop"), "Stop");
        assert!(fl!("msg-stopped", pid = 1234).contains("1234"));

        for (tag, title) in [("fr", "Gestionnaire des tâches"), ("de", "Taskmanager")] {
            let lang: LanguageIdentifier = tag.parse().unwrap();
            localizer.select(&[lang]).unwrap();
            assert_eq!(fl!("app-title"), title, "wrong title for {tag}");
        }

        // Every catalog must define every message the fallback does, or the
        // UI falls back to English mid-sentence.
        let expected = message_ids("en");
        assert!(!expected.is_empty());

        for path in Localizations::iter() {
            let Some(lang) = path.split('/').next() else {
                continue;
            };
            let missing: Vec<_> = expected
                .iter()
                .filter(|id| !message_ids(lang).contains(*id))
                .collect();
            assert!(missing.is_empty(), "{lang} is missing {missing:?}");
        }

        let en: LanguageIdentifier = "en".parse().unwrap();
        localizer.select(&[en]).unwrap();
    }
}
