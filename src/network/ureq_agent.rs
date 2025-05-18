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


use std::collections::HashMap;

use thiserror::Error;

use serde::de::DeserializeOwned;

pub use super::ureq_agent::*;

pub type AgentError = ureq::Error;

#[derive(Error, Debug)]

pub enum Error {
    #[error("Agent error")]
    AgentError(Box<AgentError>),
    #[error("IO Error")]
    IoError(std::io::Error),
    #[error("Unauthorized")]
    Unauthorized,
}

pub fn get<T>(url: &str) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    let res = match get_agent().get(url).call() {
        Ok(o) => Ok(o),
        Err(e) => Err(Error::AgentError(Box::new(e))),
    }?;
    match res.into_json::<T>() {
        Ok(o) => Ok(o),
        Err(e) => Err(Error::IoError(e)),
    }
}

pub fn get_bytes(url: &str) -> Result<Vec<u8>, Error> {
    Ok(get_str(url)?.into_bytes())
}
pub fn get_str(url: &str) -> Result<String, Error> {
    let res = match get_agent().get(url).call() {
        Ok(o) => Ok(o),
        Err(e) => Err(Error::AgentError(Box::new(e))),
    }?;
    match res.into_string() {
        Ok(o) => Ok(o),
        Err(e) => Err(Error::IoError(e)),
    }
}
pub fn post<T>(
    url: &str,
    data: impl serde::Serialize,
    headers: &HashMap<&str, &str>,
) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    let mut req = get_agent().post(url);

    for (key, val) in headers {
        req = req.set(key, val);
    }

    let res = match req.send_json(data) {
        Ok(o) => Ok(o),
        Err(e) => match e {
            ureq::Error::Status(code, _) => {
                if code == 401 {
                    Err(Error::Unauthorized)
                } else {
                    Err(Error::AgentError(Box::new(e)))
                }
            }
            ureq::Error::Transport(_) => Err(Error::AgentError(Box::new(e))),
        },
    }?;

    match res.into_json::<T>() {
        Ok(o) => Ok(o),
        Err(e) => Err(Error::IoError(e)),
    }
}

pub fn get_opt<T>(url: &str) -> Result<Option<T>, Error>
where
    T: DeserializeOwned,
{
    match get::<T>(url) {
        Ok(o) => Ok(Some(o)),
        Err(e) => Err(match &e {
            Error::AgentError(u) => match u.as_ref() {
                ureq::Error::Status(code, _) => {
                    if *code == 404u16 {
                        return Ok(None);
                    }
                    return Err(e);
                }
                _ => e,
            },
            _ => e,
        }),
    }
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
