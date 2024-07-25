//! [![crates.io](https://img.shields.io/crates/v/sudo?logo=rust)](https://crates.io/crates/sudo/)
//! [![docs.rs](https://docs.rs/sudo/badge.svg)](https://docs.rs/sudo)
//!
//! Detect if you are running as root, restart self with `sudo` if needed or
//! setup uid zero when running with the SUID flag set.
//!
//! ## Requirements
//!
//! * The `sudo` program is required to be installed and setup correctly on the
//!   target system.
//! * Linux or Mac OS X tested
//!     * It should work on *BSD. However, you can also create an Escalate
//!       builder with `doas` as the wrapper should you prefer that.
#![allow(clippy::bool_comparison)]
#![cfg(unix)]

use std::error::Error;
use std::process::Command;

/// Cross platform representation of the state the current program running
#[derive(Debug, PartialEq)]
pub enum RunningAs {
    /// Root (Linux/Mac OS/Unix) or Administrator (Windows)
    Root,
    /// Running as a normal user
    User,
    /// Started from SUID, a call to `sudo2::escalate_if_needed` or
    /// `sudo2::with_env` is required to claim the root privileges at runtime.
    /// This does not restart the process.
    Suid,
}
use RunningAs::*;

/// Check getuid() and geteuid() to learn about the configuration this program
/// is running under
fn check() -> RunningAs {
    let uid = unsafe { libc::getuid() };
    let euid = unsafe { libc::geteuid() };

    match (uid, euid) {
        (0, 0) => Root,
        (_, 0) => Suid,
        (_, _) => User,
    }
    //if uid == 0 { Root } else { User }
}

/// Returns `true` if binary is already running as root.
pub fn running_as_root() -> bool {
    check() == RunningAs::Root
}

/// Returns `true` if binary is already running as suid.
pub fn running_as_suid() -> bool {
    check() == RunningAs::Suid
}

pub struct Escalate {
    wrapper: String,
}

impl Default for Escalate {
    fn default() -> Self {
        Escalate {
            wrapper: "sudo".to_string(),
        }
    }
}

impl Escalate {
    fn builder() -> Self {
        Default::default()
    }

    fn wrapper(&mut self, wrapper: &str) -> &mut Self {
        self.wrapper = wrapper.to_string();
        self
    }

    /// Escalate privileges while maintaining RUST_BACKTRACE and selected
    /// environment variables (or none).
    ///
    /// Activates SUID privileges when available.
    fn with_env(&self, prefixes: &[&str]) -> Result<RunningAs, Box<dyn Error>> {
        self.collect_envs(prefixes, false)
    }

    /// Escalate privileges while maintaining RUST_BACKTRACE and selected
    /// environment variables (or none) as wildcard. Use can use `*` to select
    /// all environment variables (mimics `sudo -E`)
    ///
    /// Activates SUID privileges when available.
    fn with_env_wildcards(&self, wildcards: &[&str]) -> Result<RunningAs, Box<dyn Error>> {
        self.collect_envs(wildcards, true)
    }

    fn collect_envs(&self, patterns: &[&str], is_glob: bool) -> Result<RunningAs, Box<dyn Error>> {
        let current = check();
        tracing::trace!("Running as {:?}", current);
        match current {
            Root => {
                tracing::trace!("already running as Root");
                return Ok(current);
            }
            Suid => {
                tracing::trace!("setuid(0)");
                unsafe {
                    libc::setuid(0);
                }
                return Ok(current);
            }
            User => {
                tracing::debug!("Escalating privileges");
            }
        }

        let mut args: Vec<_> = std::env::args().collect();
        if let Some(absolute_path) = std::env::current_exe()
            .ok()
            .and_then(|p| p.to_str().map(|p| p.to_string()))
        {
            args[0] = absolute_path;
        }
        let mut command: Command = Command::new(&self.wrapper);

        // Always propagate RUST_BACKTRACE
        if let Ok(trace) = std::env::var("RUST_BACKTRACE") {
            let value = match &*trace.to_lowercase() {
                "" => None,
                "1" | "true" => Some("1"),
                "full" => Some("full"),
                invalid => {
                    tracing::warn!(
                        "RUST_BACKTRACE has invalid value {:?} -> defaulting to \"full\"",
                        invalid
                    );
                    Some("full")
                }
            };
            if let Some(value) = value {
                tracing::trace!("relaying RUST_BACKTRACE={}", value);
                // command.arg(format!("RUST_BACKTRACE={}", value));
                command.env("RUST_BACKTRACE", value);
            }
        }

        if !patterns.is_empty() {
            // Only add env for pkexec if we're passing any additional env vars
            if self.wrapper == "pkexec" {
                tracing::trace!(
                    "Prefixing `env` to pkexec command to pass additional environment variables! \
                     This may break pkexec system policies."
                );
                command.arg("env");
            }

            for (name, value) in std::env::vars().filter(|(name, _)| name != "RUST_BACKTRACE") {
                if patterns.iter().any(|pattern| {
                    if is_glob {
                        wildmatch::WildMatch::new(pattern).matches(&name)
                    } else {
                        name.starts_with(pattern)
                    }
                }) {
                    tracing::trace!("propagating {}={}", name, value);
                    if self.wrapper == "pkexec" {
                        command.arg(format!("{}={}", name, value));
                    }
                    command.env(name, value);
                }
            }
        }

        let mut child = command.args(args).spawn().expect("failed to execute child");

        let ecode = child.wait().expect("failed to wait on child");

        if ecode.success() == false {
            std::process::exit(ecode.code().unwrap_or(1));
        } else {
            std::process::exit(0);
        }
    }

