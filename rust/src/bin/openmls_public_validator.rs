//! One-shot public-only validator. No argv paths, network, private state or DB.
use openmls_frb::external_resync::codec::{REQUEST_MAX_BYTES, execute};
use std::io::{Read, Write};
fn main() {
    // Do not print panic internals or request bytes into backend diagnostic logs.
    std::panic::set_hook(Box::new(|_| {}));
    let mut input = Vec::new();
    if std::io::stdin()
        .take((REQUEST_MAX_BYTES + 1) as u64)
        .read_to_end(&mut input)
        .is_err()
    {
        std::process::exit(74);
    }
    let result = execute(&input);
    if std::io::stdout().write_all(&result).is_err() {
        std::process::exit(74);
    }
}
