fn main() {
    if let Err(err) = canlink_tscan::daemon::server::run_server() {
        eprintln!("daemon error: {err}");
        std::process::exit(1);
    }
}
