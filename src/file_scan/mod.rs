use crate::file_scan::data_scan::DataScan;
use crate::http::AppState;

mod data_scan;
mod timestamp;

pub fn scan_new_files(app_state: AppState) {
    DataScan::run(app_state);
}
