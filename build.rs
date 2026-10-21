fn main() {
    println!("cargo:rerun-if-changed=assets/windows/spotiamp.ico");
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // Build-time resource only: no image decoder or extra runtime process.
        winresource::WindowsResource::new()
            .set_icon("assets/windows/spotiamp.ico")
            .set("ProductName", "Spotiamp")
            .set("FileDescription", "Spotiamp — lecteur Spotify")
            .set("OriginalFilename", "spotiamp.exe")
            .compile_for("spotiamp")
            .expect("compile Windows icon and version resource");
    }
}
