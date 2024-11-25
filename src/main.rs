#![windows_subsystem = "windows"]

use std::{fs::File, io::BufWriter, path::Path, process::Command};

use progress::ProgressHandle;

//mod gpg;
mod progress;
mod verify;

const LATEST_APP_URL: &str = "https://mullvad.net/en/download/app/exe/latest";
const LATEST_SIGNATURE_URL: &str = "https://mullvad.net/download/app/exe/latest/signature";

fn main() -> anyhow::Result<()> {
    let handle = progress::open().unwrap();

    let current_exe = std::env::current_exe().unwrap();
    let target_dir = current_exe.parent().unwrap();
    let latest_exe = target_dir.join("latest.exe");
    let latest_sig = target_dir.join("latest.exe.sig");

    get_to_file(&latest_exe, LATEST_APP_URL, &handle)?;
    get_to_file(&latest_sig, LATEST_SIGNATURE_URL, &handle)?;

    // TODO: display marquee progress and write something during verification?
    // TODO: toctou?
    verify::verify(&latest_exe, &latest_sig)?;

    // TODO: display message box on error

    // Launch actual installer
    Command::new(latest_exe).spawn().unwrap();
    
    Ok(())
}

struct WriterWithProgress<'a> {
    file: BufWriter<File>,
    progress_handle: &'a ProgressHandle,
    written_nbytes: usize,
    /// Actual or estimated total number of bytes
    total_nbytes: usize,
}

impl<'a> std::io::Write for WriterWithProgress<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let nbytes = self.file.write(buf)?;

        self.written_nbytes += nbytes;
        self.progress_handle.set_progress(
            (self.written_nbytes as f32 / self.total_nbytes as f32)
        );

        Ok(nbytes)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

fn get_to_file(
    file: impl AsRef<Path>,
    url: &str,
    progress_handle: &ProgressHandle,
) -> anyhow::Result<()> {
    let file = BufWriter::new(File::create(file)?);
    let mut get_result = reqwest::blocking::get(url)?;
    
    // TODO: handle unknown length. currently guessing 100 MB
    let total_size = get_result.content_length().unwrap_or(100 * 1024 * 1024);

    let mut writer = WriterWithProgress {
        file,
        progress_handle,
        written_nbytes: 0,
        total_nbytes: total_size as usize,
    };

    get_result.copy_to(&mut writer)?;
    Ok(())
}
