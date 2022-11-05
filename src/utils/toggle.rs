use clap::{Subcommand};

#[derive(Subcommand, Debug, Clone)]

pub enum Toggle {
    // Enable this thing
    Enable,
    // Disable this thing
    Disable,
    // Anything else
    Invalid,
}

impl std::str::FromStr for Toggle {
    type Err = std::string::ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            // values that should return "true" more or less
            "enable" => Toggle::Enable,
            "e" => Toggle::Enable,
            "true" => Toggle::Enable,
            "t" => Toggle::Enable,
            "1" => Toggle::Enable,
            // values that should return "false" more or less
            "disable" => Toggle::Disable,
            "d" => Toggle::Disable,
            "false" => Toggle::Disable,
            "f" => Toggle::Disable,
            "0" => Toggle::Disable,
            // anything else
            _ => Toggle::Invalid,
        })
    }
}

impl From<bool> for Toggle {
    fn from(b: bool) -> Self {
        match b {
            true => Toggle::Enable,
            false => Toggle::Disable,
        }
    }
}

impl From<Toggle> for bool {
    fn from(t: Toggle) -> Self {
        match t {
            Toggle::Enable => true,
            Toggle::Disable => false,
            Toggle::Invalid => false,
        }
    }
}
