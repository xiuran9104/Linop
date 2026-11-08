#[tokio::main]
async fn main() {
    if let Err(error) = linop_db_tdengine_driver::run().await {
        eprintln!("TDengine driver failed: {error:#}");
        std::process::exit(1);
    }
}
