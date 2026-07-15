use std::{
    env,
    io::{ErrorKind, Read, Write},
    sync,
    thread::sleep,
    time::Duration,
};

use bytes::{BufMut, BytesMut};
use color_eyre::{Result, eyre::Context};

static AGENT: sync::OnceLock<reqwest::blocking::Client> = sync::OnceLock::new();

const DEFAULT_TIMEOUT_MS: u32 = 5000;

fn build_agent(timeout_ms: u32) -> reqwest::blocking::Client {
    reqwest::blocking::ClientBuilder::new()
        .connect_timeout(Duration::from_millis(timeout_ms.into()))
        .tcp_keepalive(Duration::from_secs(5))
        .tcp_nodelay(false)
        .https_only(true)
        .user_agent(format!("questpackagemanager-rs3/{}", env!("CARGO_PKG_VERSION")).as_str())
        .build()
        .expect("Client agent was not buildable")
}

/// Initializes the shared HTTP agent with an explicit connect timeout. Only the first call
/// across `init_agent`/`get_agent` has any effect - once the underlying client is built it
/// can't be reconfigured. Call this once at startup with the user's configured timeout before
/// any network-touching command runs; `get_agent()` alone falls back to `DEFAULT_TIMEOUT_MS`.
pub fn init_agent(timeout_ms: u32) -> &'static reqwest::blocking::Client {
    AGENT.get_or_init(|| build_agent(timeout_ms))
}

pub fn get_agent() -> &'static reqwest::blocking::Client {
    AGENT.get_or_init(|| build_agent(DEFAULT_TIMEOUT_MS))
}

pub fn download_file<F>(url: &str, buffer: &mut impl Write, mut callback: F) -> Result<usize>
where
    F: FnMut(usize, usize),
{
    let mut request = get_agent().get(url).build()?;

    request.timeout_mut().take(); // Set to none

    let mut response = get_agent()
        .execute(request)
        .with_context(|| format!("Unable to download file {url}"))?
        .error_for_status()?;

    let expected_amount = response.content_length().unwrap_or(0) as usize;

    if expected_amount == 0 {
        println!("Unable to determine content length for download from {url}");
    }

    let mut written: usize = 0;

    let mut temp_buf = vec![0u8; 1024];

    loop {
        let read = response.read(&mut temp_buf);

        match read {
            // EOF
            Ok(0) => break,

            Ok(amount) => {
                written += amount;
                buffer.write_all(&temp_buf[0..amount])?;
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
    use pbr::ProgressBar;

    let mut progress_bar = ProgressBar::new(0);
    progress_bar.set_units(pbr::Units::Bytes);

    if env::var("CI") == Ok("true".to_string()) {
        progress_bar.set_max_refresh_rate(Some(Duration::from_millis(500)));
    }

    let result = download_file(url, buffer, |current, expected| {
        progress_bar.total = expected as u64;
        progress_bar.set(current as u64);

        callback(current, expected)
    });

    progress_bar.finish_println("Finished download!");
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
