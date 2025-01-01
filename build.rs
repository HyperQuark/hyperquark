use std::env;
use std::fs;
use std::path::Path;

// I hate to admit this, but much of this was written by chatgpt to speed things up
// and to allow me to continue to procrastinate about learning how to do i/o stuff in rust.

fn main() {
    println!("cargo::rerun-if-changed=src/instructions/**/*.rs");

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("ir-opcodes.rs");

    // Define the base directory to search for files.
    let base_dir = "src/instructions";

    // Collect file paths
    let mut paths = Vec::new();
    visit_dirs(Path::new(base_dir), &mut |entry| {
        if let Some(ext) = entry.path().extension() {
            if ext == "rs" {
                if let Ok(relative_path) = entry.path().strip_prefix(base_dir) {
                    let components: Vec<_> = relative_path
                        .components()
                        .filter_map(|comp| comp.as_os_str().to_str())
                        .collect();

                    if components.len() == 2 && components[1].ends_with(".rs") {
                        paths.push((
                            format!(
                                "{}::{}",
                                components[0],
                                components[1].trim_end_matches(".rs")
                            ),
                            format!(
                                "{}_{}",
                                components[0],
                                components[1].trim_end_matches(".rs")
                            ),
                        ));
                    }
                }
            }
        }
    });

    fs::write(
        &dest_path,
        format!(
            "/// A list of all instructions.
        #[allow(non_camel_case_types)]
        pub enum IrOpcode {{
            {}
        }}
        
        /// maps an opcode to its acceptable input types
        pub fn acceptable_inputs(opcode: IrOpcode) -> Rc<[crate::ir::Type]> {{
            match opcode {{
                {}
            }}
        }}
        
        /// maps an opcode to its WASM instructions
        pub fn wasm(opcode: IrOpcode, step_func: &crate::wasm::StepFunc, inputs: Rc<[crate::ir::Type]>) -> HQResult<Vec<wasm_encoder::Instruction<'static>>> {{
            match opcode {{
                {}
            }}
        }}
        
        /// maps an opcode to its output type
        pub fn output_type(opcode: IrOpcode, inputs: Rc<[crate::ir::Type]>) -> HQResult<Option<crate::ir::Type>> {{
            match opcode {{
                {}
            }}
        }}
        ",
            paths.iter().map(|(_, id)| id.clone()).collect::<Vec<_>>().join(", "),
            paths.iter().map(|(path, id)| format!("IrOpcode::{} => {}::acceptable_inputs(),", id, path)).collect::<Vec<_>>().join("\n"),
            paths.iter().map(|(path, id)| format!("IrOpcode::{} => {}::wasm(step_func, inputs),", id, path)).collect::<Vec<_>>().join("\n"),
            paths.iter().map(|(path, id)| format!("IrOpcode::{} => {}::output_type(inputs),", id, path)).collect::<Vec<_>>().join("\n"),
    ))
    .unwrap();
}

fn visit_dirs(dir: &Path, cb: &mut dyn FnMut(&fs::DirEntry)) {
    if dir.is_dir() {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    visit_dirs(&path, cb);
                } else {
                    cb(&entry);
                }
            }
        }
    }
}
