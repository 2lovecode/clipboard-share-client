fn main() {
    // mingw/GNU 链接 Tauri cdylib 时会导出全部符号，超过 PE 65535 ordinal 上限
    // （error: export ordinal too large）。限制导出，行为接近 MSVC。
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu")
    {
        println!("cargo::rustc-link-arg=-Wl,--exclude-libs=ALL,--exclude-all-symbols");
    }
    tauri_build::build();
}
