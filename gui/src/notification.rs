//! Desktop notifications for `--background` mode — same shell-out-to-
//! `notify-send`, best-effort stance as `kio-protondrive`'s
//! `daemon::notification`, and the same reason for a `Notifier` trait: a
//! developer running `cargo test` must never see a real notification pop up
//! from a test.
//!
//! Lives in the GUI, not the daemon: the daemon may run headless on a
//! remote server with no desktop session to notify (see the plan's
//! "Déploiement distant" and `daemon::poll`'s doc comment) — only a client
//! machine has one.

use std::process::Command;

pub trait Notifier {
    fn new_strip(&self, number: i32);
    fn new_rant(&self, number: i32);
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RealNotifier;

impl Notifier for RealNotifier {
    fn new_strip(&self, number: i32) {
        send(
            &format!("New strip: #{number}"),
            "Open Megatokyo to read it.",
        );
    }

    fn new_rant(&self, number: i32) {
        send(
            &format!("New rant: #{number}"),
            "Open Megatokyo to read it.",
        );
    }
}

fn send(summary: &str, body: &str) {
    let result = Command::new("notify-send")
        .arg("--app-name=Megatokyo")
        .arg("--urgency=normal")
        .arg(summary)
        .arg(body)
        .status();
    if let Err(err) = result {
        log::debug!("could not send a desktop notification (notify-send missing?): {err}");
    }
}

#[cfg(test)]
pub mod tests_support {
    use super::Notifier;
    use std::cell::RefCell;

    /// Records calls instead of shelling out to a real `notify-send` — see
    /// this module's own doc comment for why.
    #[derive(Default)]
    pub struct RecordingNotifier {
        pub strips: RefCell<Vec<i32>>,
        pub rants: RefCell<Vec<i32>>,
    }

    impl Notifier for RecordingNotifier {
        fn new_strip(&self, number: i32) {
            self.strips.borrow_mut().push(number);
        }

        fn new_rant(&self, number: i32) {
            self.rants.borrow_mut().push(number);
        }
    }
}
