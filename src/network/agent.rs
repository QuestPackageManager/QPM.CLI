use std::{sync, time::Duration};

use color_eyre::{eyre::Context, Result};
use itertools::Itertools;
use std::io::Read;

use crate::models::config::get_combine_config;

static AGENT: sync::OnceLock<ureq::Agent> = sync::OnceLock::new();

pub fn get_agent() -> &'static ureq::Agent {
    let timeout = get_combine_config().timeout.unwrap_or(5000);

    AGENT.get_or_init(|| {
        ureq::AgentBuilder::new()
            // .timeout_read(Duration::from_millis(timeout.into()))
            // .timeout_connect(Duration::from_millis(timeout.into()))
            // .timeout_write(Duration::from_millis(timeout.into()))
            .user_agent(format!("questpackagemanager-rust2/{}", env!("CARGO_PKG_VERSION")).as_str())
            .no_delay(false)
            .https_only(true)
            .build()
    })
}

pub fn download_file<F>(url: &str, _callback: F) -> Result<Vec<u8>>
where
    F: FnMut(usize, usize),
{
    let request = get_agent().get(url).timeout(Duration::MAX);

    // non-200 status codes are raised as errors
    let response = request
        .call()
        .with_context(|| format!("Unable to download file {url}"))?;
    // .error_for_status()?;
    // if response.status() == ureq::OrAnyStatus::NOT_FOUND {
    //     bail!("Not found!");
    // }

    let reader = response.into_reader();

    Ok(reader.bytes().try_collect()?)

    // TODO: Fix
    // let mut bytes = Vec::with_capacity(response.content_length().unwrap_or(0) as usize);
    // let mut read_bytes = vec![0u8; 4 * 1024];

    // loop {
    //     let read = response.read(&mut read_bytes)?;
    //     bytes.append(&mut read_bytes);

    //     callback(bytes.len(), bytes.capacity());
    //     if read == 0 {
    //         println!("Done!");
    //         break;
    //     }
    // }

    // Ok(bytes)
}

#[inline(always)]
pub fn download_file_report<F>(url: &str, mut callback: F) -> Result<Vec<u8>>
where
    F: FnMut(usize, usize),
{
    // let mut progress_bar = ProgressBar::new(1000);

    // progress_bar.finish_println("");
    download_file(url, |current, expected| {
        // progress_bar.set((current / expected) as u64 * 1000);

        callback(current, expected)
    })
}
