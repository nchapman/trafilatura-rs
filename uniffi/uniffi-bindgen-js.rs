use clap::Parser;

fn main() {
    let cli = uniffi_bindgen_js::cli::Cli::parse();
    if let Err(err) = uniffi_bindgen_js::run(cli) {
        eprintln!("{err:#}");
        std::process::exit(1);
    }
}
