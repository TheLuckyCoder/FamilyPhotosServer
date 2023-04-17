use crate::file_scan::data_scan::DataScan;
use crate::utils::AppState;

mod data_scan;
mod timestamp;

pub async fn scan_new_files(app_state: AppState) {
    DataScan::run(app_state).await;
}
