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

    /// Categorize functions by type (view, pure, payable, state-changing)
    #[arg(long, default_value_t = false)]
    categorize: bool,

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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum FunctionCategory {
    Constructor,
    Pure,
    View,
    Payable,
    StateChanging,
    Fallback,
    Receive,
}

impl FunctionCategory {
    fn comment(&self) -> &str {
        match self {
            FunctionCategory::Constructor => "Constructor",
            FunctionCategory::Pure => "Pure functions (no state read or write)",
            FunctionCategory::View => "View functions (read-only)",
            FunctionCategory::Payable => "Payable functions (can receive ETH)",
            FunctionCategory::StateChanging => "State-changing functions",
            FunctionCategory::Fallback => "Fallback function",
            FunctionCategory::Receive => "Receive function",
        }
    }
}

fn generate_sol_interface(abi: &JsonAbi, args: &Args) -> Result<String> {
    let mut output = String::new();

    // Start interface declaration
    if !args.compact {
        output.push_str("// Generated with abi2sol\n");
        output.push_str("// Usage: sol! { ... }\n\n");
    }

    output.push_str(&format!("interface {} {{\n", args.interface_name));

    // Collect and categorize items
    let mut structs = Vec::new();
    let mut functions_by_category: std::collections::BTreeMap<FunctionCategory, Vec<alloy::json_abi::Function>> = std::collections::BTreeMap::new();
    let mut events = Vec::new();
    let mut errors = Vec::new();

    // Process constructor
    if let Some(constructor) = &abi.constructor {
        if !args.compact && args.categorize {
            output.push_str(&format!("    // {}\n", FunctionCategory::Constructor.comment()));
        } else if !args.compact {
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

    // Process fallback
    if let Some(fallback) = &abi.fallback {
        let func = alloy::json_abi::Function {
            name: "".to_string(),
            inputs: vec![],
            outputs: vec![],
            state_mutability: fallback.state_mutability,
        };
        let category = categorize_function(&func);
        functions_by_category.entry(category).or_default().push(func);
    }

    // Process receive
    if let Some(receive) = &abi.receive {
        let func = alloy::json_abi::Function {
            name: "".to_string(),
            inputs: vec![],
            outputs: vec![],
            state_mutability: receive.state_mutability,
        };
        let category = categorize_function(&func);
        functions_by_category.entry(category).or_default().push(func);
    }

    // Extract items from ABI
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
                let category = categorize_function(func.as_ref());
                functions_by_category.entry(category).or_default().push(func.as_ref().clone());
            }
            _ => {}
        }
    }

    // Generate structs from function/event parameters
    if args.types {
        let mut seen_structs = std::collections::HashSet::new();

        // Scan all functions and events for tuple types (structs)
        for funcs in functions_by_category.values() {
            for func in funcs {
                for input in &func.inputs {
                    if let Some(struct_def) = extract_struct_from_param(&input.ty, &input.components, &input.name) {
                        if seen_structs.insert(get_struct_name(&input.ty, &input.internal_type, &input.name)) {
                            structs.push(struct_def);
                        }
                    }
                }
                for output in &func.outputs {
                    if let Some(struct_def) = extract_struct_from_param(&output.ty, &output.components, &output.name) {
                        if seen_structs.insert(get_struct_name(&output.ty, &output.internal_type, &output.name)) {
                            structs.push(struct_def);
                        }
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

    // Generate functions (categorized or not)
    if !functions_by_category.is_empty() {
        if args.categorize {
            // Output functions by category
            for (category, funcs) in functions_by_category {
                if funcs.is_empty() {
                    continue;
                }

                if !args.compact {
                    output.push_str(&format!("    // {}\n", category.comment()));
                }

                for func in funcs {
                    output.push_str(&format_function(&func, &category));
                }
                output.push('\n');
            }
        } else {
            // Output all functions together
            if !args.compact {
                output.push_str("    // Functions\n");
            }
            for funcs in functions_by_category.values() {
                for func in funcs {
                    let category = categorize_function(func);
                    output.push_str(&format_function(&func, &category));
                }
            }
            output.push('\n');
        }
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

fn categorize_function(func: &alloy::json_abi::Function) -> FunctionCategory {
    if func.name.is_empty() && func.inputs.is_empty() {
        return FunctionCategory::Receive;
    }
    if func.name.is_empty() {
        return FunctionCategory::Fallback;
    }

    match func.state_mutability {
        alloy::json_abi::StateMutability::Pure => FunctionCategory::Pure,
        alloy::json_abi::StateMutability::View => FunctionCategory::View,
        alloy::json_abi::StateMutability::Payable => FunctionCategory::Payable,
        alloy::json_abi::StateMutability::NonPayable => FunctionCategory::StateChanging,
    }
}

fn format_function(func: &alloy::json_abi::Function, category: &FunctionCategory) -> String {
    let mut output = String::new();

    // Handle special functions
    match category {
        FunctionCategory::Fallback => {
            output.push_str("    fallback(");
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
            if func.state_mutability == alloy::json_abi::StateMutability::Payable {
                output.push_str(" payable");
            }
            output.push_str(";\n");
            return output;
        }
        FunctionCategory::Receive => {
            output.push_str("    receive() external payable;\n");
            return output;
        }
        _ => {}
    }

    // Regular function
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
    output
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

fn get_struct_name(
    ty: &str,
    internal_type: &Option<alloy::json_abi::InternalType>,
    param_name: &str,
) -> String {
    if let Some(internal) = internal_type {
        match internal {
            alloy::json_abi::InternalType::Struct { contract: _, ty } => {
                return ty.clone();
            }
            _ => {}
        }
    }

    // Fallback: use parameter name or generic name
    if !param_name.is_empty() {
        // Convert camelCase to PascalCase
        let mut chars = param_name.chars();
        if let Some(first) = chars.next() {
            return format!("{}{}", first.to_uppercase(), chars.as_str());
        }
    }

    "CustomStruct".to_string()
}

fn extract_struct_from_param(
    ty: &str,
    components: &[alloy::json_abi::Param],
    param_name: &str,
) -> Option<String> {
    // Check if this is a tuple type (struct)
    if !ty.starts_with("tuple") || components.is_empty() {
        return None;
    }

    // Get struct name from internal type or derive from parameter name
    let struct_name = if !param_name.is_empty() {
        // Convert camelCase to PascalCase
        let mut chars = param_name.chars();
        if let Some(first) = chars.next() {
            format!("{}{}", first.to_uppercase(), chars.as_str())
        } else {
            "CustomStruct".to_string()
        }
    } else {
        "CustomStruct".to_string()
    };

    let mut struct_def = format!("    struct {} {{\n", struct_name);

    for component in components {
        let field_type = format_type(&component.ty, &component.internal_type);
        struct_def.push_str(&format!("        {} {};\n", field_type, component.name));
    }

    struct_def.push_str("    }");

    Some(struct_def)
}
