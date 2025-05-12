#![feature(try_find)]
#![feature(iterator_try_collect)]
#![feature(let_chains)]
#![feature(exit_status_error)]
#![feature(if_let_guard)]
#![feature(path_add_extension)]

#[cfg(feature = "cli")]
pub mod commands;

pub mod models;
pub mod network;
pub mod repository;
pub mod resolver;
pub mod terminal;
pub mod utils;

pub use qpm_package as package;
pub use qpm_qmod as qmod;
