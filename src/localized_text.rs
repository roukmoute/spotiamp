//! Language-neutral application messages. Static statuses borrow their catalog
//! strings; dynamic statuses are formatted only when the state changes, never
//! while painting. Metadata and third-party diagnostic text remain untouched.
use crate::locale::Language;
use std::{borrow::Cow, fmt, ops::Deref, sync::Arc};

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct LocalizedText {
    french: Cow<'static, str>,
    english: Option<Cow<'static, str>>,
    additional: Option<Arc<[String; 4]>>,
}

impl LocalizedText {
    pub fn new(
        french: impl Into<Cow<'static, str>>,
        english: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            french: french.into(),
            english: Some(english.into()),
            additional: None,
        }
    }
    /// Dynamic application messages retain all translations once, at the
    /// state change. Static messages use the shared catalogue without an allocation.
    #[cfg(any(feature = "spotify", feature = "updates", test))]
    pub fn with_languages(
        french: impl Into<Cow<'static, str>>,
        english: impl Into<Cow<'static, str>>,
        german: String,
        spanish: String,
        italian: String,
        portuguese: String,
    ) -> Self {
        Self {
            french: french.into(),
            english: Some(english.into()),
            additional: Some(Arc::new([german, spanish, italian, portuguese])),
        }
    }

    pub fn text(&self, language: Language) -> &str {
        match language {
            Language::French => &self.french,
            Language::English => self.english.as_deref().unwrap_or(&self.french),
            Language::German | Language::Spanish | Language::Italian | Language::Portuguese => {
                if let Some(additional) = &self.additional {
                    return &additional[match language {
                        Language::German => 0,
                        Language::Spanish => 1,
                        Language::Italian => 2,
                        Language::Portuguese => 3,
                        _ => unreachable!(),
                    }];
                }
                // Only a borrowed application key can enter the catalogue.
                // Owned third-party diagnostics stay opaque in every language.
                match &self.english {
                    Some(Cow::Borrowed(english)) => language.text(&self.french, english),
                    english => english.as_deref().unwrap_or(&self.french),
                }
            }
        }
    }
}

