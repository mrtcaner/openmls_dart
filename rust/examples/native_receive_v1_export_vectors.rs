fn main() {
    let output = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "../native/receive_v1/fixtures".to_string());
    if let Err(error) = openmls_frb::native_receive_v1_vectors::write_native_receive_v1_vectors(
        std::path::Path::new(&output),
    ) {
        eprintln!("native receive v1 vector export failed: {error}");
        std::process::exit(1);
    }
}
