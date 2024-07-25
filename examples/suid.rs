#[cfg(unix)]
fn main() {
    simple_logger::SimpleLogger::new()
        .init()
        .expect("unable to initialize logger");

    uid_euid("①");

    spawn("/usr/bin/id");

    sudo2::escalate_if_needed().expect("sudo failed");

    uid_euid("②");

    spawn("/usr/bin/id");
}

#[cfg(unix)]
fn uid_euid(nth: &str) {
    let euid = unsafe { libc::geteuid() };
    let uid = unsafe { libc::getuid() };
    tracing::info!("{} uid: {}; euid: {};", nth, uid, euid);
}

#[cfg(unix)]
fn spawn(cmd: &str) {
    let mut child = std::process::Command::new(cmd)
        .spawn()
        .expect("unable to start child");

    let _ecode = child.wait().expect("failed to wait on child");
}

#[cfg(not(unix))]
fn main() {
    panic!("only unix is supported");
}
