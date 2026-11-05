fn main() {
    slint_build::compile("ui/app.slint").expect("failed to compile Slint UI");

    // Some Android 15 devices use 16 KiB memory pages. Align the native
    // library's ELF load segments accordingly so it stays loadable there.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        println!("cargo:rustc-link-arg=-Wl,-z,max-page-size=16384");
    }
}
