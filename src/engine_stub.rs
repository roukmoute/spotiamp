use crate::model::{Command, Snapshot};
use std::sync::{Arc, Mutex};
pub struct Sender;
pub struct SendError;
impl std::fmt::Display for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Version de mesure UI seule : moteur audio non compile.")
    }
}
impl Sender {
    pub fn try_send(&self, _command: Command) -> Result<(), SendError> {
        Err(SendError)
    }
}
pub struct Engine {
    pub tx: Sender,
    pub state: Arc<Mutex<Snapshot>>,
}
impl Engine {
    pub fn spawn() -> Self {
        Self {
            tx: Sender,
            state: Arc::new(Mutex::new(Snapshot {
                status: "MESURE UI SEULE - sans moteur Spotify".into(),
                ..Snapshot::default()
            })),
        }
    }
}
