//! [![crates.io](https://img.shields.io/crates/v/sudo2?logo=rust)](https://crates.io/crates/sudo2/)
//! [![docs.rs](https://docs.rs/sudo2/badge.svg)](https://docs.rs/sudo2)
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
#![cfg(unix)]

use std::error::Error;
use std::process::Command;

/// Cross platform representation of the state the current program running
#[derive(Debug, PartialEq, Copy, Clone)]
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

impl Escalate {
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
/// #      sudo2::escalate_if_needed()?;
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
/// #        sudo2::escalate_with_env()?;
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
/// #       sudo2::doas()?;
/// #   } else {
/// #       eprintln!("not actually testing");
/// #   }
/// #   Ok(())
/// # }
/// ```
#[inline]
pub fn doas() -> Result<RunningAs, Box<dyn Error>> {
    builder().wrapper("doas").escalate_if_needed()
}

/// Escalate privileges while maintaining selected environment variables
/// (or none).
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

/// Escalate privileges while maintaining selected environment variables
/// that matches given wildcard (or none).
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
/// #        sudo2::with_env_wildcards(&["CARGO_*", "MY_APP_*"])?;
/// #   } else {
/// #        eprintln!("not actually testing");
/// #   }
/// #   Ok(())
/// # }
/// ```
pub fn with_env_wildcards(wildcards: &[&str]) -> Result<RunningAs, Box<dyn Error>> {
    Escalate::default().with_env_wildcards(wildcards)
}

// ---------------------------------------------------------------------------
// Private implementation
// ---------------------------------------------------------------------------

/// Check getuid() and geteuid() to learn about the configuration this program
/// is running under
fn check() -> RunningAs {
    let uid = unsafe { libc::getuid() };
    let euid = unsafe { libc::geteuid() };

    match (uid, euid) {
        (0, 0) => RunningAs::Root,
        (_, 0) => RunningAs::Suid,
        (_, _) => RunningAs::User,
    }
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

    /// Escalate privileges while maintaining selected environment variables
    /// (or none).
    ///
    /// Activates SUID privileges when available.
    fn with_env(&self, prefixes: &[&str]) -> Result<RunningAs, Box<dyn Error>> {
        self.spawn_and_exit(prefixes, false)
    }

    /// Escalate privileges while maintaining selected environment variables
    /// (or none) as wildcard. Use can use `*` to select all environment
    /// variables (mimics `sudo -E`)
    ///
    /// Activates SUID privileges when available.
    fn with_env_wildcards(&self, wildcards: &[&str]) -> Result<RunningAs, Box<dyn Error>> {
        self.spawn_and_exit(wildcards, true)
    }

    /// Build the `Command` used to re-exec `args` under `self.wrapper`,
    /// carrying along any env vars matching `patterns`.
    ///
    /// Split out of `spawn_and_exit` so the command can be inspected/spawned
    /// directly in tests without going through the process-exiting escalation
    /// path.
    fn build_escalated_command(
        &self,
        args: &[String],
        patterns: &[&str],
        is_glob: bool,
    ) -> Command {
        let mut command: Command = Command::new(&self.wrapper);

        let mut relayed: Vec<(String, String)> = Vec::new();

        if !patterns.is_empty() {
            for (name, value) in std::env::vars() {
                // check if any pattern matches
                if patterns.iter().any(|pattern| {
                    if is_glob {
                        wildmatch::WildMatch::new(pattern).matches(&name)
                    } else {
                        name.starts_with(pattern)
                    }
                }) {
                    relayed.push((name, value));
                }
            }
        }

        if !relayed.is_empty() {
            // `sudo`/`doas` exec the target with a *reset* environment by
            // default (`env_reset` in sudoers): vars set on the wrapper
            // process itself via `Command::env()` never reach the process it
            // execs. Passing them as literal `NAME=value` arguments to `env`
            // survives that reset, since they're argv, not inherited
            // environment. This used to be done only for `pkexec` (whose
            // policy-based exec has the same problem); see issue #3.
            tracing::trace!(
                "Prefixing `env` to {} command to pass additional environment variables!",
                self.wrapper
            );
            // Only the `env`-argv form above is relied on; `Command::env()`
            // would only affect `self.wrapper` itself, which every wrapper
            // we support discards before reaching the target, so it's not
            // set here.
            command.arg("env");
            for (name, value) in &relayed {
                tracing::trace!("propagating {}={}", name, value);
                command.arg(format!("{}={}", name, value));
            }
        }

        command.args(args);
        command
    }

    /// Escalate if needed, then spawn the wrapper, wait for it, and exit this
    /// process with its exit code — this function never returns on the
    /// escalation path (only the already-Root/Suid early returns do).
    fn spawn_and_exit(
        &self,
        patterns: &[&str],
        is_glob: bool,
    ) -> Result<RunningAs, Box<dyn Error>> {
        let current = check();
        tracing::trace!("Running as {:?}", current);
        match current {
            RunningAs::Root => {
                tracing::trace!("already running as Root");
                return Ok(current);
            }
            RunningAs::Suid => {
                tracing::trace!("setuid(0)");
                unsafe {
                    libc::setuid(0);
                }
                return Ok(current);
            }
            RunningAs::User => {
                tracing::debug!("Escalating privileges");
            }
        }

        let mut args: Vec<_> = std::env::args().collect();
        // argv[0] may be relative (or just a bare name found via $PATH); the
        // wrapper needs a path that still resolves once re-exec'd with a
        // reset environment/cwd, so use the resolved absolute path instead.
        if let Some(absolute_path) = std::env::current_exe()
            .ok()
            .and_then(|p| p.to_str().map(|p| p.to_string()))
        {
            args[0] = absolute_path;
        }

        let mut command = self.build_escalated_command(&args, patterns, is_glob);
        let mut child = command.spawn().expect("failed to execute child");
        let ecode = child.wait().expect("failed to wait on child");

        if !ecode.success() {
            std::process::exit(ecode.code().unwrap_or(1));
        } else {
            std::process::exit(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_test::traced_test;

    #[test]
    #[traced_test]
    fn test_it_works() {
        let c = check();
        println!("{:?}", c);
    }

    #[test]
    #[traced_test]
    fn test_sudo_with_env() {
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
    fn test_sudo_with_env_wildcard() {
        std::env::set_var("CARGO_FOO", "1");
        std::env::set_var("CARGO_BAR_BAZ", "1");
        with_env_wildcards(&["CARGO_*"]).unwrap();

        let mut vars = std::env::vars();
        assert!(vars.any(|(k, _v)| k == "CARGO_FOO"));
        assert!(vars.any(|(k, _v)| k == "CARGO_BAR_BAZ"));
        assert!(!vars.any(|(k, _v)| k == "CARGO_FOO_BAR_BAZ"));
    }

    /// Writes a tiny throwaway shell script that resets its environment
    /// before exec'ing its arguments (`exec env -i "$@"`), to stand in for a
    /// privilege-escalation wrapper with `sudo`/`doas`-style `env_reset`
    /// semantics — without needing real root or a password.
    fn write_env_reset_wrapper() -> std::path::PathBuf {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;

        let mut path = std::env::temp_dir();
        path.push(format!(
            "sudo2_test_env_reset_wrapper_{}_{:?}.sh",
            std::process::id(),
            std::thread::current().id()
        ));
        let mut f = std::fs::File::create(&path).expect("failed to create wrapper script");
        writeln!(f, "#!/bin/sh\nexec env -i \"$@\"").expect("failed to write wrapper script");
        drop(f);
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .expect("failed to chmod wrapper script");
        path
    }

    /// Regression test for https://github.com/dilawar/sudo2/issues/3.
    ///
    /// `sudo` execs its target with a *reset* environment by default
    /// (`env_reset` in sudoers): vars set via `Command::env()` on the `sudo`
    /// process itself never reach the process it execs. Before the fix,
    /// `build_escalated_command` relied solely on `Command::env()` for any
    /// non-`pkexec` wrapper, so propagated vars silently never arrived. The
    /// fix passes them as literal `NAME=value` arguments to `env` for every
    /// wrapper, which survives the reset because they're argv, not
    /// inherited environment.
    #[test]
    fn test_issue_3_fix_propagates_vars_through_an_env_reset_wrapper() {
        std::env::set_var("SUDO2_TEST_VAR", "hello");

        let wrapper_path = write_env_reset_wrapper();
        let mut escalate = Escalate::builder();
        escalate.wrapper(wrapper_path.to_str().unwrap());
        let args = vec!["printenv".to_string(), "SUDO2_TEST_VAR".to_string()];
        let mut command = escalate.build_escalated_command(&args, &["SUDO2_TEST_VAR"], false);

        let output = command.output().expect("failed to run env-reset wrapper");
        let _ = std::fs::remove_file(&wrapper_path);

        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "hello");
    }

    /// Negative path: with no patterns requested (the plain
    /// `escalate_if_needed`/`sudo2::builder().escalate_if_needed()` case),
    /// `build_escalated_command` must not add the `env` prefix or any
    /// `NAME=value` arguments at all — the wrapper should just re-exec
    /// `args` as given, with no extra env plumbing.
    #[test]
    #[traced_test]
    fn test_build_escalated_command_without_patterns_adds_no_extra_env() {
        // Present in the environment, but must be ignored: nothing was asked
        // to be propagated.
        std::env::set_var("SUDO2_TEST_UNRELATED", "should-not-appear");

        let escalate = Escalate::builder();
        let args = vec!["true".to_string()];
        let command = escalate.build_escalated_command(&args, &[], false);

        let got_args: Vec<_> = command
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert_eq!(got_args, args, "no env-related args should be injected");
        assert_eq!(
            command.get_envs().count(),
            0,
            "no env vars should be set on the command"
        );
    }

    #[test]
    #[traced_test]
    fn test_pkexec_wrapper_passes_vars_as_explicit_env_args() {
        std::env::set_var("SUDO2_TEST_VAR2", "world");

        let mut escalate = Escalate::builder();
        escalate.wrapper("pkexec");
        let command =
            escalate.build_escalated_command(&["true".to_string()], &["SUDO2_TEST_VAR2"], false);

        let args: Vec<_> = command
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(args.contains(&"env".to_string()));
        assert!(args.contains(&"SUDO2_TEST_VAR2=world".to_string()));
    }

    #[test]
    #[ignore = "invokes real sudo; run manually with `cargo test -- --ignored --nocapture`"]
    fn test_manual_real_sudo_propagates_env_var_after_fix() {
        let status = Command::new("sudo")
            .args(["-S", "-v"])
            .status()
            .expect("failed to run sudo -v");
        assert!(status.success(), "sudo -v failed to authenticate");

        std::env::set_var("SUDO2_TEST_VAR", "hello-from-sudo2-test");

        let mut escalate = Escalate::builder();
        escalate.wrapper("sudo");
        let args = vec!["printenv".to_string(), "SUDO2_TEST_VAR".to_string()];
        let mut command = escalate.build_escalated_command(&args, &["SUDO2_TEST_VAR"], false);

        let output = command.output().expect("failed to run sudo");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "hello-from-sudo2-test"
        );
    }

    /// Confirms the fix's mechanism directly against real `sudo`: passing
    /// the var as a literal `NAME=value` argument to `env` survives sudo's
    /// env reset, unlike plain `Command::env()`.
    #[test]
    #[ignore = "invokes real sudo; run manually with `cargo test -- --ignored --nocapture`"]
    fn test_manual_real_sudo_env_prefix_workaround_propagates_var() {
        let output = Command::new("sudo")
            .args([
                "-S",
                "env",
                "SUDO2_TEST_VAR3=hello-again",
                "printenv",
                "SUDO2_TEST_VAR3",
            ])
            .output()
            .expect("failed to run sudo");

        assert!(
            output.status.success(),
            "sudo exited with {:?}; stderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout.trim(), "hello-again");
    }
}
