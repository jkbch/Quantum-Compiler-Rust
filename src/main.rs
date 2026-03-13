mod ast;
mod helpers;
mod intrepret;
mod reduce;
mod show;
// mod typechecker;

use crate::{helpers::*, reduce::reduce_program, show::*};
use lalrpop_util::lalrpop_mod;
use std::{collections::HashMap, fs};

use crate::cq::ProgramParser;

lalrpop_mod!(cq);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let folder = "./programs"; // Folder with .cq files

    // Collect all .cq files
    let files: Vec<_> = fs::read_dir(folder)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().map(|ext| ext == "cq").unwrap_or(false))
        .collect();

    let parser = ProgramParser::new();

    for file_path in files {
        println!("Processing file: {}", file_path.display());

        let code = fs::read_to_string(&file_path)?;
        let code: &'static str = Box::leak(code.into_boxed_str());

        // Parse the program
        let program = parser.parse(code)?;
        println!("\nOriginal program:\n{}", show_program(program.clone()));

        // Initialize static input environment
        let mut static_env = Env::new();

        // Define classical variables for testing
        // For loops that depend on 'd'
        static_env.insert("d".to_string(), Value::Int(3));

        // For initialize_2qubit classical array input 'a'
        static_env.insert(
            "a".to_string(),
            Value::Array(vec![
                Value::Float(0.5),
                Value::Float(0.5),
                Value::Float(0.5),
                Value::Float(0.5),
            ]),
        );

        // Partially evaluate / reduce the program
        let reduced_program = reduce_program(program, static_env.clone());
        let reduced_program = reduce_program(reduced_program, static_env.clone());

        println!("\nReduced program:\n{}", show_program(reduced_program));
        println!("--------------------------------------------\n");
    }

    Ok(())
}
