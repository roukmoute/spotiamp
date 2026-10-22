//! Small About actions. Only a fixed project URL leaves the application; the
//! copied diagnostic text is an explicit allowlist, never a snapshot dump.
use crate::{locale::Language, localized_text::LocalizedText, model::Snapshot};
use std::{
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver, TryRecvError},
};

pub const PROJECT_URL: &str = "https://github.com/roukmoute/spotiamp";

#[cfg(test)]
pub fn diagnostic_info(snapshot: &Snapshot) -> String {
    diagnostic_info_in(snapshot, Language::French)
}

pub fn diagnostic_info_in(snapshot: &Snapshot, language: Language) -> String {
    #[cfg(feature = "spotify")]
    let (features, engine) = (
        language.text("lecture Spotify", "Spotify playback"),
        format!("librespot {}", librespot_core::version::SEMVER),
    );
    #[cfg(not(feature = "spotify"))]
    let (features, engine) = (
        language.text("interface seule", "UI only"),
        language
            .text("aucun (interface seule)", "none (UI only)")
            .to_owned(),
    );

    let labels = [
        language.text("Systeme", "System"),
        language.text("Architecture", "Architecture"),
        language.text("Fonctionnalites", "Features"),
        language.text("Moteur", "Engine"),
        language.text("Qualite preferee", "Preferred quality"),
        language.text("Projet", "Project"),
    ];
    format!(
        "Spotiamp {}\n{} : {}\n{} : {}\n{} : {}\n{} : {}\n{} : {} kbit/s\n{} : {}\n",
        env!("CARGO_PKG_VERSION"),
        labels[0],
        std::env::consts::OS,
        labels[1],
        std::env::consts::ARCH,
        labels[2],
        features,
        labels[3],
        engine,
        labels[4],
        snapshot.quality.kbps(),
        labels[5],
        PROJECT_URL,
    )
}

#[derive(Default)]
pub struct ProjectOpener {
    pending: Option<Receiver<Result<(), LocalizedText>>>,
}

impl ProjectOpener {
    pub fn start(&mut self) -> Result<(), LocalizedText> {
        if self.pending.is_some() {
            return Err(LocalizedText::new(
                "Ouverture du projet deja en cours.",
                "The project page is already opening.",
            ));
        }
        let (sender, receiver) = mpsc::channel();
        // Some desktop launchers wait for the browser. Keep that wait outside
        // the UI thread, and allow only one outstanding launch operation.
        std::thread::Builder::new()
            .name("spotiamp-project-link".into())
            .spawn(move || {
                let result = project_command().and_then(|mut command| {
                    command
                        .status()
                        .map_err(|_| {
                            LocalizedText::new(
                                "Impossible de lancer le navigateur.",
                                "Cannot launch the browser.",
                            )
                        })
                        .and_then(|status| {
                            if status.success() {
                                Ok(())
                            } else {
                                Err(LocalizedText::new(
                                    "L'ouverture du projet dans le navigateur a echoue.",
                                    "Could not open the project page in your browser.",
                                ))
                            }
                        })
                });
                let _ = sender.send(result);
            })
            .map_err(|_| {
                LocalizedText::new(
                    "Impossible de demarrer l'ouverture du projet.",
                    "Cannot start opening the project page.",
                )
            })?;
        self.pending = Some(receiver);
        Ok(())
    }

    pub fn poll(&mut self) -> Option<Result<(), LocalizedText>> {
        let result = match self.pending.as_ref()?.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Err(LocalizedText::new(
                "L'ouverture du projet s'est interrompue.",
                "Opening the project page was interrupted.",
            )),
        };
        self.pending = None;
        Some(result)
    }
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
fn project_command() -> Result<Command, LocalizedText> {
    #[cfg(target_os = "windows")]
    let mut command = {
        use std::os::windows::process::CommandExt;
        let mut command = Command::new("cmd.exe");
        // START consumes its first quoted argument as a window title. The
        // empty title and fixed URL are separate arguments; no user input is
        // interpolated into shell code. /D disables Command Prompt AutoRun.
        command.args(["/D", "/C", "start", "", PROJECT_URL]);
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        command
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("/usr/bin/open");
        command.arg(PROJECT_URL);
        command
    };
    #[cfg(target_os = "linux")]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(PROJECT_URL);
        command
    };
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    Ok(command)
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn project_command() -> Result<Command, LocalizedText> {
    Err(LocalizedText::new(
        "Ouverture du navigateur non prise en charge sur ce systeme.",
        "Opening a browser is not supported on this system.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AudioQuality, TrackRow};

    #[test]
    fn diagnostics_contain_only_public_build_and_quality_fields() {
        let private = "private-account-track-or-session";
        let snapshot = Snapshot {
            connected: true,
            status: private.into(),
            queue: vec![TrackRow {
                uri: private.into(),
                title: private.into(),
                artist: private.into(),
                ..TrackRow::default()
            }],
            quality: AudioQuality::Kbps96,
            ..Snapshot::default()
        };
        let info = diagnostic_info(&snapshot);
        assert!(info.contains(env!("CARGO_PKG_VERSION")));
        assert!(info.contains(std::env::consts::OS));
        assert!(info.contains(std::env::consts::ARCH));
        assert!(info.contains("Qualite preferee : 96 kbit/s"));
        assert!(info.contains(PROJECT_URL));
        assert!(!info.contains(private));
        assert_eq!(info.lines().count(), 7);
        #[cfg(feature = "spotify")]
        {
            assert!(info.contains("Fonctionnalites : lecture Spotify"));
            assert!(info.contains(librespot_core::version::SEMVER));
        }
        #[cfg(not(feature = "spotify"))]
        assert!(info.contains("Moteur : aucun (interface seule)"));
    }

    #[test]
    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    fn browser_command_contains_only_the_fixed_project_url() {
        let command = project_command().unwrap();
        let args: Vec<_> = command
            .get_args()
            .map(|arg| arg.to_str().unwrap())
            .collect();
        #[cfg(target_os = "windows")]
        {
            assert_eq!(command.get_program(), "cmd.exe");
            assert_eq!(args, ["/D", "/C", "start", "", PROJECT_URL]);
        }
        #[cfg(target_os = "macos")]
        {
            assert_eq!(command.get_program(), "/usr/bin/open");
            assert_eq!(args, [PROJECT_URL]);
        }
        #[cfg(target_os = "linux")]
        {
            assert_eq!(command.get_program(), "xdg-open");
            assert_eq!(args, [PROJECT_URL]);
        }
    }

    #[test]
    fn opener_rejects_overlapping_requests_and_reports_completion_once() {
        let (sender, receiver) = mpsc::channel();
        let mut opener = ProjectOpener {
            pending: Some(receiver),
        };
        assert!(opener.start().is_err()); // Must not launch a real browser.
        assert!(opener.poll().is_none());
        sender.send(Ok(())).unwrap();
        assert_eq!(opener.poll(), Some(Ok(())));
        assert!(opener.poll().is_none());
        assert!(opener.pending.is_none());

        let (sender, receiver) = mpsc::channel();
        opener.pending = Some(receiver);
        drop(sender);
        assert!(matches!(opener.poll(), Some(Err(_))));
        assert!(opener.pending.is_none());
    }
}
