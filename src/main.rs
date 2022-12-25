use anyhow::Result;
use clap::Parser;
use glob::glob;
use object::{File as ObjectFile, Object, ObjectSymbol};
use std::{collections, fs, path::PathBuf};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    kernel: String,

    #[arg(short, long)]
    modules: String,

    #[arg(short, long)]
    target: String,
}

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
    let binary_data = fs::read(&path)?;

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

fn main() {
    let args = Args::parse();

    print!("Parsing kernel...");

    let kernel_brief = read_to_module(PathBuf::from(args.kernel)).unwrap();

    println!("done.");
    print!("Parsing modules...");

    let kernel_modules: Vec<ModuleBrief> = glob(format!("{}/**/*.ko", args.modules).as_str())
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

    println!("done.");
    print!("Resolving dependency tree...");

    let kernel_plus_all_modules = [&kernel_modules[..], &[kernel_brief]].concat();

    let wireguard_module_tree = resolve_dependency_tree(kernel_plus_all_modules, args.target);

    println!("done.");

    for module in wireguard_module_tree {
        if module.name != "vmlinux" {
            println!("{}", module.path);
        }
    }
}
