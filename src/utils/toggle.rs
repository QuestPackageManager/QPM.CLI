use clap::Subcommand;

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

#[cfg(test)]
mod tests {
    use super::Toggle;

    fn parse(s: &str) -> Toggle {
        s.parse().unwrap()
    }

    #[test]
    fn every_enable_alias_parses_to_enable() {
        for s in ["enable", "e", "true", "t", "1", "ENABLE", "True"] {
            assert!(matches!(parse(s), Toggle::Enable), "{s:?}");
        }
    }

    #[test]
    fn every_disable_alias_parses_to_disable() {
        for s in ["disable", "d", "false", "f", "0", "DISABLE", "False"] {
            assert!(matches!(parse(s), Toggle::Disable), "{s:?}");
        }
    }

    #[test]
    fn anything_else_is_invalid() {
        for s in ["", "yes", "no", "2", "enabled", " true"] {
            assert!(matches!(parse(s), Toggle::Invalid), "{s:?}");
        }
    }

    #[test]
    fn bool_conversions() {
        assert!(matches!(Toggle::from(true), Toggle::Enable));
        assert!(matches!(Toggle::from(false), Toggle::Disable));
        assert!(bool::from(Toggle::Enable));
        assert!(!bool::from(Toggle::Disable));
        assert!(!bool::from(Toggle::Invalid));
    }
}
