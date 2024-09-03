use std::{
    env,
    io::{ErrorKind, Read, Write},
    sync,
    thread::sleep,
    time::Duration,
};

use color_eyre::{
    eyre::{bail, ensure, eyre, Context},
    Result,
};
use pbr::ProgressBar;
use reqwest::StatusCode;

use crate::models::config::get_combine_config;

static AGENT: sync::OnceLock<reqwest::blocking::Client> = sync::OnceLock::new();

pub fn get_agent() -> &'static reqwest::blocking::Client {
    let timeout = get_combine_config().timeout.unwrap_or(5000);

    AGENT.get_or_init(|| {
        reqwest::blocking::ClientBuilder::new()
            .connect_timeout(Duration::from_millis(timeout.into()))
            .tcp_keepalive(Duration::from_secs(5))
            .tcp_nodelay(false)
            .https_only(true)
            .user_agent(format!("questpackagemanager-rust2/{}", env!("CARGO_PKG_VERSION")).as_str())
            .build()
            .expect("Client agent was not buildable")
    })
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
                    .with_context(|| format!("Failed to continue reading bytes from {url}"))
            }
        }
    }

    ensure!(
        written == expected_amount,
        "Read: 0x{written:x} Expected: 0x{expected_amount:x}"
    );

    Ok(expected_amount)
}

#[inline(always)]
pub fn download_file_report<F>(url: &str, buffer: &mut impl Write, mut callback: F) -> Result<usize>
where
    F: FnMut(usize, usize),
{
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
