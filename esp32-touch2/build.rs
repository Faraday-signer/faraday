fn main() {
    embuild::espidf::sysenv::output();

    // Patch ESP-IDF Kconfig to disable WiFi for air-gapped builds.
    // The `default y if SOC_WIFI_SUPPORTED` in esp_wifi/Kconfig
    // cannot be overridden by sdkconfig.defaults because kconfgen
    // treats it as a non-promptable default.
    if let Ok(idf_path) = std::env::var("IDF_PATH") {
        let kconfig = std::path::PathBuf::from(&idf_path)
            .join("components/esp_wifi/Kconfig");
        if kconfig.exists() {
            let content = std::fs::read_to_string(&kconfig).unwrap_or_default();
            let patched = content.replace(
                "default y if SOC_WIFI_SUPPORTED",
                "# Faraday: overridden to n (air-gapped device)\n        default n",
            );
            if patched != content {
                std::fs::write(&kconfig, patched).ok();
                println!("cargo:warning=Patched {kconfig:?} to disable WiFi");
            }
        }
    }
}
