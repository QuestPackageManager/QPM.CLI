use std::{env, io::Write, sync, time::Duration};

use color_eyre::{
    eyre::{bail, Context},
    Result,
};
use pbr::ProgressBar;
use reqwest::StatusCode;

use crate::models::config::get_combine_config;

static AGENT: sync::OnceLock<reqwest::blocking::Client> = sync::OnceLock::new();
static ASYNC_AGENT: sync::OnceLock<reqwest::Client> = sync::OnceLock::new();
static RUNTIME: sync::OnceLock<tokio::runtime::Runtime> = sync::OnceLock::new();

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

pub fn get_async_agent() -> &'static reqwest::Client {
    let timeout = get_combine_config().timeout.unwrap_or(5000);

    ASYNC_AGENT.get_or_init(|| {
        reqwest::ClientBuilder::new()
            .connect_timeout(Duration::from_millis(timeout.into()))
            .read_timeout(Duration::from_millis(timeout.into()))
            .tcp_keepalive(Duration::from_secs(5))
            .tcp_nodelay(false)
            .https_only(true)
            .user_agent(format!("questpackagemanager-rust2/{}", env!("CARGO_PKG_VERSION")).as_str())
            .build()
            .expect("Client agent was not buildable")
    })
}

pub fn get_runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Unable to build tokio runtime")
    })
}

pub fn download_file<F>(url: &str, buffer: &mut impl Write, mut callback: F) -> Result<usize>
where
    F: FnMut(usize, usize),
{
    let mut request = get_async_agent().get(url).build()?;

    request.timeout_mut().take(); // Set to none

    get_runtime().block_on(async {
        let mut response = get_async_agent()
            .execute(request)
            .await
            .with_context(|| format!("Unable to download file {url}"))?
            .error_for_status()?;
        if response.status() == StatusCode::NOT_FOUND {
            bail!("Not found!");
        }

        // TODO: Fix
        let expected_amount = response.content_length().unwrap_or(0) as usize;
        let mut written: usize = 0;

        while let Some(chunk) = response.chunk().await? {
            written += chunk.len();
            buffer.write_all(&chunk)?;
            callback(written, expected_amount);
        }

        Ok(expected_amount)
    })
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
