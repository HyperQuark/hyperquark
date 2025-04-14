use convert_case::{Case, Casing};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;

// I hate to admit this, but a fair bit of this file was written by chatgpt to speed things up
// and to allow me to continue to procrastinate about learning how to do i/o stuff in rust.
// But I did write some of it!

fn main() {
    println!("cargo::rerun-if-changed=src/instructions/**/*.rs");

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path_rs = Path::new(&out_dir).join("ir-opcodes.rs");
    let dest_path_ts = Path::new(&out_dir).join("imports.ts");

    let base_dir = "src/instructions";

    let mut paths = Vec::new();
    let mut ts_paths = Vec::new();
    visit_dirs(Path::new(base_dir), &mut |entry| {
        if let Some(ext) = entry.path().extension() {
            if ext == "rs" {
                if let Ok(relative_path) = entry.path().strip_prefix(base_dir) {
                    let components: Vec<_> = relative_path
                        .components()
                        .filter_map(|comp| comp.as_os_str().to_str())
                        .collect();

                    if components.len() == 2 {
                        let category = components[0];
                        let opcode = components[1].trim_end_matches(".rs");
                        let contents =
                            fs::read_to_string(format!("src/instructions/{category}/{opcode}.rs"))
                                .unwrap();
                        let fields = contents.contains("pub struct Fields");
                        let fields_name =
                            format!("{}_{}_fields", category, opcode).to_case(Case::Pascal);
                        paths.push((
                            format!(
                                "{}::{}",
                                category,
                                match opcode {
                                    "yield" | "loop" => format!("r#{opcode}"),
                                    _ => opcode.to_string(),
                                }
                            ),
                            format!("{}_{}", category, opcode,),
                            fields,
                            fields_name,
                        ));
                    }
                }
            }
        }
    });
    visit_dirs(Path::new("js"), &mut |entry| {
        if let Some(ext) = entry.path().extension() {
            if ext == "ts" {
                if let Ok(relative_path) = entry.path().strip_prefix("js") {
                    let components: Vec<_> = relative_path
                        .components()
                        .filter_map(|comp| comp.as_os_str().to_str())
                        .collect();

                    if components.len() == 2 {
                        let category = components[0];
                        if category == "compiler" || category == "no-compiler" {
                            return;
                        }
                        let func = components[1].trim_end_matches(".ts");
                        ts_paths.push((category.to_string(), func.to_string()));
                    }
                }
            }
        }
    });

    fs::write(
        &dest_path_ts,
        format!(
            "
{}
export const imports = {{
    {}
}};
            ",
            ts_paths
                .iter()
                .map(|(dir, name)| format!("import {{ {name} }} from './{dir}/{name}.ts';"))
                .collect::<Vec<_>>()
                .join("\n"),
            HashSet::<String>::from_iter(ts_paths.iter().map(|(dir, _)| dir.clone()))
                .iter()
                .map(|dir| {
                    format!(
                        "{dir}: {{ {} }}",
                        ts_paths
                            .iter()
                            .filter(|(d, _)| d == dir)
                            .map(|(_, name)| name.clone())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                })
                .collect::<Vec<_>>()
                .join(",\n\t")
        ),
    )
    .unwrap();

    fs::write(
        &dest_path_rs,
        format!(
            "
/// A list of all instructions.
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
    pub fn wasm(&self, step_func: &crate::wasm::StepFunc, inputs: Rc<[crate::ir::Type]>) -> HQResult<Vec<crate::wasm::InternalInstruction>> {{
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

    /// does this opcode request a screen refresh (and by extension yields)?
    pub const fn requests_screen_refresh(&self) -> bool {{
        match self {{
            {}
        }}
    }}
}}
pub mod fields {{
    use super::*;
    {}
}}
pub use fields::*;
        ",
            paths.iter().map(|(_, id, fields, fields_name)| {
                if *fields {
                    format!("{}({})", id.clone(), fields_name.clone())
                } else {
                    id.clone()
                }
            }).collect::<Vec<_>>().join(",\n\t"),
            paths.iter().map(|(path, id, fields, _)| {
                if *fields {
                    format!("IrOpcode::{}(fields) => {}::acceptable_inputs(fields),", id, path)
                } else {
                    format!("IrOpcode::{} => {}::acceptable_inputs(),", id, path)
                }
            }).collect::<Vec<_>>().join("\n\t\t\t"),
            paths.iter().map(|(path, id, fields, _)| {
                if *fields {
                    format!("IrOpcode::{}(fields) => {}::wasm(step_func, inputs, fields),", id, path)
                } else {
                    format!("IrOpcode::{} => {}::wasm(step_func, inputs),", id, path)
                }
            }).collect::<Vec<_>>().join("\n\t\t\t"),
            paths.iter().map(|(path, id, fields, _)| {
                if *fields {
                    format!("IrOpcode::{}(fields) => {}::output_type(inputs, fields),", id, path)
                } else {
                    format!("IrOpcode::{} => {}::output_type(inputs),", id, path)
                }
            }).collect::<Vec<_>>().join("\n\t\t\t"),
            paths.iter().map(|(path, id, fields, _)| {
                if *fields {
                    format!("IrOpcode::{}(_) => {}::REQUESTS_SCREEN_REFRESH,", id, path)
                } else {
                    format!("IrOpcode::{} => {}::REQUESTS_SCREEN_REFRESH,", id, path)
                }
            }).collect::<Vec<_>>().join("\n\t\t\t"),
            paths.iter().filter(|(_, _, fields, _)| *fields)
            .map(|(path, _, _, fields_name)|
                format!("pub use {}::Fields as {};", path, fields_name)
            ).collect::<Vec<_>>().join("\n\t"),
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
