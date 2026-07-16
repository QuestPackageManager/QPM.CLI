use std::{
    env,
    io::{ErrorKind, Read, Write},
    sync,
    thread::sleep,
    time::Duration,
};

use bytes::{BufMut, BytesMut};
use color_eyre::{Result, eyre::Context};

static AGENT: sync::OnceLock<ureq::Agent> = sync::OnceLock::new();

const DEFAULT_TIMEOUT_MS: u32 = 5000;

fn build_agent(timeout_ms: u32) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_millis(timeout_ms.into())))
        .no_delay(false)
        .https_only(true)
        .user_agent(format!("questpackagemanager-rs3/{}", env!("CARGO_PKG_VERSION")).as_str())
        .build()
        .new_agent()
}

/// Initializes the shared HTTP agent with an explicit connect timeout. Only the first call
/// across `init_agent`/`get_agent` has any effect - once the underlying client is built it
/// can't be reconfigured. Call this once at startup with the user's configured timeout before
/// any network-touching command runs; `get_agent()` alone falls back to `DEFAULT_TIMEOUT_MS`.
pub fn init_agent(timeout_ms: u32) -> &'static ureq::Agent {
    AGENT.get_or_init(|| build_agent(timeout_ms))
}

pub fn get_agent() -> &'static ureq::Agent {
    AGENT.get_or_init(|| build_agent(DEFAULT_TIMEOUT_MS))
}
pub fn download_file<F>(url: &str, buffer: &mut impl Write, mut callback: F) -> Result<usize>
where
    F: FnMut(usize, usize),
{
    // Perform the request with ureq
    let response = get_agent()
        .get(url)
        .call()
        .with_context(|| format!("Unable to download file {url}"))?;
    let mut body = response.into_body();

    // Read content-length header if present
    let expected_amount = body.content_length().map(|s| s as usize).unwrap_or(0);

    let mut reader = body.as_reader();

    if expected_amount == 0 {
        println!("Unable to determine content length for download from {url}");
    }

    let mut written: usize = 0;
    let mut temp_buf = vec![0u8; 1024];

    loop {
        match reader.read(&mut temp_buf) {
            // EOF
            Ok(0) => break,

            Ok(amount) => {
                written += amount;
                buffer.write_all(&temp_buf[..amount])?;
                callback(written, expected_amount);
            }

            Err(e) if e.kind() == ErrorKind::Interrupted => {
                sleep(Duration::from_millis(1));
            }

            Err(e) => {
                return Err(e)
                    .with_context(|| format!("Failed to continue reading bytes from {url}"));
            }
        }
    }

    if written != expected_amount {
        println!("Downloaded size does not match expected size!");
        println!("Read: 0x{written:x} Expected: 0x{expected_amount:x}");
    }

    Ok(expected_amount)
}

#[inline(always)]
#[cfg(not(feature = "cli"))]
pub fn download_file_report<F>(url: &str, buffer: &mut impl Write, callback: F) -> Result<usize>
where
    F: FnMut(usize, usize),
{
    download_file(url, buffer, callback)
}

#[inline(always)]
#[cfg(feature = "cli")]
pub fn download_file_report<F>(url: &str, buffer: &mut impl Write, mut callback: F) -> Result<usize>
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

    let result = download_file(url, buffer, |current, expected| {
        progress_bar.set_length(expected as u64);
        progress_bar.set_position(current as u64);

        callback(current, expected)
    });

    progress_bar.finish_with_message("Finished download!");
    println!();

    result
}

/// Downloads a URL fully into memory, reporting progress along the way.
pub fn download_bytes(url: &str) -> Result<BytesMut> {
    let mut bytes = BytesMut::new().writer();
    download_file_report(url, &mut bytes, |_, _| {})
        .with_context(|| format!("Failed while downloading {url}"))?;

    Ok(bytes.into_inner())
}
