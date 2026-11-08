fn main() {
    if let Err(error) = linop_db_plugin_cli::run_cli(std::env::args().skip(1)) {
        linop_db_plugin_cli::print_error(&error);
        std::process::exit(1);
    }
}
