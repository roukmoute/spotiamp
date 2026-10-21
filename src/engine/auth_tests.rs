use super::*;
use tokio::sync::Notify;

const OLD_SECRET: &[u8] = b"offline old reusable fixture";
const NEW_SECRET: &[u8] = b"offline rotated reusable fixture";

#[derive(Debug, PartialEq)]
enum Call {
    Load,
    Authenticate(Credentials),
    Browser,
    Save(Credentials),
    Clear,
}

struct Fake {
    calls: Mutex<Vec<Call>>,
    stored: Mutex<Result<Option<Credentials>, &'static str>>,
    reject_saved: bool,
    load_gate: Option<Notify>,
    save_gate: Option<Notify>,
    save_error: Option<&'static str>,
    sessions: Mutex<Vec<Session>>,
}

impl Default for Fake {
    fn default() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            stored: Mutex::new(Ok(None)),
            reject_saved: false,
            load_gate: None,
            save_gate: None,
            save_error: None,
            sessions: Mutex::new(Vec::new()),
        }
    }
}

fn saved() -> Credentials {
    Credentials {
        username: Some("offline-user".into()),
        auth_type: AuthenticationType::AUTHENTICATION_STORED_SPOTIFY_CREDENTIALS,
        auth_data: OLD_SECRET.to_vec(),
    }
}

fn reusable() -> Credentials {
    Credentials {
        auth_data: NEW_SECRET.to_vec(),
        ..saved()
    }
}

impl Fake {
    fn saved() -> Self {
        Self {
            stored: Mutex::new(Ok(Some(saved()))),
            ..Self::default()
        }
    }
    fn record(&self, call: Call) {
        self.calls.lock().unwrap().push(call);
    }
}

impl Backend for Fake {
    fn load(&self) -> Task<'_, Result<Option<Credentials>, &'static str>> {
        Box::pin(async {
            self.record(Call::Load);
            if let Some(gate) = &self.load_gate {
                gate.notified().await;
            }
            self.stored.lock().unwrap().clone()
        })
    }
    fn authenticate(
        &self,
        config: SessionConfig,
        credentials: Credentials,
    ) -> Task<'_, Result<Connected, String>> {
        Box::pin(async move {
            self.record(Call::Authenticate(credentials.clone()));
            assert_eq!(config.autoplay, Some(false));
            if self.reject_saved
                && credentials.auth_type
                    == AuthenticationType::AUTHENTICATION_STORED_SPOTIFY_CREDENTIALS
            {
                return Err("offline rejected session or network failure".into());
            }
            // No connect() call, device, browser or credentials from the OS.
            let session = Session::new(config, None);
            session.set_username("offline-user");
            session.set_auth_data(NEW_SECRET);
            self.sessions.lock().unwrap().push(session.clone());
            Ok(Connected::new(session))
        })
    }
    fn browser(
        &self,
        _: String,
        _: Arc<credentials::LoginAttempt>,
    ) -> Task<'_, Result<Credentials, String>> {
        Box::pin(async {
            self.record(Call::Browser);
            Ok(Credentials::with_access_token("offline OAuth fixture"))
        })
    }
    fn save(
        &self,
        credentials: Credentials,
        epoch: Arc<AtomicU64>,
        generation: u64,
    ) -> Task<'_, Result<bool, &'static str>> {
        Box::pin(async move {
            self.record(Call::Save(credentials.clone()));
            if let Some(gate) = &self.save_gate {
                gate.notified().await;
            }
            if epoch.load(Ordering::Acquire) != generation {
                return Ok(false);
            }
            if let Some(error) = self.save_error {
                return Err(error);
            }
            *self.stored.lock().unwrap() = Ok(Some(credentials));
            Ok(true)
        })
    }
    fn clear(&self) -> Task<'_, Result<(), &'static str>> {
        Box::pin(async {
            self.record(Call::Clear);
            *self.stored.lock().unwrap() = Ok(None);
            Ok(())
        })
    }
}

async fn attempt(backend: &Fake, mode: Mode) -> Result<Connected, String> {
    connect(
        backend,
        mode,
        (mode == Mode::Manual).then(|| Arc::new(credentials::LoginAttempt::default())),
        Arc::new(AtomicU64::new(1)),
        1,
    )
    .await
}

async fn until(mut condition: impl FnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !condition() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("offline worker made no progress");
}

fn worker(backend: Arc<Fake>) -> Worker {
    let mut worker = Worker::new(Arc::new(Mutex::new(Snapshot::default())));
    worker.auth_backend = backend;
    worker
}

