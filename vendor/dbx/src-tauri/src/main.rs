// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if linop_db_lib::run_backup_worker_if_requested() {
        return;
    }
    linop_db_lib::run();
}
