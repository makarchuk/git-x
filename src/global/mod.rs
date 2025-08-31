use std::sync::OnceLock;

pub struct Config {
    pub git_debug: bool,
    pub api_debug: bool,
    pub debug: bool,
}

static CONFIG: OnceLock<Config> = OnceLock::new();

pub fn init_config(config: Config) -> Result<(), Config> {
    CONFIG.set(config)
}

pub fn config() -> &'static Config {
    CONFIG.get().expect("Config not initialized")
}

#[macro_export]
macro_rules! log_git {
    ($($arg:tt)*) => {
        if $crate::global::config().git_debug {
            println!($($arg)*);
        }
    };
}

#[macro_export]
macro_rules! log_api {
    ($($arg:tt)*) => {
        if $crate::global::config().api_debug {
            println!($($arg)*);
        }
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        if $crate::global::config().debug {
            println!($($arg)*);
        }
    };
}
