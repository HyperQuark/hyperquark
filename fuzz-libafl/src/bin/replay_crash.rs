use arbitrary::Arbitrary;
use hyperquark::wasm::flags::all_wasm_features;
use std::backtrace::Backtrace;
use std::path::PathBuf;

fn usage(bin_name: &str) {
    eprintln!("Usage: {bin_name} <crash-file> [output-project-json]");
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> &'static str {
    if let Some(msg) = payload.downcast_ref::<&'static str>() {
        msg
    } else {
        "<non-string panic payload>"
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args();
    let bin_name = args.next().unwrap_or_else(|| "replay_crash".into());

    let Some(crash_file) = args.next() else {
        usage(&bin_name);
        return Err("missing crash-file argument".into());
    };

    let output_path = args.next().map(PathBuf::from);

    let data = std::fs::read(&crash_file)?;
    let mut unstructured = arbitrary::Unstructured::new(&data);

    let structured = sb3::structured::StructuredProject::arbitrary(&mut unstructured)
        .map_err(|_| "failed to decode crash bytes as StructuredProject")?;

    let raw_project: sb3::raw::Sb3Project = structured
        .try_into()
        .map_err(|_| "decoded structure could not be converted to raw sb3")?;

    let json = serde_json::to_string_pretty(&raw_project)?;

    if let Some(path) = output_path {
        std::fs::write(&path, json.as_bytes())?;
        println!("Recovered SB3 JSON written to {}", path.display());
    }

    let flags = hyperquark::wasm::WasmFlags::new(all_wasm_features());

    let replay = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = hyperquark::sb3_to_wasm(&json, flags);
    }));

    match replay {
        Ok(()) => {
            println!("No panic reproduced for this crash input.");
            Ok(())
        }
        Err(payload) => {
            eprintln!("Panic reproduced while replaying {crash_file}");

            if let Some(msg) = payload.downcast_ref::<String>() {
                eprintln!("panic message: {msg}");
            } else {
                eprintln!("panic message: {}", panic_message(&*payload));
            }

            eprintln!("backtrace:\n{}", Backtrace::force_capture());
            std::process::exit(101);
        }
    }
}
