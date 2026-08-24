mod background;
mod config;
mod control;
mod daemon_link;
mod launcher;
mod notification;

fn main() -> std::process::ExitCode {
    env_logger::init();

    if std::env::args().any(|arg| arg == "--background") {
        let interval = std::time::Duration::from_secs(daemon_link::poll_interval_minutes() * 60);
        background::run(&notification::RealNotifier, interval);
        // background::run loops forever; reached only if that ever changes.
        std::process::ExitCode::SUCCESS
    } else {
        match daemon_link::resolve() {
            Some(link) => launcher::run(&link),
            None => {
                eprintln!(
                    "Could not find or start a megatokyo daemon. Is megatokyo-daemon installed?"
                );
                std::process::ExitCode::FAILURE
            }
        }
    }
}
