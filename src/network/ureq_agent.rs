use std::{
    io::{BufReader, Read},
    sync,
    time::Duration,
};

use color_eyre::{eyre::Context, Result};
use ureq::{Agent, AgentBuilder};

use crate::models::config::get_combine_config;

static AGENT: sync::OnceLock<ureq::Agent> = sync::OnceLock::new();

pub fn get_agent() -> &'static Agent {
    AGENT.get_or_init(|| {
        AgentBuilder::new()
            .timeout(Duration::from_millis(
                get_combine_config().timeout.unwrap().into(),
            ))
            .https_only(true)
            .user_agent(format!("questpackagemanager-rust2/{}", env!("CARGO_PKG_VERSION")).as_str())
            .build()
    })
}

pub fn download_file<F>(url: &str, _callback: F) -> Result<Vec<u8>>
where
    F: FnMut(usize, usize),
{
    let request = get_agent().get(url).timeout(Duration::MAX);

    let response = request
        .call()
        .with_context(|| format!("Unable to download file {url}"))?;

    let mut bytes: Vec<u8> = Vec::with_capacity(
        response
            .header("Content-Length")
            .unwrap_or("0")
            .parse::<usize>()
            .unwrap_or(0),
    );

    let mut reader = BufReader::new(response.into_reader());

    reader.read_to_end(&mut bytes)?;

    Ok(bytes)

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
