use anyhow::Result;
use clap::{Arg, Command};
use glob::glob;
use object::{File as ObjectFile, Object, ObjectSymbol};
use std::{
    collections,
    fs::{self, File},
    io::Read,
    path::PathBuf,
};
use xz::read::XzDecoder;
use zstd::decode_all;

mod live;

#[derive(Debug, PartialEq, Clone, Default, Eq, PartialOrd, Ord)]
struct ModuleBrief {
    name: String,
    path: String,
    provides_symbols: Vec<String>,
    references_symbols: Vec<String>,
}

#[derive(Debug, PartialEq, Clone, Eq, PartialOrd, Ord)]
enum SymbolDirection {
    Provides,
    References,
}

#[derive(Debug, PartialEq, Clone, Eq, PartialOrd, Ord)]
struct SymbolBrief {
    name: String,
    direction: SymbolDirection,
}

fn resolve_dependency_tree(
    all_modules: Vec<ModuleBrief>,
    for_module_name: String,
) -> Vec<ModuleBrief> {
    // Perform a topological sort on the dependency graph to get the correct order of modules to be loaded
    let mut sorted_modules: Vec<ModuleBrief> = Vec::new();

    // Get the module we are trying to load
    let for_module = all_modules
        .iter()
        .find(|m| m.name == for_module_name)
        .unwrap();

    // Get all the modules that the module we are trying to load references
    let referenced_modules: Vec<ModuleBrief> = all_modules
        .iter()
        .filter(|m| {
            for_module
                .references_symbols
                .iter()
                .any(|s| m.provides_symbols.contains(s))
        })
        .cloned()
        .collect();

    // Recursively resolve the dependency tree for each of the referenced modules
    for referenced_module in referenced_modules {
        sorted_modules.append(&mut resolve_dependency_tree(
            all_modules.clone(),
            referenced_module.name,
        ));
    }

    // Add the module we are trying to load to the end of the list
    sorted_modules.push(for_module.clone());

    // Remove duplicates from the list but preserve the order

    let mut seen: collections::HashSet<String> = collections::HashSet::new();
    let mut unique_sorted_modules: Vec<ModuleBrief> = Vec::new();

    for module in sorted_modules {
        let module_name = module.name.clone();
        if !seen.contains(&module_name) {
            unique_sorted_modules.push(module);
            seen.insert(module_name);
        }
    }

    unique_sorted_modules
}

fn read_to_module(path: PathBuf) -> Result<ModuleBrief> {
    // println!("[Debug] filetype for {}: {}", &path.to_str().unwrap(), infer::get_from_path(&path).unwrap().unwrap().mime_type());

    let binary_data: Vec<u8> = match infer::get_from_path(&path).unwrap().unwrap().mime_type() {
        "application/x-executable" | "application/vnd.microsoft.portable-executable" => {
            fs::read(&path)?
        }
        "application/zstd" => decode_all(File::open(&path)?)?,
        "application/x-xz" => {
            let decoder = XzDecoder::new(File::open(&path)?);
            decoder.bytes().collect::<Result<Vec<u8>, _>>()?
        }
        _ => {
            panic!("Unknown file type for {}", path.to_str().unwrap());
        }
    };

    let obj_file = ObjectFile::parse(&*binary_data)?;

    let all_symbols: Vec<SymbolBrief> = obj_file
        .symbols()
        .filter_map(|sym| {
            if sym.kind() == object::SymbolKind::Unknown && sym.is_global() {
                Some(SymbolBrief {
                    name: sym.name().unwrap().to_string(),
                    direction: SymbolDirection::References,
                })
            } else if sym.kind() != object::SymbolKind::Unknown && sym.is_global() {
                Some(SymbolBrief {
                    name: sym.name().unwrap().to_string(),
                    direction: SymbolDirection::Provides,
                })
            } else {
                None
            }
        })
        .collect();

    Ok(ModuleBrief {
        name: path.file_name().unwrap().to_str().unwrap().to_string(),
        path: path.to_str().unwrap().to_string(),
        provides_symbols: all_symbols
            .iter()
            .filter(|s| s.direction == SymbolDirection::Provides)
            .map(|s| s.name.clone())
            .collect(),
        references_symbols: all_symbols
            .iter()
            .filter(|s| s.direction == SymbolDirection::References)
            .map(|s| s.name.clone())
            .collect(),
    })
}

fn print_module_dependency_tree(kernel_path: &str, modules_pattern: &str, module_name: &str) {
    let kernel_brief = read_to_module(PathBuf::from(kernel_path)).unwrap();
    let modules_glob_pattern = modules_pattern.to_string();
    let kernel_modules: Vec<ModuleBrief> = glob(modules_glob_pattern.as_str())
        .expect("Failed to read glob pattern")
        .filter_map(|entry| match entry {
            Ok(path) => match read_to_module(path) {
                Ok(module) => Some(module),
                Err(e) => {
                    println!("Error: {:?}", e);
                    None
                }
            },
            Err(e) => {
                println!("Error: {:?}", e);
                None
            }
        })
        .collect();

    let kernel_plus_all_modules = [&kernel_modules[..], &[kernel_brief]].concat();

    let wireguard_module_tree =
        resolve_dependency_tree(kernel_plus_all_modules, module_name.to_string());

    for module in wireguard_module_tree {
        if module.name != "vmlinux" {
            println!("{}", module.path);
        }
    }
}

fn main() {
    let m = Command::new("ModuleRS")
        .author("Isaac Parker, isaac@linux.com")
        .version("0.1.0")
        .about("Linux kernel module utility")
        .subcommand(Command::new("modprobe").about("Load a module"))
        .subcommand(Command::new("lsmod").about("List loaded modules"))
        .subcommand(Command::new("modinspect").args(vec![
                Arg::new("kernel")
                    .short('k')
                    .long("kernel")
                    .default_value("/boot/vmlinuz"),
                Arg::new("modules")
                    .short('m')
                    .long("modules")
                    .default_value("/lib/modules/*/kernel/**/*.ko"),
                Arg::new("target")
                    .short('t')
                    .long("target")
                    .default_value(""),
                ]))
        .get_matches();

    match m.subcommand() {
        Some(("lsmod", _)) => {
            live::parse_module_listing(fs::read_to_string("/proc/modules").unwrap().as_str());
        }
        Some(("modprobe", _)) => {
            panic!("Not yet implemented");
        }
        Some(("modinspect", args)) => {
            let kernel = args
                .get_one::<String>("kernel")
                .ok_or("No kernel path provided")
                .unwrap();
            let modules = args
                .get_one::<String>("modules")
                .ok_or("No modules path provided")
                .unwrap();
            let target = args
                .get_one::<String>("target")
                .ok_or("No target module provided")
                .unwrap();
            print_module_dependency_tree(kernel.as_str(), modules.as_str(), target.as_str());
        }
        _ => {
            println!("No subcommand");
        }
    }
}
