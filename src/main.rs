mod ast;
mod eval;
mod helper;
mod part_eval;
mod route;
mod show;
mod synth;
mod typer;
mod vars;

use crate::{helper::*, part_eval::part_eval_program, show::*, typer::type_program};
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
        type_program(&program);

        let mut static_env = Env::new();
        static_env.insert("d".to_string(), Some(Value::Scalar(Scalar::Int(5))));
        static_env.insert(
            "a".to_string(),
            Some(Value::Array(Array::Float(vec![0.1, 0.2, 0.3, 0.4]))),
        );

        program = part_eval_program(program, static_env.clone());

        println!("\nReduced program:\n{}", show_program(&program));
        println!("--------------------------------------------\n");
        type_program(&program);
    }

    Ok(())
}
