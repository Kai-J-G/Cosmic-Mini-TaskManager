//! Localization loader and fluent macros.

use i18n_embed::fluent::{fluent_language_loader, FluentLanguageLoader};
use i18n_embed::{DefaultLocalizer, LanguageLoader, Localizer};
use rust_embed::RustEmbed;
use std::sync::{LazyLock, OnceLock};

#[derive(RustEmbed)]
#[folder = "i18n/"]
struct Localizations;

pub static LANGUAGE_LOADER: LazyLock<FluentLanguageLoader> = LazyLock::new(|| {
    let loader: FluentLanguageLoader = fluent_language_loader!();

    loader
        .load_fallback_language(&Localizations)
        .expect("Error while loading fallback language");

    loader
});

static LOCALIZATION_INITIALIZED: OnceLock<()> = OnceLock::new();

#[macro_export]
macro_rules! fl {
    ($message_id:literal) => {{
        $crate::localize::localize();
        i18n_embed_fl::fl!($crate::localize::LANGUAGE_LOADER, $message_id)
    }};
    ($message_id:literal, $($args:expr),*) => {{
        $crate::localize::localize();
        i18n_embed_fl::fl!($crate::localize::LANGUAGE_LOADER, $message_id, $($args), *)
    }};
}

/// Returns the `Localizer` for cosmic-mini-taskmanager.
pub fn localizer() -> Box<dyn Localizer> {
    Box::from(DefaultLocalizer::new(&*LANGUAGE_LOADER, &Localizations))
}

/// Selects desktop language if not already initialized.
pub fn localize() {
    LOCALIZATION_INITIALIZED.get_or_init(|| {
        let localizer = localizer();
        let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();
        if let Err(error) = localizer.select(&requested_languages) {
            eprintln!(
                "Error while loading language for cosmic-mini-taskmanager: {}",
                error
            );
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use i18n_embed::unic_langid::LanguageIdentifier;

    #[test]
    fn test_fallback_strings() {
        let title = fl!("app-title");
        assert_eq!(title, "Task Manager");

        let stop = fl!("btn-stop");
        assert_eq!(stop, "Stop");
    }

    #[test]
    fn test_message_with_arguments() {
        let msg = fl!("msg-stopped", pid = 1234);
        assert!(msg.contains("1234"));
        assert!(msg.contains("Stopped process PID"));
    }

    #[test]
    fn test_language_selection() {
        let localizer = localizer();
        let fr: LanguageIdentifier = "fr".parse().unwrap();
        let _ = localizer.select(&[fr]);
        let fr_title = i18n_embed_fl::fl!(&*LANGUAGE_LOADER, "app-title");
        assert_eq!(fr_title, "Gestionnaire des tâches");

        let de: LanguageIdentifier = "de".parse().unwrap();
        let _ = localizer.select(&[de]);
        let de_title = i18n_embed_fl::fl!(&*LANGUAGE_LOADER, "app-title");
        assert_eq!(de_title, "Taskmanager");

        // Restore fallback
        let en: LanguageIdentifier = "en".parse().unwrap();
        let _ = localizer.select(&[en]);
    }
}