impl From<&'static str> for LocalizedText {
    fn from(french: &'static str) -> Self {
        let english = match french {
            "Connectez Spotify pour commencer." => "Connect Spotify to get started.",
            "Connectez Spotify Premium pour commencer" => "Connect Spotify Premium to get started",
            "Impossible de demarrer le moteur audio" => "Could not start the audio engine",
            "Impossible de creer le thread audio" => "Could not create the audio thread",
            "Connexion perdue. Cliquez sur Connecter pour reprendre." => {
                "Connection lost. Click Connect to resume."
            }
            "Sortie audio indisponible. Verifiez le peripherique puis reconnectez-vous." => {
                "Audio output unavailable. Check the device, then reconnect."
            }
            "Effacement du trousseau en cours..." => "Clearing the credential store...",
            "Reconnexion a la session Spotify enregistree..." => {
                "Reconnecting to your saved Spotify session..."
            }
            "Connexion Spotify : autorisez l'acces dans votre navigateur si demande" => {
                "Connecting to Spotify: authorize access in your browser if prompted"
            }
            "Connexion annulee; session enregistree conservee" => {
                "Connection cancelled; saved session retained"
            }
            "Deconnecte; effacement du trousseau en cours..." => {
                "Disconnected; clearing the credential store..."
            }
            "Connectez Spotify avant d'ajouter un lien" => "Connect Spotify before adding a link",
            "Import en cours; Vider permet de l'annuler" => "Import in progress; Clear cancels it",
            "File pleine : 500 titres maximum" => "Queue full: 500 tracks maximum",
            "Chargement des titres Spotify..." => "Loading Spotify tracks...",
            "Connectez Spotify pour lire la file" => "Connect Spotify to play the queue",
            "File vide" => "Queue empty",
            "Impossible d'initialiser le volume audio" => "Could not initialize audio volume",
            "Connecte. Collez un lien de titre, album ou playlist." => {
                "Connected. Paste a track, album or playlist link."
            }
            "Deconnecte; identifiants effaces du trousseau" => {
                "Disconnected; credentials removed from the credential store"
            }
            "Chargement audio..." => "Loading audio...",
            "Fin de la file" => "End of queue",
            "Arrete" => "Stopped",
            "Pause" => "Paused",
            "Titre indisponible, acces refuse ou erreur audio. Essayez Suivant." => {
                "Track unavailable, access denied or audio error. Try Next."
            }
            "Selection invalide; aucun titre ajoute" => "Invalid selection; no tracks added",
            "Selection limitee a 500 titres" => "Selection limited to 500 tracks",
            "Ajout a la playlist Spotify en cours..." => "Adding to the Spotify playlist...",
            "Ajout en cours; attendez sa confirmation" => {
                "Addition in progress; wait for confirmation"
            }
            "Connectez Spotify pour modifier une playlist" => "Connect Spotify to edit a playlist",
            "Cette playlist est en lecture seule; aucun titre ajoute" => {
                "This playlist is read-only; no tracks added"
            }
            "Impossible de verifier la playlist; aucun titre ajoute" => {
                "Could not verify the playlist; no tracks added"
            }
            "La playlist a change; actualisez avant de reessayer" => {
                "The playlist changed; refresh before retrying"
            }
            "Spotify limite les ajouts; patientez avant de reessayer" => {
                "Spotify is limiting additions; wait before retrying"
            }
            "Ajout non confirme; verifiez la playlist avant de reessayer" => {
                "Addition unconfirmed; check the playlist before retrying"
            }
            "Spotify a refuse l'ajout; aucun titre ajoute" => {
                "Spotify refused the addition; no tracks added"
            }
            "La destination n'est plus disponible; actualisez les playlists" => {
                "The destination is no longer available; refresh your playlists"
            }
            "Position d'insertion invalide; la file a ete conservee" => {
                "Invalid insertion position; the queue was retained"
            }
            "Titre ajoute depuis la recherche" => "Track added from search",
            "Connectez Spotify pour rechercher des titres" => {
                "Connect Spotify to search for tracks"
            }
            "Recherche Spotify en cours..." => "Searching Spotify...",
            "Recherche incomplete; relancez la recherche pour reessayer" => {
                "Search incomplete; submit it again to retry"
            }
            "Ce resultat a change; selectionnez-le a nouveau" => {
                "This result changed; select it again"
            }
            "Connectez Spotify pour lire ce titre" => "Connect Spotify to play this track",
            "Aucun titre charge; essayez une recherche plus precise" => {
                "No tracks loaded; try a more specific search"
            }
            "Aucun titre trouve. Essayez un autre nom ou artiste." => {
                "No tracks found. Try another name or artist."
            }
            "Recherche limitee a 256 caracteres" => "Search is limited to 256 characters",
            "Spotify ne repond pas a la recherche; reessayez" => {
                "Spotify did not respond to the search; try again"
            }
            "Recherche Spotify indisponible; reessayez dans un instant" => {
                "Spotify search is unavailable; try again shortly"
            }
            "Reponse de recherche Spotify trop volumineuse" => {
                "Spotify search response is too large"
            }
            "Connectez Spotify pour afficher vos playlists" => {
                "Connect Spotify to view your playlists"
            }
            "Sortie audio fermee" => "Audio output closed",
            "La sortie audio ne repond plus; verifiez votre peripherique" => {
                "Audio output stopped responding; check your device"
            }
            "Lien trop long" => "Link too long",
            "URI attendue : spotify:track|album|playlist:identifiant" => {
                "Expected URI: spotify:track|album|playlist:identifier"
            }
            "Collez un lien Spotify de titre, album ou playlist" => {
                "Paste a Spotify track, album or playlist link"
            }
            "Lien attendu : https://open.spotify.com/track|album|playlist/identifiant" => {
                "Expected link: https://open.spotify.com/track|album|playlist/identifier"
            }
            "Ce lien Spotify n'est pas un titre, album ou playlist" => {
                "This Spotify link is not a track, album or playlist"
            }
            "Seuls les liens Spotify de titre, album et playlist sont pris en charge" => {
                "Only Spotify track, album and playlist links are supported"
            }
            "Identifiant Spotify invalide" => "Invalid Spotify identifier",
            "Spotify ne repond pas; reessayez" => "Spotify is not responding; try again",
            "Impossible de lire ce lien Spotify; verifiez son acces" => {
                "Could not read this Spotify link; check its access"
            }
            "Aucun titre musical accessible dans ce lien" => {
                "No accessible music tracks in this link"
            }
            "Application fermee" => "Application closed",
            "Connexion Spotify impossible. Verifiez le reseau et votre abonnement Premium." => {
                "Could not connect to Spotify. Check your network and Premium subscription."
            }
            "Connecte; sauvegarde non confirmee, reconnexion automatique non garantie" => {
                "Connected; saving is unconfirmed, automatic reconnection is not guaranteed"
            }
            "Le service du trousseau s'est arrete" => "The credential store service stopped",
            "Le trousseau ne repond pas" => "The credential store is not responding",
            "Reconnexion impossible; session conservee. Cliquez sur Connecter pour reessayer." => {
                "Could not reconnect; session retained. Click Connect to try again."
            }
            "Connexion annulee" => "Connection cancelled",
            "Import de liens en cours; attendez sa fin ou videz la file" => {
                "Importing links; wait for completion or clear the queue"
            }
            "Connectez Spotify pour charger cette playlist" => {
                "Connect Spotify to load this playlist"
            }
            "Chargement de vos playlists Spotify..." => "Loading your Spotify playlists...",
            "Cette playlist est indisponible; actualisez la bibliotheque" => {
                "This playlist is unavailable; refresh the library"
            }
            "Connectez Spotify pour lire cette playlist" => "Connect Spotify to play this playlist",
            "Identifiant de playlist Spotify invalide" => "Invalid Spotify playlist identifier",
            "Chargement de la playlist..." => "Loading playlist...",
            "Import de liens en cours; la file a ete conservee" => {
                "Importing links; the queue was retained"
            }
            "Connectez Spotify pour lire cette playlist; la file a ete conservee" => {
                "Connect Spotify to play this playlist; the queue was retained"
            }
            "Aucun titre musical accessible; la file a ete conservee" => {
                "No accessible music tracks; the queue was retained"
            }
            "Ce titre ne figure plus dans la playlist; selectionnez-le a nouveau" => {
                "This track is no longer in the playlist; select it again"
            }
            "File pleine : 500 titres maximum; la file a ete conservee" => {
                "Queue full: 500 tracks maximum; the queue was retained"
            }
            "Trousseau indisponible" => "Credential store unavailable",
            "Session enregistree illisible" => "The saved session could not be read",
            "Trousseau verrouille ou indisponible" => "Credential store locked or unavailable",
            "Trousseau indisponible : connexion pour cette session uniquement" => {
                "Credential store unavailable: signed in for this session only"
            }
            "Session non enregistrable" => "The session could not be saved",
            "Impossible d'effacer la session du trousseau; deverrouillez-le puis reessayez" => {
                "Could not remove the saved session; unlock the credential store and try again"
            }
            "L'autorisation precedente se termine; reessayez dans quelques instants" => {
                "The previous authorization is finishing; try again shortly"
            }
            "Connexion terminee. Vous pouvez fermer cet onglet et revenir dans Spotiamp." => {
                "Connected. You can close this tab and return to Spotiamp."
            }
            "Impossible de preparer la connexion Spotify" => {
                "Could not prepare the Spotify connection"
            }
            "Connexion annulee ou refusee. Fermez l'onglet puis reessayez; le port local 5588 doit etre libre." => {
                "Connection cancelled or denied. Close the tab and try again; local port 5588 must be available."
            }
            "Le service de connexion s'est arrete" => "The connection service stopped",
            "Connexion expiree apres 3 minutes; cliquez sur Connecter pour reessayer" => {
                "Connection timed out after 3 minutes; click Connect to try again"
            }
            "Version de mesure UI seule : moteur audio non compile." => {
                "UI measurement build: audio engine not compiled."
            }
            "MESURE UI SEULE - sans moteur Spotify" => "UI MEASUREMENT ONLY - no Spotify engine",
            "Version de mesure UI seule : moteur Spotify non compile." => {
                "UI measurement build: Spotify engine not compiled."
            }
            "Spotify ne repond pas au chargement des playlists" => {
                "Spotify is not responding while loading playlists"
            }
            "Impossible de recuperer vos playlists Spotify" => {
                "Could not retrieve your Spotify playlists"
            }
            "Le chargement des playlists Spotify a depasse une minute" => {
                "Loading Spotify playlists exceeded one minute"
            }
            "Reponse Spotify trop volumineuse pour la bibliotheque" => {
                "Spotify response too large for the library"
            }
            "Reponse de bibliotheque Spotify illisible" => {
                "The Spotify library response could not be read"
            }
            "Vos playlists ont change pendant le chargement; actualisez" => {
                "Your playlists changed while loading; refresh"
            }
            "Longueur de bibliotheque Spotify invalide" => "Invalid Spotify library length",
            "Bibliotheque Spotify limitee a 2000 entrees, dossiers compris" => {
                "Spotify library limited to 2000 entries, including folders"
            }
            "Spotify a renvoye une bibliotheque incomplete; actualisez" => {
                "Spotify returned an incomplete library; refresh"
            }
            "Spotify a renvoye une page de playlists desordonnee" => {
                "Spotify returned an out-of-order playlist page"
            }
            "Les noms des playlists Spotify ne correspondent pas aux entrees" => {
                "Spotify playlist names do not match their entries"
            }
            "Longueur de bibliotheque Spotify incoherente" => "Inconsistent Spotify library length",
            "Spotify a renvoye une page incomplete sans progression" => {
                "Spotify returned an incomplete page without progress"
            }
            "Spotify ne fournit pas la revision de vos playlists" => {
                "Spotify did not provide your playlists' revision"
            }
            "Dossier Spotify sans identifiant" => "Spotify folder without an identifier",
            "La bibliotheque Spotify contient trop de dossiers imbriques" => {
                "The Spotify library has too many nested folders"
            }
            "Bibliotheque Spotify limitee a 1000 playlists" => {
                "Spotify library limited to 1000 playlists"
            }
            "Dossier de preferences indisponible" => "Preferences directory unavailable",
            "Qualite appliquee mais non memorisee : ecriture impossible" => {
                "Quality applied but not saved: could not write preferences"
            }
            "Skin applique mais non memorise : dossier indisponible" => {
                "Skin applied but not saved: directory unavailable"
            }
            "Skin applique mais non memorise : ecriture impossible" => {
                "Skin applied but not saved: could not write preferences"
            }
            _ => {
                return Self {
                    french: Cow::Borrowed(french),
                    english: None,
                    additional: None,
                };
            }
        };
        Self::new(french, english)
    }
}

