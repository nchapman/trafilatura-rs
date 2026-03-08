fn main() {
    if let Err(err) = uniffi_bindgen_dart::run(std::env::args_os()) {
        eprintln!("{err:#}");
        std::process::exit(1);
    }
}
