pub mod database;
pub mod error;
pub mod models;
pub mod services;
pub mod ui;
pub mod updates;
pub mod utils;

pub use database::Database;
pub use error::{AppError, AppResult};

/// Android entry point used by `android-activity`'s NativeActivity.
///
/// This symbol lives in the `cdylib` (`libfat.so`) packaged by cargo-apk.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: slint::android::AndroidApp) {
    // Resolve the app-private directory before the AndroidApp handle is moved
    // into the backend. Slint's desktop data-directory lookup is not reliable
    // on Android, so pass the path explicitly.
    let data_dir = app
        .internal_data_path()
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    if let Err(error) = slint::android::init(app) {
        eprintln!("Failed to initialize the Slint Android backend: {error}");
        return;
    }
    if let Err(error) = ui::run_with_data_dir(data_dir) {
        eprintln!("FAT UI error: {error}");
    }
}
