// bin/oc-rsync/src/daemon.rs
use oc_rsync_cli::version;

fn main() {
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        if !std::env::args().any(|a| a == "--quiet" || a == "-q") {
            println!("{}", version::render_version_lines().join("\n"));
        }
        return;
    }
    eprintln!("oc-rsyncd is not yet implemented. Use `oc-rsync --daemon` instead.");
    std::process::exit(1);
}
