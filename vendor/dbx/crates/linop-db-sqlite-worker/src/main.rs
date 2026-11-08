fn main() {
    if let Err(error) = linop_db_sqlite_worker::runtime::run_stdio() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
