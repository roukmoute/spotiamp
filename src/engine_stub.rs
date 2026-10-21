use crate::model::{Command, LibrarySnapshot, Snapshot};
use std::sync::{Arc, Mutex};
pub struct Sender;
pub struct SendError;
impl std::fmt::Display for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Version de mesure UI seule : moteur audio non compile.")
    }
}
impl Sender {
    pub fn try_send(&self, command: Command) -> Result<(), SendError> {
        if matches!(command, Command::Library(_)) {
            Ok(())
        } else {
            Err(SendError)
        }
    }
}
pub struct Engine {
    pub tx: Sender,
    pub state: Arc<Mutex<Snapshot>>,
    pub library: Arc<Mutex<LibrarySnapshot>>,
}
impl Engine {
    pub fn spawn() -> Self {
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
