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
}
