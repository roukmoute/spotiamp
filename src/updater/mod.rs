//! On-demand update workers. No polling thread or HTTP client is kept at idle.
use crate::localized_text::LocalizedText;
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(not(feature = "updates"), allow(dead_code))]
pub enum UpdateStatus {
    Disabled,
    Idle,
    Checking,
    Current,
    Available { version: String },
    Downloading { percent: u8 },
    Ready { version: String },
    Installing,
    Failed(LocalizedText),
}

impl Default for UpdateStatus {
    fn default() -> Self {
        if cfg!(feature = "updates") {
            Self::Idle
        } else {
            Self::Disabled
        }
    }
}

#[cfg(feature = "updates")]
pub mod install;
#[cfg(feature = "updates")]
mod package;
#[cfg(feature = "updates")]
mod release;

#[cfg(feature = "updates")]
mod worker {
    use super::{UpdateStatus, install, package, release};
    use crate::localized_text::LocalizedText;
    use std::{
        sync::mpsc::{self, Receiver, SyncSender, TryRecvError},
        time::Duration,
    };

    enum Event {
        Checked(Result<Option<release::Release>, LocalizedText>),
        Progress(u8),
        Prepared(Result<install::Prepared, LocalizedText>),
        Handoff(Result<(), LocalizedText>),
    }

    pub struct Controller {
        pub status: UpdateStatus,
        pending: Option<Receiver<Event>>,
        release: Option<release::Release>,
        prepared: Option<install::Prepared>,
        initial_checked: bool,
    }

    impl Default for Controller {
        fn default() -> Self {
            Self {
                status: if release::platform().is_some() {
                    UpdateStatus::Idle
                } else {
                    UpdateStatus::Disabled
                },
                pending: None,
                release: None,
                prepared: None,
                initial_checked: false,
            }
        }
    }

    impl Controller {
        pub fn automatic_check(&mut self, elapsed: Duration) {
            if !self.initial_checked && elapsed >= Duration::from_secs(3) {
                self.check();
            }
        }

        fn start(&mut self, task: impl FnOnce(SyncSender<Event>) + Send + 'static) {
            let (tx, rx) = mpsc::sync_channel(8);
            match std::thread::Builder::new()
                .name("spotiamp-update".into())
                .spawn(move || task(tx))
            {
                Ok(_) => self.pending = Some(rx),
                Err(_) => {
                    self.status = UpdateStatus::Failed(LocalizedText::new(
                        "Impossible de lancer la mise a jour.",
                        "Could not start the update.",
                    ))
                }
            }
        }

        pub fn check(&mut self) {
            self.initial_checked = true;
            if self.pending.is_some() || self.status == UpdateStatus::Disabled {
                return;
            }
            self.prepared = None;
            self.release = None;
            self.status = UpdateStatus::Checking;
            self.start(|tx| {
                let _ = tx.send(Event::Checked(release::check()));
            });
        }

        pub fn download(&mut self) {
            if self.pending.is_some() || !matches!(self.status, UpdateStatus::Available { .. }) {
                return;
            }
            let Some(release) = self.release.clone() else {
                return;
            };
            self.status = UpdateStatus::Downloading { percent: 0 };
            self.start(move |tx| {
                let prepared = package::prepare(&release, |percent| {
                    // Progress never blocks the network worker or fills an unbounded queue.
                    let _ = tx.try_send(Event::Progress(percent));
                });
                let _ = tx.send(Event::Prepared(prepared));
            });
        }

        pub fn install(&mut self, scale2: bool) {
            if self.pending.is_some() || !matches!(self.status, UpdateStatus::Ready { .. }) {
                return;
            }
            let Some(prepared) = self.prepared.take() else {
                return;
            };
            self.status = UpdateStatus::Installing;
            self.start(move |tx| {
                let _ = tx.send(Event::Handoff(install::launch(prepared, scale2)));
            });
        }

