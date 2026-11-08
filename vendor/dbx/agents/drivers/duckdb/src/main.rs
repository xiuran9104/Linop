#[tokio::main]
async fn main() {
    if let Err(error) = linop_db_duckdb_driver::run_stdio_worker().await {
        eprintln!("DuckDB driver failed: {error}");
        std::process::exit(1);
    }
}
