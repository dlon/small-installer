use anyhow::Context;

fn main() -> anyhow::Result<()> {
    let mut res = winres::WindowsResource::new();

    res.set_language(make_lang_id(
        windows_sys::Win32::System::SystemServices::LANG_ENGLISH as u16,
        windows_sys::Win32::System::SystemServices::SUBLANG_ENGLISH_US as u16,
    ));

    println!("cargo:rerun-if-changed=loader.manifest");
    res.set_manifest_file("loader.manifest");

    res.compile().context("Failed to compile resources")
}

// Sourced from winnt.h: https://learn.microsoft.com/en-us/windows/win32/api/winnt/nf-winnt-makelangid
fn make_lang_id(p: u16, s: u16) -> u16 {
    (s << 10) | p
}