        /// True only after the installer has acknowledged the handoff.
        pub fn poll(&mut self) -> bool {
            loop {
                let Some(rx) = self.pending.as_ref() else {
                    return false;
                };
                let event = match rx.try_recv() {
                    Ok(event) => event,
                    Err(TryRecvError::Empty) => return false,
                    Err(TryRecvError::Disconnected) => {
                        self.pending = None;
                        self.status = UpdateStatus::Failed(LocalizedText::new(
                            "La verification s'est interrompue. Reessayez.",
                            "The update check was interrupted. Try again.",
                        ));
                        return false;
                    }
                };
                match event {
                    Event::Progress(percent) => self.status = UpdateStatus::Downloading { percent },
                    Event::Checked(result) => {
                        self.pending = None;
                        self.status = match result {
                            Ok(Some(release)) => {
                                let version = release.version.clone();
                                self.release = Some(release);
                                UpdateStatus::Available { version }
                            }
                            Ok(None) => UpdateStatus::Current,
                            Err(error) => UpdateStatus::Failed(error),
                        };
                        return false;
                    }
                    Event::Prepared(result) => {
                        self.pending = None;
                        self.status = match result {
                            Ok(prepared) => {
                                self.prepared = Some(prepared);
                                UpdateStatus::Ready {
                                    version: self.release.as_ref().unwrap().version.clone(),
                                }
                            }
                            Err(error) => UpdateStatus::Failed(error),
                        };
                        return false;
                    }
                    Event::Handoff(result) => {
                        self.pending = None;
                        match result {
                            Ok(()) => return true,
                            Err(error) => self.status = UpdateStatus::Failed(error),
                        }
                        return false;
                    }
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::locale::Language;

        #[test]
        fn a_failed_update_can_change_language_without_repeating_the_request() {
            let mut controller = Controller::default();
            let (tx, rx) = mpsc::sync_channel(8);
            controller.pending = Some(rx);
            tx.send(Event::Checked(Err(LocalizedText::new(
                "Reseau indisponible : connexion refusee",
                "Network unavailable: connection refused",
            ))))
            .unwrap();
            assert!(!controller.poll());
            let UpdateStatus::Failed(message) = &controller.status else {
                panic!("Expected an update failure");
            };
            assert_eq!(
                message.text(Language::French),
                "Reseau indisponible : connexion refusee"
            );
            assert_eq!(
                message.text(Language::English),
                "Network unavailable: connection refused"
            );
            assert!(controller.pending.is_none());
        }

        #[test]
        fn worker_errors_release_busy_state_and_never_request_exit() {
            let mut controller = Controller::default();
            let (tx, rx) = mpsc::sync_channel(8);
            controller.pending = Some(rx);
            tx.send(Event::Checked(Err("offline".into()))).unwrap();
            assert!(!controller.poll());
            assert_eq!(controller.status, UpdateStatus::Failed("offline".into()));
            assert!(controller.pending.is_none());
            controller.install(false);
            controller.download();
            assert!(controller.pending.is_none());
        }
        #[test]
        fn only_successful_installer_handoff_requests_exit() {
            for success in [false, true] {
                let mut controller = Controller::default();
                let (tx, rx) = mpsc::sync_channel(8);
                controller.pending = Some(rx);
                tx.send(Event::Handoff(if success {
                    Ok(())
                } else {
                    Err("denied".into())
                }))
                .unwrap();
                assert_eq!(controller.poll(), success);
            }
        }
    }
}

#[cfg(feature = "updates")]
pub use worker::Controller;

#[cfg(not(feature = "updates"))]
#[derive(Default)]
pub struct Controller {
    pub status: UpdateStatus,
}
#[cfg(not(feature = "updates"))]
impl Controller {
    pub fn automatic_check(&mut self, _: std::time::Duration) {}
    pub fn check(&mut self) {}
    pub fn download(&mut self) {}
    pub fn install(&mut self, _: bool) {}
    pub fn poll(&mut self) -> bool {
        false
    }
}
