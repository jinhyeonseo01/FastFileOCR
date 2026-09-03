fn main() {
    println!("cargo:rerun-if-env-changed=FASTFILEOCR_GITHUB_REPOSITORY");
    tauri_build::build()
}
