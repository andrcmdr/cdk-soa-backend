use alloy::json_abi::JsonAbi;
use anyhow::{Context, Result};
use clap::Parser;
use std::io::{self, Read};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "abi2sol")]
#[command(about = "Convert ABI JSON to Solidity signatures for sol!() macro", long_about = None)]
struct Args {
    /// Path to ABI JSON file (use '-' for stdin)
    #[arg(value_name = "FILE")]
    input: Option<PathBuf>,

    /// Interface name to use in the output
    #[arg(short, long, default_value = "IContract")]
    interface_name: String,

    /// Include events in the output
    #[arg(short, long, default_value_t = true)]
    events: bool,

    /// Include errors in the output
    #[arg(short = 'r', long, default_value_t = true)]
    errors: bool,

    /// Include structs/types in the output
    #[arg(short = 't', long, default_value_t = true)]
    types: bool,

    /// Compact output (no comments)
    #[arg(short, long, default_value_t = false)]
    compact: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Read ABI JSON from file or stdin
    let json_content = if let Some(path) = &args.input {
        if path.to_str() == Some("-") {
            read_stdin()?
        } else {
            std::fs::read_to_string(path)
                .with_context(|| format!("Failed to read file: {}", path.display()))?
        }
    } else {
        read_stdin()?
    };

    // Parse JSON ABI
    let abi: JsonAbi = serde_json::from_str(&json_content)
        .context("Failed to parse ABI JSON")?;

    // Generate Solidity interface
    let output = generate_sol_interface(&abi, &args)?;

    println!("{}", output);

    Ok(())
}

fn read_stdin() -> Result<String> {
    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .context("Failed to read from stdin")?;
    Ok(buffer)
}

