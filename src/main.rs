mod ast;
mod helpers;
mod intrepret;
mod reduce;
mod show;
mod vars;
// mod typechecker;
// mod flatten;

use crate::{helpers::*, reduce::reduce_program, show::*};
use lalrpop_util::lalrpop_mod;
use std::fs;

use crate::cq::ProgramParser;
lalrpop_mod!(cq);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let folder = "/home/jakob/Uni/Quantum Compiler/Rust/programs";
    let files = vec!["initialize.cq", "qft.cq", "qft2.cq"];
    let parser = ProgramParser::new();

    for file_name in files {
        let file_path = format!("{}/{}", folder, file_name);
        println!("Processing file: {}", file_path);
        let code: &'static str = Box::leak(fs::read_to_string(&file_path)?.into_boxed_str());

        let mut program = parser.parse(code)?;
        println!("\nOriginal program:\n{}", show_program(&program));

        let mut static_env = Env::new();
        static_env.insert("d".to_string(), Value::Scalar(Scalar::Int(5)));
        static_env.insert(
            "a".to_string(),
            Value::Array(Array::Float(vec![0.5, 0.5, 0.5, 0.5])),
        );

        for _ in 1..3 {
            program = reduce_program(program, static_env.clone());
        }

        println!("\nReduced program:\n{}", show_program(&program));
        println!("--------------------------------------------\n");
    }

    Ok(())
}
