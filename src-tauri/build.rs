fn main() {
    println!("cargo:rerun-if-changed=.pake/pake.json");
    println!("cargo:rerun-if-changed=.pake/tauri.conf.json");
    // Kimi web client builds flip app::kimi_web::ENABLED via option_env!.
    println!("cargo:rerun-if-env-changed=PAKE_KIMI_WEB");
    tauri_build::build()
}
