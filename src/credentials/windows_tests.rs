//! Native Windows Credential Manager persistence across two independent processes.
//! Every entry uses a random test-only namespace. Never call load/save/clear,
//! which address the user's real Spotify entry; exercise their private wrappers.
use super::*;
use std::{
    process::{Child, Command, Stdio},
    time::Instant,
};

const CHILD_TEST: &str = "credentials::windows_tests::native_store_child";
const NONCE_ENV: &str = "SPOTIAMP_TEST_KEYRING_NONCE";

fn test_entry(nonce: &str) -> Entry {
    // Derive both names ourselves: no environment value can select a real
    // service/account, even if the ignored child test is invoked manually.
    assert!(nonce.len() == 32 && nonce.bytes().all(|byte| byte.is_ascii_hexdigit()));
    let service = format!("org.spotiamp.tests.native-keyring.{nonce}");
    let account = format!("synthetic-account-{nonce}");
    assert!(service != SERVICE && account != ACCOUNT);
    let entry = Entry::new(&service, &account).expect("create synthetic native entry");
    assert!(
        entry
            .get_credential()
            .is::<keyring::windows::WinCredential>(),
        "this test must use the actual Windows backend, never the mock store"
    );
    entry
}

fn synthetic_credentials(nonce: &str) -> Credentials {
    Credentials {
        username: Some(format!("synthetic-user-{nonce}")),
        auth_type: librespot_protocol::authentication::AuthenticationType::AUTHENTICATION_STORED_SPOTIFY_CREDENTIALS,
        // Deliberately binary, not a password/token. Never sent to Spotify.
        auth_data: (0..=255).collect(),
    }
}

struct DeleteOnDrop(Entry);

impl Drop for DeleteOnDrop {
    fn drop(&mut self) {
        // Also runs on assertion failure. Normal completion explicitly checks
        // deletion; the parent repeats cleanup if its child failed or was killed.
        let _ = clear_entry(&self.0);
    }
}

struct ReapOnDrop(Child);

impl Drop for ReapOnDrop {
    fn drop(&mut self) {
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

#[test]
fn native_credentials_persist_between_processes_and_delete_cleanly() {
    let nonce = format!("{:032x}", rand::random::<u128>());
    let entry = test_entry(&nonce);
    assert!(
        load_entry(&entry).expect("read fresh test entry").is_none(),
        "refuse to overwrite any pre-existing entry"
    );
    let cleanup = DeleteOnDrop(entry);
    save_entry(&cleanup.0, &synthetic_credentials(&nonce)).expect("save synthetic credential");

    // The child has a fresh address space, Entry and Windows API call. It reads
    // the exact serialized secret and removes it; no shared mock can pass this.
    let mut child = ReapOnDrop(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                CHILD_TEST,
                "--ignored",
                "--test-threads=1",
                "--nocapture",
            ])
            .env(NONCE_ENV, &nonce)
            .stdin(Stdio::null())
            .spawn()
            .expect("start native credential reader subprocess"),
    );
    let started = Instant::now();
    loop {
        if let Some(status) = child.0.try_wait().expect("wait for credential subprocess") {
            assert!(status.success(), "native credential subprocess failed");
            break;
        }
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "native credential subprocess timed out"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
    assert!(
        load_entry(&cleanup.0)
            .expect("check child's deletion")
            .is_none()
    );
    clear_entry(&cleanup.0).expect("native deletion must be idempotent");
}

#[test]
#[ignore = "internal subprocess fixture launched by the native persistence test"]
fn native_store_child() {
    let Ok(nonce) = std::env::var(NONCE_ENV) else {
        return;
    };
    let cleanup = DeleteOnDrop(test_entry(&nonce));
    let expected = synthetic_credentials(&nonce);
    let loaded = load_entry(&cleanup.0).expect("read synthetic credential from another process");
    // Boolean assertions intentionally avoid formatting any credential content.
    assert!(
        loaded.as_ref() == Some(&expected),
        "stored synthetic credential differs"
    );
    assert!(
        cleanup
            .0
            .get_secret()
            .expect("read native serialized secret")
            == serde_json::to_vec(&expected).unwrap(),
        "stored serialization differs"
    );
    clear_entry(&cleanup.0).expect("delete synthetic native entry");
    assert!(
        load_entry(&cleanup.0)
            .expect("check deleted test entry")
            .is_none()
    );
}