#[tokio::test(flavor = "current_thread")]
async fn startup_missing_locked_corrupt_or_failed_credentials_never_open_browser_or_clear_store() {
    for (stored, rejected, expected) in [
        (Ok(None), false, "Connectez Spotify Premium"),
        (
            Err("Trousseau verrouille ou indisponible"),
            false,
            "Trousseau verrouille",
        ),
        (
            Err("Session enregistree illisible"),
            false,
            "Session enregistree illisible",
        ),
        (
            Ok(Some(saved())),
            true,
            "Reconnexion impossible; session conservee",
        ),
    ] {
        let fake = Fake {
            stored: Mutex::new(stored.clone()),
            reject_saved: rejected,
            ..Fake::default()
        };
        let message = attempt(&fake, Mode::Automatic).await.err().unwrap();
        assert!(message.contains(expected), "{message}");
        let expected_calls = if rejected {
            vec![Call::Load, Call::Authenticate(saved())]
        } else {
            vec![Call::Load]
        };
        assert_eq!(*fake.calls.lock().unwrap(), expected_calls);
        assert_eq!(*fake.stored.lock().unwrap(), stored);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn restored_connection_rotates_reusable_credentials_without_oauth() {
    let fake = Fake::saved();
    let connection = attempt(&fake, Mode::Automatic).await.unwrap();
    assert!(connection.notice.is_none());
    assert_eq!(*fake.stored.lock().unwrap(), Ok(Some(reusable())));
    assert_eq!(
        *fake.calls.lock().unwrap(),
        vec![
            Call::Load,
            Call::Authenticate(saved()),
            Call::Save(reusable())
        ]
    );
    drop(connection);
    assert!(fake.sessions.lock().unwrap()[0].is_invalid());
}

#[tokio::test(flavor = "current_thread")]
async fn manual_connect_allows_oauth_after_missing_or_failed_stored_login_and_saves_no_token() {
    for fake in [
        Fake::default(),
        Fake {
            reject_saved: true,
            ..Fake::saved()
        },
    ] {
        let _connection = attempt(&fake, Mode::Manual).await.unwrap();
        let calls = fake.calls.lock().unwrap();
        assert_eq!(
            calls.iter().filter(|call| **call == Call::Browser).count(),
            1
        );
        assert_eq!(calls.last(), Some(&Call::Save(reusable())));
        assert_eq!(*fake.stored.lock().unwrap(), Ok(Some(reusable())));
    }
}

#[tokio::test(flavor = "current_thread")]
async fn failed_or_unconfirmed_save_does_not_claim_persistence() {
    for (error, expected) in [
        (
            "Trousseau indisponible : connexion pour cette session uniquement",
            "session uniquement",
        ),
        (STORE_TIMED_OUT, "sauvegarde non confirmee"),
    ] {
        let fake = Fake {
            save_error: Some(error),
            ..Fake::saved()
        };
        let connection = attempt(&fake, Mode::Automatic).await.unwrap();
        assert!(connection.notice.as_ref().unwrap().contains(expected));
        assert_eq!(*fake.stored.lock().unwrap(), Ok(Some(saved())));
    }
}

#[tokio::test(flavor = "current_thread")]
async fn worker_starts_once_remains_responsive_coalesces_connect_and_clears_busy_on_missing_session()
 {
    let fake = Arc::new(Fake {
        load_gate: Some(Notify::new()),
        ..Fake::default()
    });
    let worker = worker(fake.clone());
    let state = worker.state.clone();
    let (tx, rx) = mpsc::channel(8);
    let running = tokio::spawn(worker.run(rx));
    until(|| !fake.calls.lock().unwrap().is_empty()).await;
    assert!(state.lock().unwrap().busy);
    tx.send(Command::Connect).await.unwrap();
    tx.send(Command::Connect).await.unwrap();
    tx.send(Command::Volume(123)).await.unwrap();
    until(|| state.lock().unwrap().volume == 123).await;
    assert_eq!(*fake.calls.lock().unwrap(), vec![Call::Load]);
    fake.load_gate.as_ref().unwrap().notify_one();
    until(|| !state.lock().unwrap().busy).await;
    assert!(!state.lock().unwrap().connected);
    assert!(
        state
            .lock()
            .unwrap()
            .status
            .contains("Connectez Spotify Premium")
    );
    tx.send(Command::Volume(456)).await.unwrap();
    until(|| state.lock().unwrap().volume == 456).await;
    tx.send(Command::Shutdown).await.unwrap();
    running.await.unwrap();
    assert_eq!(*fake.calls.lock().unwrap(), vec![Call::Load]);
}

#[tokio::test(flavor = "current_thread")]
async fn worker_publishes_connected_only_after_save_and_fresh_worker_reconnects_after_close() {
    let fake = Arc::new(Fake {
        save_gate: Some(Notify::new()),
        ..Fake::saved()
    });
    for launch in 1..=2 {
        // A new worker models closing and reopening the process. Only the
        // injected OS credential store survives between these two launches.
        let worker = worker(fake.clone());
        let state = worker.state.clone();
        let (tx, rx) = mpsc::channel(8);
        let running = tokio::spawn(worker.run(rx));
        until(|| {
            fake.calls
                .lock()
                .unwrap()
                .iter()
                .filter(|call| matches!(call, Call::Save(_)))
                .count()
                == launch
        })
        .await;
        assert!(state.lock().unwrap().busy);
        assert!(!state.lock().unwrap().connected);
        fake.save_gate.as_ref().unwrap().notify_one();
        until(|| state.lock().unwrap().connected).await;
        assert!(!state.lock().unwrap().busy);
        assert_eq!(*fake.stored.lock().unwrap(), Ok(Some(reusable())));
        // Closing immediately after the visible connected state is now safe.
        tx.send(Command::Shutdown).await.unwrap();
        running.await.unwrap();
        assert_eq!(*fake.stored.lock().unwrap(), Ok(Some(reusable())));
        assert!(
            fake.sessions
                .lock()
                .unwrap()
                .iter()
                .all(Session::is_invalid)
        );
    }
    assert_eq!(
        *fake.calls.lock().unwrap(),
        vec![
            Call::Load,
            Call::Authenticate(saved()),
            Call::Save(reusable()),
            Call::Load,
            Call::Authenticate(reusable()),
            Call::Save(reusable()),
        ]
    );
}

#[tokio::test(flavor = "current_thread")]
async fn logout_cancels_pending_startup_save_and_does_not_resurrect_credentials() {
    let fake = Arc::new(Fake {
        save_gate: Some(Notify::new()),
        ..Fake::saved()
    });
    let worker = worker(fake.clone());
    let state = worker.state.clone();
    let (tx, rx) = mpsc::channel(8);
    let running = tokio::spawn(worker.run(rx));
    until(|| {
        fake.calls
            .lock()
            .unwrap()
            .iter()
            .any(|call| matches!(call, Call::Save(_)))
    })
    .await;
    tx.send(Command::Logout).await.unwrap();
    until(|| {
        state
            .lock()
            .unwrap()
            .status
            .contains("identifiants effaces")
    })
    .await;
    fake.save_gate.as_ref().unwrap().notify_one();
    tx.send(Command::Volume(999)).await.unwrap();
    until(|| state.lock().unwrap().volume == 999).await;
    assert!(!state.lock().unwrap().connected);
    assert_eq!(*fake.stored.lock().unwrap(), Ok(None));
    assert!(fake.sessions.lock().unwrap()[0].is_invalid());
    tx.send(Command::Shutdown).await.unwrap();
    running.await.unwrap();
    assert_eq!(
        fake.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|call| **call == Call::Clear)
            .count(),
        1
    );
}

#[tokio::test(flavor = "current_thread")]
async fn logout_rejects_already_queued_auto_success() {
    let fake = Arc::new(Fake::saved());
    let mut worker = worker(fake.clone());
    let (updates, mut rx) = mpsc::channel(8);
    worker.start_connection(Mode::Automatic, &updates);
    let result = rx.recv().await.unwrap();
    assert!(!fake.sessions.lock().unwrap()[0].is_invalid());
    worker.command(Command::Logout, &updates);
    worker.update(result, &updates);
    worker.update(rx.recv().await.unwrap(), &updates);
    assert!(!worker.snapshot().connected);
    assert!(!worker.snapshot().busy);
    assert_eq!(*fake.stored.lock().unwrap(), Ok(None));
    assert!(fake.sessions.lock().unwrap()[0].is_invalid());
}

#[tokio::test(flavor = "current_thread")]
async fn cancel_preserves_store_stale_result_does_not_clear_new_attempt_and_late_cancel_keeps_session()
 {
    let fake = Arc::new(Fake::saved());
    let mut worker = worker(fake.clone());
    let (updates, mut rx) = mpsc::channel(8);
    worker.start_connection(Mode::Automatic, &updates);
    let stale_result = rx.recv().await.unwrap();
    worker.command(Command::CancelConnect, &updates);
    assert_eq!(*fake.stored.lock().unwrap(), Ok(Some(reusable())));
    worker.command(Command::Connect, &updates);
    worker.update(stale_result, &updates);
    assert!(worker.connect_task.is_some());
    assert!(fake.sessions.lock().unwrap()[0].is_invalid());
    worker.update(rx.recv().await.unwrap(), &updates);
    assert!(worker.snapshot().connected);
    worker.command(Command::CancelConnect, &updates);
    assert!(worker.snapshot().connected);
    assert!(!fake.calls.lock().unwrap().contains(&Call::Clear));
    worker.disconnect();
}

#[tokio::test(flavor = "current_thread")]
async fn native_store_wait_is_bounded_even_when_os_operation_continues() {
    let (release, blocked) = std::sync::mpsc::channel();
    let result = store_job(Duration::from_millis(15), move || {
        let _ = blocked.recv_timeout(Duration::from_secs(2));
        Ok(())
    })
    .await;
    // Release before assertions: a failure cannot leave a runtime stuck on an
    // artificial OS operation. No real keyring is consulted by this test.
    let _ = release.send(());
    assert_eq!(result, Err(STORE_TIMED_OUT));
}