// Owned input is external diagnostic text. Never match or rewrite song titles,
// paths, server responses or user-provided strings against the app catalogue.
impl From<String> for LocalizedText {
    fn from(text: String) -> Self {
        Self {
            french: Cow::Owned(text),
            english: None,
            additional: None,
        }
    }
}
impl From<&LocalizedText> for LocalizedText {
    fn from(text: &LocalizedText) -> Self {
        text.clone()
    }
}
impl From<&String> for LocalizedText {
    fn from(text: &String) -> Self {
        text.clone().into()
    }
}
impl std::error::Error for LocalizedText {}
impl Deref for LocalizedText {
    type Target = str;
    fn deref(&self) -> &str {
        &self.french
    }
}
impl fmt::Display for LocalizedText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.french)
    }
}
impl PartialEq<&str> for LocalizedText {
    fn eq(&self, other: &&str) -> bool {
        self.french == *other
    }
}
impl PartialEq<String> for LocalizedText {
    fn eq(&self, other: &String) -> bool {
        self.french == *other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_messages_are_borrowed_and_available_in_every_language() {
        for french in [
            "Selection invalide; aucun titre ajoute",
            "Selection limitee a 500 titres",
            "Ajout a la playlist Spotify en cours...",
            "Ajout en cours; attendez sa confirmation",
            "Connectez Spotify pour modifier une playlist",
            "Cette playlist est en lecture seule; aucun titre ajoute",
            "Impossible de verifier la playlist; aucun titre ajoute",
            "La playlist a change; actualisez avant de reessayer",
            "Spotify limite les ajouts; patientez avant de reessayer",
            "Ajout non confirme; verifiez la playlist avant de reessayer",
            "Spotify a refuse l'ajout; aucun titre ajoute",
            "La destination n'est plus disponible; actualisez les playlists",
            "Position d'insertion invalide; la file a ete conservee",
            "Recherche limitee a 256 caracteres",
            "Spotify ne repond pas a la recherche; reessayez",
            "Recherche Spotify indisponible; reessayez dans un instant",
            "Reponse de recherche Spotify trop volumineuse",
            "Titre ajoute depuis la recherche",
            "Connectez Spotify pour rechercher des titres",
            "Recherche Spotify en cours...",
            "Recherche incomplete; relancez la recherche pour reessayer",
            "Ce resultat a change; selectionnez-le a nouveau",
            "Connectez Spotify pour lire ce titre",
            "Aucun titre charge; essayez une recherche plus precise",
            "Aucun titre trouve. Essayez un autre nom ou artiste.",
        ] {
            let text = LocalizedText::from(french);
            let Some(Cow::Borrowed(english)) = text.english else {
                panic!("Search message has no static English translation: {french}");
            };
            assert!(matches!(text.french, Cow::Borrowed(_)));
            assert!(text.additional.is_none());
            for language in Language::ALL[2..].iter().copied() {
                let expected = crate::translations::lookup(english, language)
                    .unwrap_or_else(|| panic!("Missing {language:?} search message: {english}"));
                assert_eq!(text.text(language), expected);
                assert!(std::ptr::eq(
                    text.text(language).as_ptr(),
                    expected.as_ptr()
                ));
            }
        }
    }

    #[test]
    fn static_messages_borrow_and_switch_without_recreating_state() {
        let text = LocalizedText::from("Connectez Spotify pour lire la file");
        assert!(matches!(text.french, Cow::Borrowed(_)));
        assert!(matches!(text.english, Some(Cow::Borrowed(_))));
        assert!(text.additional.is_none());
        assert_eq!(
            text.text(Language::French),
            "Connectez Spotify pour lire la file"
        );
        for (language, expected) in Language::ALL.into_iter().zip([
            "Connectez Spotify pour lire la file",
            "Connect Spotify to play the queue",
            "Spotify verbinden, um die Warteschlange abzuspielen",
            "Conecta Spotify para reproducir la cola",
            "Collega Spotify per riprodurre la coda",
            "Liga o Spotify para reproduzir a fila",
        ]) {
            assert_eq!(text.text(language), expected);
        }
    }
    #[test]
    fn owned_external_text_is_preserved_in_every_language_even_if_it_matches_an_app_key() {
        for original in [
            "Connectez Spotify pour lire la file",
            "Connect Spotify to play the queue",
            "opaque/error: filename.txt",
        ] {
            let text = LocalizedText::from(original.to_owned());
            assert!(text.additional.is_none());
            for language in Language::ALL {
                assert_eq!(text.text(language), original);
            }
        }
    }
    #[test]
    fn dynamic_translation_clones_share_storage_in_ui_snapshots() {
        let original = LocalizedText::with_languages(
            String::from("1 titre"),
            String::from("1 track"),
            String::from("1 Titel"),
            String::from("1 canción"),
            String::from("1 brano"),
            String::from("1 faixa"),
        );
        let copy = original.clone();
        let original_storage = original.additional.as_ref().unwrap();
        assert!(Arc::ptr_eq(
            original_storage,
            copy.additional.as_ref().unwrap()
        ));
        assert_eq!(Arc::strong_count(original_storage), 2);
        for language in Language::ALL[2..].iter().copied() {
            assert!(std::ptr::eq(
                original.text(language).as_ptr(),
                copy.text(language).as_ptr()
            ));
        }
    }
}
