//! One-shot authentication policy. Startup never opens a browser or deletes a
//! stored session; only an explicit manual connection may start OAuth.
use std::{future::Future, pin::Pin};

use super::*;

const STORE_TIMEOUT: Duration = Duration::from_secs(5);
const CONNECT_FAILED: &str =
    "Connexion Spotify impossible. Verifiez le reseau et votre abonnement Premium.";
const SAVE_UNCONFIRMED: &str =
    "Connecte; sauvegarde non confirmee, reconnexion automatique non garantie";

type Task<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Mode {
    Automatic,
    Manual,
}

// A cancelled attempt or a result discarded after Logout must also shut down
// its session. Accepted connections explicitly transfer ownership to Worker.
pub(super) struct Connected {
    session: Option<Session>,
    pub notice: Option<String>,
}

impl Connected {
    fn new(session: Session) -> Self {
        Self {
            session: Some(session),
            notice: None,
        }
    }

    pub fn take_session(&mut self) -> Session {
        self.session.take().expect("connection consumed once")
    }

    fn reusable(&self) -> Credentials {
        let session = self.session.as_ref().expect("pending connection");
        Credentials {
            username: Some(session.username()),
            auth_type: AuthenticationType::AUTHENTICATION_STORED_SPOTIFY_CREDENTIALS,
            auth_data: session.auth_data(),
        }
    }
}

impl Drop for Connected {
    fn drop(&mut self) {
        if let Some(session) = &self.session {
            session.shutdown();
        }
    }
}

// Only authentication uses this interface. Tests exercise the production
// policy and worker dispatch without contacting Spotify or the real keyring.
pub(super) trait Backend: Send + Sync {
    fn load(&self) -> Task<'_, Result<Option<Credentials>, &'static str>>;
    fn authenticate(
        &self,
        config: SessionConfig,
        credentials: Credentials,
    ) -> Task<'_, Result<Connected, String>>;
    fn browser(
        &self,
        client_id: String,
        attempt: Arc<credentials::LoginAttempt>,
    ) -> Task<'_, Result<Credentials, String>>;
    fn save(
        &self,
        credentials: Credentials,
        epoch: Arc<AtomicU64>,
        generation: u64,
    ) -> Task<'_, Result<bool, &'static str>>;
    fn clear(&self) -> Task<'_, Result<(), &'static str>>;
}

pub(super) struct Native;

impl Backend for Native {
    fn load(&self) -> Task<'_, Result<Option<Credentials>, &'static str>> {
        Box::pin(store_job(STORE_TIMEOUT, credentials::load))
    }

    fn authenticate(
        &self,
        config: SessionConfig,
        credentials: Credentials,
    ) -> Task<'_, Result<Connected, String>> {
        Box::pin(async move {
            let connection = Connected::new(Session::new(config, None));
            connection
                .session
                .as_ref()
                .expect("pending connection")
                .connect(credentials, false)
                .await
                .map_err(|_| CONNECT_FAILED.to_owned())?;
            Ok(connection)
        })
    }

    fn browser(
        &self,
        client_id: String,
        attempt: Arc<credentials::LoginAttempt>,
    ) -> Task<'_, Result<Credentials, String>> {
        Box::pin(credentials::browser_login(client_id, attempt))
    }

    fn save(
        &self,
        credentials: Credentials,
        epoch: Arc<AtomicU64>,
        generation: u64,
    ) -> Task<'_, Result<bool, &'static str>> {
        Box::pin(store_job(STORE_TIMEOUT, move || {
            credentials::save_if_current(&credentials, &epoch, generation)
        }))
    }

    fn clear(&self) -> Task<'_, Result<(), &'static str>> {
        // Do not cancel a queued deletion: Logout must still clear a prior
        // in-progress save once it releases STORE_LOCK. This wait is async.
        Box::pin(async {
            tokio::task::spawn_blocking(credentials::clear)
                .await
                .unwrap_or(Err("Le service du trousseau s'est arrete"))
        })
    }
}

const STORE_TIMED_OUT: &str = "Le trousseau ne repond pas";

struct StoreTask<T>(JoinHandle<Result<T, &'static str>>);
impl<T> Drop for StoreTask<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn store_job<T: Send + 'static>(
    limit: Duration,
    job: impl FnOnce() -> Result<T, &'static str> + Send + 'static,
) -> Result<T, &'static str> {
    let mut task = StoreTask(tokio::task::spawn_blocking(job));
    match tokio::time::timeout(limit, &mut task.0).await {
        Ok(result) => result.unwrap_or(Err("Le service du trousseau s'est arrete")),
        Err(_) => {
            // Removes a queued job; an OS call already in progress may finish
            // later. Saves still check the epoch under the credential lock.
            task.0.abort();
            Err(STORE_TIMED_OUT)
        }
    }
}

pub(super) async fn connect(
    backend: &dyn Backend,
    mode: Mode,
    attempt: Option<Arc<credentials::LoginAttempt>>,
    epoch: Arc<AtomicU64>,
    generation: u64,
) -> Result<Connected, String> {
    let config = SessionConfig {
        autoplay: Some(false),
        ..SessionConfig::default()
    };
    let saved = backend.load().await;
    let mut connected = match saved {
        Ok(Some(credentials)) => {
            match tokio::time::timeout(
                NETWORK_TIMEOUT,
                backend.authenticate(config.clone(), credentials),
            )
            .await
            {
                Ok(Ok(connection)) => Some(connection),
                _ if mode == Mode::Automatic => {
                    return Err("Reconnexion impossible; session conservee. Cliquez sur Connecter pour reessayer.".into());
                }
                _ => None,
            }
        }
        Ok(None) if mode == Mode::Automatic => {
            return Err("Connectez Spotify Premium pour commencer".into());
        }
        Err(message) if mode == Mode::Automatic => {
            return Err(format!(
                "{message}. Cliquez sur Connecter pour vous connecter."
            ));
        }
        _ => None,
    };
    if connected.is_none() {
        // Automatic mode has returned above on every unsuccessful store path.
        let attempt = attempt.ok_or("Connexion annulee")?;
        let credentials = backend.browser(config.client_id.clone(), attempt).await?;
        connected = Some(
            tokio::time::timeout(NETWORK_TIMEOUT, backend.authenticate(config, credentials))
                .await
                .map_err(|_| CONNECT_FAILED.to_owned())??,
        );
    }
    let mut connection = connected.expect("successful authentication");
    // Reconnects may rotate the reusable secret. Persist the session's freshly
    // returned credentials, never the OAuth access token used to authenticate.
    // Publishing Connected only afterwards removes the normal quick-close gap.
    connection.notice = match backend.save(connection.reusable(), epoch, generation).await {
        Ok(true) => None,
        Ok(false) => return Err("Connexion annulee".into()),
        Err(STORE_TIMED_OUT) => Some(SAVE_UNCONFIRMED.into()),
        Err(message) => Some(format!("Connecte; {message}")),
    };
    Ok(connection)
}

#[cfg(test)]
#[path = "auth_tests.rs"]
mod tests;
