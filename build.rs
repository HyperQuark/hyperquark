use convert_case::{Case, Casing};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

// I hate to admit this, but a fair bit of this file was written by chatgpt to speed things up
// and to allow me to continue to procrastinate about learning how to do i/o stuff in rust.
// But I did write some of it!

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
                        let category = components[0];
                        let opcode = components[1].trim_end_matches(".rs");
                        println!("src/instructions/{category}/{opcode}.rs");
                        let contents = fs::read_to_string(
                            format!("src/instructions/{category}/{opcode}.rs")
                        )
                        .unwrap();
                        let fields = contents.contains("pub struct Fields");
                        let fields_name =
                            format!("{}_{}_fields", category, opcode).to_case(Case::Pascal);
                        paths.push((
                            format!("{}::{}", category, opcode),
                            format!("{}_{}", category, opcode,),
                            fields,
                            fields_name,
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
        #[derive(Clone, Debug)]
        pub enum IrOpcode {{
            {}
        }}
        
        impl IrOpcode {{
            /// maps an opcode to its acceptable input types
            pub fn acceptable_inputs(&self) -> Rc<[crate::ir::Type]> {{
                match self {{
                    {}
                }}
            }}
        
            /// maps an opcode to its WASM instructions
            pub fn wasm(&self, step_func: &crate::wasm::StepFunc, inputs: Rc<[crate::ir::Type]>) -> HQResult<Vec<wasm_encoder::Instruction<'static>>> {{
                match self {{
                    {}
                }}
            }}
            
            /// maps an opcode to its output type
            pub fn output_type(&self, inputs: Rc<[crate::ir::Type]>) -> HQResult<Option<crate::ir::Type>> {{
                match self {{
                    {}
                }}
            }}
        }}

        {}
        ",
            paths.iter().map(|(_, id, fields, fields_name)| {
                if *fields {
                    format!("{}({})", id.clone(), fields_name.clone())
                } else {
                    id.clone()
                }
            }).collect::<Vec<_>>().join(", "),
            paths.iter().map(|(path, id, fields, _)| {
                if *fields {
                    format!("IrOpcode::{}(_) => {}::acceptable_inputs(),", id, path)
                } else {
                    format!("IrOpcode::{} => {}::acceptable_inputs(),", id, path)
                }
            }).collect::<Vec<_>>().join("\n"),
            paths.iter().map(|(path, id, fields, _)| {
                if *fields {
                    format!("IrOpcode::{}(fields) => {}::wasm(step_func, inputs, fields),", id, path)
                } else {
                    format!("IrOpcode::{} => {}::wasm(step_func, inputs),", id, path)
                }
            }).collect::<Vec<_>>().join("\n"),
            paths.iter().map(|(path, id, fields, _)| {
                if *fields {
                    format!("IrOpcode::{}(_) => {}::output_type(inputs),", id, path)
                } else {
                    format!("IrOpcode::{} => {}::output_type(inputs),", id, path)
                }
            }).collect::<Vec<_>>().join("\n"),
            paths.iter().filter(|(_, _, fields, _)| *fields)
            .map(|(path, id, _, fields_name)|
                format!("pub use {}::Fields as {};", path, fields_name)
            ).collect::<Vec<_>>().join("\n"),
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