fn generate_sol_interface(abi: &JsonAbi, args: &Args) -> Result<String> {
    let mut output = String::new();

    // Start interface declaration
    if !args.compact {
        output.push_str("// Generated with abi2sol\n");
        output.push_str("// Usage: sol! { ... }\n\n");
    }

    output.push_str(&format!("interface {} {{\n", args.interface_name));

    // Collect and sort items by type for better organization
    let mut structs = Vec::new();
    let mut functions = Vec::new();
    let mut events = Vec::new();
    let mut errors = Vec::new();

    // Process constructor
    if let Some(constructor) = &abi.constructor {
        if !args.compact {
            output.push_str("    // Constructor\n");
        }
        output.push_str("    constructor(");
        let params: Vec<String> = constructor
            .inputs
            .iter()
            .map(|p| format!("{} {}", p.ty, p.name))
            .collect();
        output.push_str(&params.join(", "));
        output.push_str(");\n\n");
    }

    // Extract user-defined types (structs)
    for item in abi.items() {
        match item {
            alloy::json_abi::AbiItem::Error(err) => {
                if args.errors {
                    errors.push((err.name.clone(), err));
                }
            }
            alloy::json_abi::AbiItem::Event(event) => {
                if args.events {
                    events.push((event.name.clone(), event));
                }
            }
            alloy::json_abi::AbiItem::Function(func) => {
                functions.push((func.name.clone(), func));
            }
            _ => {}
        }
    }

    // Generate structs from function/event parameters
    if args.types {
        let mut seen_structs = std::collections::HashSet::new();

        // Scan all functions and events for tuple types (structs)
        for (_, func) in &functions {
            for input in &func.inputs {
                if let Some(struct_def) = extract_struct_from_param(&input.ty, &input.components) {
                    if seen_structs.insert(input.name.clone()) {
                        structs.push(struct_def);
                    }
                }
            }
            for output in &func.outputs {
                if let Some(struct_def) = extract_struct_from_param(&output.ty, &output.components) {
                    if seen_structs.insert(output.name.clone()) {
                        structs.push(struct_def);
                    }
                }
            }
        }

        if !structs.is_empty() {
            if !args.compact {
                output.push_str("    // Types\n");
            }
            for struct_def in structs {
                output.push_str(&struct_def);
                output.push('\n');
            }
            output.push('\n');
        }
    }

    // Generate functions
    if !functions.is_empty() {
        if !args.compact {
            output.push_str("    // Functions\n");
        }
        for (_, func) in functions {
            output.push_str("    function ");
            output.push_str(&func.name);
            output.push('(');

            let params: Vec<String> = func
                .inputs
                .iter()
                .map(|p| {
                    let param_type = format_type(&p.ty, &p.internal_type);
                    if p.name.is_empty() {
                        param_type
                    } else {
                        format!("{} {}", param_type, p.name)
                    }
                })
                .collect();
            output.push_str(&params.join(", "));
            output.push_str(") external");

            // Add state mutability
            match func.state_mutability {
                alloy::json_abi::StateMutability::Pure => output.push_str(" pure"),
                alloy::json_abi::StateMutability::View => output.push_str(" view"),
                alloy::json_abi::StateMutability::Payable => output.push_str(" payable"),
                alloy::json_abi::StateMutability::NonPayable => {}
            }

            // Add return types
            if !func.outputs.is_empty() {
                output.push_str(" returns (");
                let returns: Vec<String> = func
                    .outputs
                    .iter()
                    .map(|p| {
                        let return_type = format_type(&p.ty, &p.internal_type);
                        if p.name.is_empty() {
                            return_type
                        } else {
                            format!("{} {}", return_type, p.name)
                        }
                    })
                    .collect();
                output.push_str(&returns.join(", "));
                output.push(')');
            }

            output.push_str(";\n");
        }
        output.push('\n');
    }

    // Generate events
    if !events.is_empty() && args.events {
        if !args.compact {
            output.push_str("    // Events\n");
        }
        for (_, event) in events {
            output.push_str("    event ");
            output.push_str(&event.name);
            output.push('(');

            let params: Vec<String> = event
                .inputs
                .iter()
                .map(|p| {
                    let param_type = format_type(&p.ty, &p.internal_type);
                    let indexed = if p.indexed { " indexed" } else { "" };
                    if p.name.is_empty() {
                        format!("{}{}", param_type, indexed)
                    } else {
                        format!("{}{} {}", param_type, indexed, p.name)
                    }
                })
                .collect();
            output.push_str(&params.join(", "));
            output.push_str(");\n");
        }
        output.push('\n');
    }

    // Generate errors
    if !errors.is_empty() && args.errors {
        if !args.compact {
            output.push_str("    // Errors\n");
        }
        for (_, error) in errors {
            output.push_str("    error ");
            output.push_str(&error.name);
            output.push('(');

            let params: Vec<String> = error
                .inputs
                .iter()
                .map(|p| {
                    let param_type = format_type(&p.ty, &p.internal_type);
                    if p.name.is_empty() {
                        param_type
                    } else {
                        format!("{} {}", param_type, p.name)
                    }
                })
                .collect();
            output.push_str(&params.join(", "));
            output.push_str(");\n");
        }
    }

    output.push_str("}\n");

    Ok(output)
}

fn format_type(ty: &str, internal_type: &Option<alloy::json_abi::InternalType>) -> String {
    // Use internal type if available for better struct names
    if let Some(internal) = internal_type {
        match internal {
            alloy::json_abi::InternalType::Struct { contract: _, ty } => {
                return ty.clone();
            }
            alloy::json_abi::InternalType::Enum { contract: _, ty } => {
                return ty.clone();
            }
            _ => {}
        }
    }

    ty.to_string()
}

fn extract_struct_from_param(
    ty: &str,
    components: &[alloy::json_abi::Param],
) -> Option<String> {
    // Check if this is a tuple type (struct)
    if !ty.starts_with("tuple") || components.is_empty() {
        return None;
    }

    // Try to derive a struct name from the type string or use a generic name
    let struct_name = "CustomStruct"; // In practice, this should be extracted from context

    let mut struct_def = format!("    struct {} {{\n", struct_name);

    for component in components {
        let field_type = format_type(&component.ty, &component.internal_type);
        struct_def.push_str(&format!("        {} {};\n", field_type, component.name));
    }

    struct_def.push_str("    }");

    Some(struct_def)
}
