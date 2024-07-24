#[cfg(unix)]
fn main() {
    simple_logger::SimpleLogger::new()
        .init()
        .expect("unable to initialize logger");

    sudo2::escalate_if_needed().expect("sudo failed");

    failing_function();
}

#[cfg(unix)]
#[inline(never)]
fn failing_function() -> ! {
    tracing::info!("entering failing_function");
    panic!("now you see me fail")
}

#[cfg(not(unix))]
fn main() {
    panic!("only unix is supported");
}
