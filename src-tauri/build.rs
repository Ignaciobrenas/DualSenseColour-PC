fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu")
    {
        for lib in [
            "dwmapi",
            "gdi32",
            "comctl32",
            "imm32",
            "oleaut32",
            "propsys",
            "rpcrt4",
            "shlwapi",
            "uxtheme",
            "hid",
            "setupapi",
            "winmm",
            "version",
            "cfgmgr32",
            "runtimeobject",
        ] {
            println!("cargo:rustc-link-lib={lib}");
        }
    }

    tauri_build::build();

    // tauri_build only attaches libresource.a (which embeds the ComCtl32 v6 SxS manifest
    // needed for TaskDialogIndirect) to `bin` targets. Attach it generally so unit test
    // harnesses (`cargo test`) also resolve ComCtl32 v6 instead of hitting 0xc0000139.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        if let Ok(out_dir) = std::env::var("OUT_DIR") {
            let resource_lib = std::path::Path::new(&out_dir).join("libresource.a");
            if resource_lib.exists() {
                println!("cargo:rustc-link-arg={}", resource_lib.display());
            }
        }
    }
}
