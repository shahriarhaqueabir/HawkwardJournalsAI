use std::fs;
use std::path::{Path, PathBuf};

pub fn resolve_data_dir() -> PathBuf {
    // Portable mode: "data" folder exists in the same directory as the executable
    let exe_path = std::env::current_exe().unwrap_or_default();
    let exe_dir = exe_path.parent().unwrap_or_else(|| Path::new("."));
    let portable_data = exe_dir.join("data");

    if portable_data.exists() && portable_data.is_dir() {
        portable_data
    } else {
        // MSI/Installed mode: use %APPDATA%/PersonalLifeOS
        let app_data = dirs::data_dir()
            .expect("Could not resolve AppData directory")
            .join("HawkwardJournals");

        if !app_data.exists() {
            fs::create_dir_all(&app_data).expect("Failed to create AppData directory");
        }
        app_data
    }
}
