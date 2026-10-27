use crate::model::{Command, LibrarySnapshot, Snapshot};
use std::sync::{Arc, Mutex};
pub struct Sender;
pub struct SendError(Command);
impl SendError {
    pub fn into_inner(self) -> Command {
        self.0
    }
}
impl std::fmt::Display for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Version de mesure UI seule : moteur audio non compile.")
    }
}
pub fn command_error(_: &SendError) -> crate::localized_text::LocalizedText {
    "Version de mesure UI seule : moteur audio non compile.".into()
}
impl Sender {
    pub fn try_send(&self, command: Command) -> Result<(), SendError> {
        if matches!(command, Command::Library(_) | Command::SetLanguage(_)) {
            Ok(())
        } else {
            Err(SendError(command))
        }
    }
}
pub struct Engine {
    pub tx: Sender,
    pub state: Arc<Mutex<Snapshot>>,
    pub library: Arc<Mutex<LibrarySnapshot>>,
}
impl Engine {
    pub fn shutdown(self) -> Result<(), crate::localized_text::LocalizedText> {
        Ok(())
    }

    pub fn sync_language(&self, _: crate::locale::Language) -> bool {
        true
    }

    pub fn spawn() -> Self {
        Self::spawn_with_language(crate::locale::Language::French)
    }
    pub fn spawn_with_language(_: crate::locale::Language) -> Self {
        Self {
            tx: Sender,
            state: Arc::new(Mutex::new(Snapshot {
                status: "MESURE UI SEULE - sans moteur Spotify".into(),
                ..Snapshot::default()
            })),
            library: Arc::new(Mutex::new(LibrarySnapshot {
                revision: 1,
                status: "Version de mesure UI seule : moteur Spotify non compile.".into(),
                ..LibrarySnapshot::default()
            })),
        }
    }
}
