use std::{
    cell::Cell,
    default,
    io::Stdout,
    sync::{atomic::AtomicUsize, Arc, RwLock},
};

use gix::{Count, NestedProgress, Progress};
use log::info;
use pbr::ProgressBar;

pub struct PbrProgress {
    pbr: std::sync::RwLock<ProgressBar<Stdout>>,
}

impl Default for PbrProgress {
    fn default() -> Self {
        Self {
            pbr: RwLock::new(ProgressBar::new(100)),
        }
    }
}

impl Count for PbrProgress {
    fn set(&self, step: prodash::progress::Step) {
        self.pbr.write().unwrap().set(step.try_into().unwrap());
    }

    fn step(&self) -> prodash::progress::Step {
        self.pbr.write().unwrap().add(0).try_into().unwrap()
    }

    fn inc_by(&self, step: prodash::progress::Step) {
        self.pbr.write().unwrap().add(step.try_into().unwrap());
    }

    /// I don't have any other alternatives here
    fn counter(&self) -> gix::progress::StepShared {
        Arc::new(AtomicUsize::new(self.step()))
    }
}

impl Progress for PbrProgress {
    fn init(&mut self, max: Option<prodash::progress::Step>, unit: Option<prodash::Unit>) {
        if let Some(max) = max {
            self.pbr.write().unwrap().total = max.try_into().unwrap();
        }
    }

    fn set_name(&mut self, name: String) {}

    fn name(&self) -> Option<String> {
        None
    }

    fn id(&self) -> gix::progress::Id {
        Default::default()
    }

    fn message(&self, level: gix::progress::MessageLevel, message: String) {
        match level {
            gix::progress::MessageLevel::Info => info!("Progressing: {}", message),
            gix::progress::MessageLevel::Failure => info!("Failure: {}", message),
            gix::progress::MessageLevel::Success => {
                self.pbr.write().unwrap().finish_println(&message);
                println!();
            }
        }
    }
}

// I genuinely don't care
impl NestedProgress for PbrProgress {
    type SubProgress =  PbrProgress;

    fn add_child(&mut self, name: impl Into<String>) -> Self::SubProgress {
        PbrProgress::default()
    }

    fn add_child_with_id(&mut self, name: impl Into<String>, id: gix::progress::Id) -> Self::SubProgress {
        PbrProgress::default()

    }
}