    /// Restart your program with root privileges if the user is not privileged
    /// enough.
    ///
    /// Activates SUID privileges when available
    pub fn escalate_if_needed(&self) -> Result<RunningAs, Box<dyn Error>> {
        self.with_env(&[])
    }
}

/// Alias for Escalate::builder() to quickly create a new sudo Escalate builder
pub fn builder() -> Escalate {
    Escalate::builder()
}

/// Restart your program with sudo if the user is not privileged enough.
///
/// Activates SUID privileges when available
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// #   if sudo2::running_as_root() {
/// sudo2::escalate_if_needed()?;
/// # // the following gets only executed in privileged mode
/// #   } else {
/// #     eprintln!("not actually testing");
/// #   }
/// #   Ok(())
/// # }
/// ```
#[inline]
pub fn escalate_if_needed() -> Result<RunningAs, Box<dyn Error>> {
    with_env(&[])
}

/// Restart your program with sudo and if the user is not privileged enough.
/// Inherit all environment variables.
///
/// Activates SUID privileges when available
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// #   if sudo2::running_as_root() {
/// sudo2::escalate_with_env()?;
/// # // the following gets only executed in privileged mode
/// #   } else {
/// #     eprintln!("not actually testing");
/// #   }
/// #   Ok(())
/// # }
/// ```
#[inline]
pub fn escalate_with_env() -> Result<RunningAs, Box<dyn Error>> {
    with_env_wildcards(&["*"])
}

/// Similar to escalate_if_needed, but with pkexec as the wrapper
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// #   if sudo2::running_as_root() {
/// sudo2::pkexec()?;
/// # // the following gets only executed in privileged mode
/// #   } else {
/// #     eprintln!("not actually testing");
/// #   }
/// #   Ok(())
/// # }
/// ```
#[inline]
pub fn pkexec() -> Result<RunningAs, Box<dyn Error>> {
    builder().wrapper("pkexec").escalate_if_needed()
}

/// Similar to escalate_if_needed, but with doas as the wrapper
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// #   if sudo2::running_as_root() {
/// sudo2::doas()?;
/// # // the following gets only executed in privileged mode
/// #   } else {
/// #     eprintln!("not actually testing");
/// #   }
/// #   Ok(())
/// # }
/// ```
#[inline]
pub fn doas() -> Result<RunningAs, Box<dyn Error>> {
    builder().wrapper("doas").escalate_if_needed()
}

/// Escalate privileges while maintaining RUST_BACKTRACE and selected
/// environment variables (or none).
///
/// Activates SUID privileges when available.
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// #   if sudo2::running_as_root() {
/// sudo2::with_env(&["CARGO_", "MY_APP_"])?;
/// # // the following gets only executed in privileged mode
/// #   } else {
/// #     eprintln!("not actually testing");
/// #   }
/// #   Ok(())
/// # }
/// ```
pub fn with_env(prefixes: &[&str]) -> Result<RunningAs, Box<dyn Error>> {
    Escalate::default().with_env(prefixes)
}

/// Escalate privileges while maintaining RUST_BACKTRACE and selected
/// environment variables that matches given wildcard (or none).
///
/// To select all env variables, use `*`. Note that it may be insecure. Use it
/// with care.
///
/// Activates SUID privileges when available.
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// #   if sudo2::running_as_root() {
/// sudo2::with_env_wildcards(&["CARGO_*", "MY_APP_*"])?;
/// # // the following gets only executed in privileged mode
/// #   } else {
/// #     eprintln!("not actually testing");
/// #   }
/// #   Ok(())
/// # }
/// ```
pub fn with_env_wildcards(wildcards: &[&str]) -> Result<RunningAs, Box<dyn Error>> {
    Escalate::default().with_env_wildcards(wildcards)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_test::traced_test;

    #[test]
    #[traced_test]
    fn it_works() {
        let c = check();
        println!("{:?}", c);
    }

    #[test]
    #[traced_test]
    fn sudo_with_env() {
        std::env::set_var("CARGO_FOO", "1");
        std::env::set_var("CARGO_BAR_BAZ", "1");
        with_env(&["CARGO_"]).unwrap();

        let mut vars = std::env::vars();
        assert!(vars.any(|(k, _v)| k == "CARGO_FOO"));
        assert!(vars.any(|(k, _v)| k == "CARGO_BAR_BAZ"));
        assert!(!vars.any(|(k, _v)| k == "CARGO_FOO_BAR_BAZ"));
    }

    #[test]
    #[traced_test]
    fn sudo_with_env_wildcard() {
        std::env::set_var("CARGO_FOO", "1");
        std::env::set_var("CARGO_BAR_BAZ", "1");
        with_env_wildcards(&["CARGO_*"]).unwrap();

        let mut vars = std::env::vars();
        assert!(vars.any(|(k, _v)| k == "CARGO_FOO"));
        assert!(vars.any(|(k, _v)| k == "CARGO_BAR_BAZ"));
        assert!(!vars.any(|(k, _v)| k == "CARGO_FOO_BAR_BAZ"));
    }
}
