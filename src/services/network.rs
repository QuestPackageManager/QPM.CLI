use std::{
    env,
    fs::File,
    io::{ErrorKind, Read, Seek, Write},
    time::Duration,
};

use color_eyre::{Result, eyre::Context};
use digest_io::HashWriter;
use sha2::{Digest, Sha256};

pub const DEFAULT_TIMEOUT_MS: u32 = 5000;

/// Builds the HTTP client. It's cheap to clone (clones share one connection pool), so build
/// it once per invocation (see `QpmContext`) and pass it down.
pub fn build_agent(timeout_ms: u32) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_millis(timeout_ms.into())))
        .no_delay(false)
        .https_only(true)
        .user_agent(format!("questpackagemanager-rs3/{}", env!("CARGO_PKG_VERSION")).as_str())
        .build()
        .new_agent()
}

/// Size of each read from the response body
const READ_CHUNK: usize = 64 * 1024;

pub fn download_file<F>(
    agent: &ureq::Agent,
    url: &str,
    buffer: &mut impl Write,
    mut callback: F,
) -> Result<usize>
where
    F: FnMut(usize, usize),
{
    let response = agent
        .get(url)
        .call()
        .with_context(|| format!("Unable to download file {url}"))?;
    let mut body = response.into_body();

    // 0 when the server sends no Content-Length
    let expected_amount = body.content_length().map(|s| s as usize).unwrap_or(0);
    if expected_amount == 0 {
        println!("Unable to determine content length for download from {url}");
    }

    let mut reader = body.as_reader();
    let mut written: usize = 0;
    let mut chunk = vec![0u8; READ_CHUNK];

    loop {
        match reader.read(&mut chunk) {
            // EOF
            Ok(0) => break,
            Ok(amount) => {
                buffer.write_all(&chunk[..amount])?;
                written += amount;
                callback(written, expected_amount);
            }
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => {
                return Err(e)
                    .with_context(|| format!("Failed to continue reading bytes from {url}"));
            }
        }
    }

    if expected_amount != 0 && written != expected_amount {
        println!("Downloaded size does not match expected size!");
        println!("Read: 0x{written:x} Expected: 0x{expected_amount:x}");
    }

    Ok(written)
}

#[inline(always)]
#[cfg(not(feature = "cli"))]
pub fn download_file_report<F>(
    agent: &ureq::Agent,
    url: &str,
    buffer: &mut impl Write,
    callback: F,
) -> Result<usize>
where
    F: FnMut(usize, usize),
{
    download_file(agent, url, buffer, callback)
}

#[inline(always)]
#[cfg(feature = "cli")]
pub fn download_file_report<F>(
    agent: &ureq::Agent,
    url: &str,
    buffer: &mut impl Write,
    mut callback: F,
) -> Result<usize>
where
    F: FnMut(usize, usize),
{
    use indicatif::{ProgressBar, ProgressDrawTarget, ProgressState};

    let progress_bar = ProgressBar::no_length().with_style(
        indicatif::ProgressStyle::with_template(
            "[{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})",
        )?
        .with_key(
            "eta",
            |state: &ProgressState, w: &mut dyn std::fmt::Write| {
                write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()
            },
        )
        .progress_chars("=>-"),
    );

    progress_bar.enable_steady_tick(Duration::from_millis(100));

    if env::var("CI") == Ok("true".to_string()) {
        progress_bar.set_draw_target(ProgressDrawTarget::stderr_with_hz(2));
    }

    let result = download_file(agent, url, buffer, |current, expected| {
        progress_bar.set_length(expected as u64);
        progress_bar.set_position(current as u64);

        callback(current, expected)
    });

    progress_bar.finish_with_message("Finished download!");
    println!();

    result
}

fn download_with_progress(
    agent: &ureq::Agent,
    url: &str,
    writer: &mut impl Write,
    show_progress: bool,
) -> Result<usize> {
    if show_progress {
        download_file_report(agent, url, writer, |_, _| {})
    } else {
        download_file(agent, url, writer, |_, _| {})
    }
}

/// Downloads a URL into an anonymous temp file (deleted on drop), rewound to the start.
/// Keeps large downloads (NDKs, QPKGs) out of memory.
pub fn download_to_tempfile(agent: &ureq::Agent, url: &str, show_progress: bool) -> Result<File> {
    let mut file = tempfile::tempfile().context("Failed to create temp file for download")?;
    download_with_progress(agent, url, &mut file, show_progress)
        .with_context(|| format!("Failed while downloading {url}"))?;
    file.rewind().context("Failed to rewind downloaded file")?;
    Ok(file)
}

/// Like [`download_to_tempfile`], also returning the hex sha256 of the content, computed
/// while it streams in.
pub fn download_to_tempfile_hashed(agent: &ureq::Agent, url: &str) -> Result<(File, String)> {
    let file = tempfile::tempfile().context("Failed to create temp file for download")?;
    let mut writer = HashWriter::<Sha256, _>::new(file);
    download_with_progress(agent, url, &mut writer, true)
        .with_context(|| format!("Failed while downloading {url}"))?;

    let (hasher, mut file) = writer.into_parts();
    file.rewind().context("Failed to rewind downloaded file")?;
    Ok((file, hex::encode(hasher.finalize())))
}
