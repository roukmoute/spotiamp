//! Credentials live only in the OS credential store, never in a cache file.
use std::{
    io::Write,
    net::{SocketAddr, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};

use crate::{locale::Language, localized_text::LocalizedText};
use keyring::{Entry, Error as KeyringError};
use librespot_core::authentication::Credentials;
use librespot_oauth::OAuthClientBuilder;

const SERVICE: &str = "app.spotiamp.spotify";
const ACCOUNT: &str = "spotify-session";
const CALLBACK: &str = "http://127.0.0.1:5588/login";
// Serialize OS keyring writes/deletion, including while a call prompts or blocks.
static STORE_LOCK: Mutex<()> = Mutex::new(());
static LOGIN_RUNNING: AtomicBool = AtomicBool::new(false);
static LOGIN_WAKE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Default)]
pub struct LoginAttempt {
    cancelled: AtomicBool,
    started: AtomicBool,
    finished: AtomicBool,
}

impl LoginAttempt {
    pub fn cancel(self: &Arc<Self>) {
        self.cancelled.store(true, Ordering::Release);
        let attempt = self.clone();
        // Upstream opens the browser BEFORE binding its listener. Retrying the
        // wakeup briefly covers that race. The single-login slot ensures these
        // wakeups cannot be delivered to a newer login attempt.
        std::thread::spawn(move || {
            for _ in 0..50 {
                let Ok(wakeup_guard) = LOGIN_WAKE_LOCK.lock() else {
                    break;
                };
                if attempt.finished.load(Ordering::Acquire) {
                    break;
                }
                if attempt.started.load(Ordering::Acquire) && wake_listener() {
                    break;
                }
                drop(wakeup_guard);
                std::thread::sleep(Duration::from_millis(20));
            }
        });
    }
}

struct LoginGuard(Arc<LoginAttempt>);
impl Drop for LoginGuard {
    fn drop(&mut self) {
        let _guard = LOGIN_WAKE_LOCK.lock().ok();
        self.0.finished.store(true, Ordering::Release);
        LOGIN_RUNNING.store(false, Ordering::Release);
    }
}

pub fn load() -> Result<Option<Credentials>, &'static str> {
    let _guard = STORE_LOCK.lock().map_err(|_| "Trousseau indisponible")?;
    let entry = Entry::new(SERVICE, ACCOUNT).map_err(|_| "Trousseau indisponible")?;
    load_entry(&entry)
}

// Keep the production encoding and error handling testable with an isolated
// entry. Public callers retain STORE_LOCK for the complete operation.
fn load_entry(entry: &Entry) -> Result<Option<Credentials>, &'static str> {
    match entry.get_secret() {
        Ok(secret) => serde_json::from_slice(&secret)
            .map(Some)
            .map_err(|_| "Session enregistree illisible"),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(_) => Err("Trousseau verrouille ou indisponible"),
    }
}

pub fn save_if_current(
    credentials: &Credentials,
    generation: &AtomicU64,
    expected: u64,
) -> Result<bool, &'static str> {
    let _guard = STORE_LOCK.lock().map_err(|_| "Trousseau indisponible")?;
    // Check after acquiring the lock: Logout may already have cleared the store.
    if generation.load(Ordering::Acquire) != expected {
        return Ok(false);
    }
    let entry = Entry::new(SERVICE, ACCOUNT)
        .map_err(|_| "Trousseau indisponible : connexion pour cette session uniquement")?;
    save_entry(&entry, credentials).map(|()| true)
}

fn save_entry(entry: &Entry, credentials: &Credentials) -> Result<(), &'static str> {
    let secret = serde_json::to_vec(credentials).map_err(|_| "Session non enregistrable")?;
    entry
        .set_secret(&secret)
        .map_err(|_| "Trousseau indisponible : connexion pour cette session uniquement")
}

pub fn clear() -> Result<(), &'static str> {
    let _guard = STORE_LOCK.lock().map_err(|_| "Trousseau indisponible")?;
    let entry = match Entry::new(SERVICE, ACCOUNT) {
        Ok(entry) => entry,
        Err(KeyringError::NoEntry) => return Ok(()),
        Err(_) => {
            return Err(
                "Impossible d'effacer la session du trousseau; deverrouillez-le puis reessayez",
            );
        }
    };
    clear_entry(&entry)
}

fn clear_entry(entry: &Entry) -> Result<(), &'static str> {
    match entry.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(_) => {
            Err("Impossible d'effacer la session du trousseau; deverrouillez-le puis reessayez")
        }
    }
}

/// Upstream OAuth's callback listener is blocking, including its `async` variant.
/// Isolate it from the audio and UI loops. Do not log errors containing callback
/// URLs, authorization codes or access tokens.
pub async fn browser_login(
    client_id: String,
    attempt: Arc<LoginAttempt>,
    language: Language,
) -> Result<Credentials, LocalizedText> {
    let background_attempt = attempt.clone();
    let mut task = tokio::task::spawn_blocking(move || {
        if background_attempt.cancelled.load(Ordering::Acquire) {
            background_attempt.finished.store(true, Ordering::Release);
            return Err("Connexion annulee");
        }
        if LOGIN_RUNNING
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            background_attempt.finished.store(true, Ordering::Release);
            return Err("L'autorisation precedente se termine; reessayez dans quelques instants");
        }
        let _guard = LoginGuard(background_attempt.clone());
        let completed = LocalizedText::from(
            "Connexion terminee. Vous pouvez fermer cet onglet et revenir dans Spotiamp.",
        );
        let client = OAuthClientBuilder::new(
            &client_id,
            CALLBACK,
            vec![
                "streaming",
                "user-read-private",
                "playlist-read-private",
                "playlist-read-collaborative",
            ],
        )
        .open_in_browser()
        .with_custom_message(completed.text(language))
        .build()
        .map_err(|_| "Impossible de preparer la connexion Spotify")?;
        if background_attempt.cancelled.load(Ordering::Acquire) {
            return Err("Connexion annulee");
        }
        background_attempt.started.store(true, Ordering::Release);
        client.get_access_token()
            .map(|token| Credentials::with_access_token(token.access_token))
            .map_err(|_| "Connexion annulee ou refusee. Fermez l'onglet puis reessayez; le port local 5588 doit etre libre.")
    });
    match tokio::time::timeout(Duration::from_secs(180), &mut task).await {
        Ok(Ok(result)) => result.map_err(LocalizedText::from),
        Ok(Err(_)) => Err("Le service de connexion s'est arrete".into()),
        Err(_) => {
            attempt.cancel();
            Err("Connexion expiree apres 3 minutes; cliquez sur Connecter pour reessayer".into())
        }
    }
}

/// Wake an outstanding upstream listener on logout/shutdown. A callback without
/// a code makes it exit normally; this never sends a credential over the socket.
fn wake_listener() -> bool {
    let address: SocketAddr = ([127, 0, 0, 1], 5588).into();
    if let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_millis(100)) {
        let _ = stream.set_write_timeout(Some(Duration::from_millis(100)));
        let _ = stream.write_all(b"GET /login?error=access_denied HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancelled_save_checks_epoch_after_acquiring_store_lock_without_touching_keyring() {
        let lock = STORE_LOCK.lock().unwrap();
        let epoch = Arc::new(AtomicU64::new(1));
        let copy = epoch.clone();
        let (started, wait) = std::sync::mpsc::channel();
        let thread = std::thread::spawn(move || {
            started.send(()).unwrap();
            save_if_current(&Credentials::default(), &copy, 1)
        });
        wait.recv().unwrap();
        epoch.store(2, Ordering::Release);
        drop(lock);
        assert_eq!(thread.join().unwrap(), Ok(false));
    }

    #[test]
    fn stored_binary_credentials_round_trip_and_clear_is_idempotent() {
        let entry = Entry::new_with_credential(Box::<keyring::mock::MockCredential>::default());
        let credentials = Credentials {
            username: Some("synthetic-serialization-test".into()),
            auth_type: librespot_protocol::authentication::AuthenticationType::AUTHENTICATION_STORED_SPOTIFY_CREDENTIALS,
            auth_data: (0..=255).collect(),
        };
        assert!(load_entry(&entry).unwrap().is_none());
        save_entry(&entry, &credentials).unwrap();
        assert!(load_entry(&entry).unwrap().as_ref() == Some(&credentials));
        clear_entry(&entry).unwrap();
        clear_entry(&entry).unwrap();
        assert!(load_entry(&entry).unwrap().is_none());
    }

    #[test]
    fn unreadable_or_unavailable_store_is_not_reported_as_no_saved_session() {
        let entry = Entry::new_with_credential(Box::<keyring::mock::MockCredential>::default());
        entry.set_secret(b"synthetic-invalid-json").unwrap();
        assert!(matches!(
            load_entry(&entry),
            Err("Session enregistree illisible")
        ));
        let mock = entry
            .get_credential()
            .downcast_ref::<keyring::mock::MockCredential>()
            .unwrap();
        mock.set_error(KeyringError::PlatformFailure(Box::new(
            std::io::Error::other("synthetic-store-failure"),
        )));
        assert!(matches!(
            load_entry(&entry),
            Err("Trousseau verrouille ou indisponible")
        ));
        // Neither failed read deletes the entry or silently turns it into None.
        assert!(entry.get_secret().unwrap() == b"synthetic-invalid-json");
        clear_entry(&entry).unwrap();
    }
}

#[cfg(all(test, target_os = "windows"))]
#[path = "credentials/windows_tests.rs"]
mod windows_tests;
