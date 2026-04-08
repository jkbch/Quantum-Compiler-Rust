mod ast;
mod eval;
mod helper;
mod part_eval;
mod route;
mod show;
mod synth;
mod typer;
mod vars;

use crate::{
    helper::*,
    part_eval::part_eval_program,
    route::{line_hardware, route_circuit, t_shape_hardware},
    show::*,
    synth::{Gate1, Gate2, Op, synth_program},
    typer::type_program,
};
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
        static_env.insert("d".to_string(), Some(Value::Scalar(Scalar::Int(4))));
        static_env.insert(
            "a".to_string(),
            Some(Value::Array(Array::Float(vec![0.1, 0.2, 0.3, 0.4]))),
        );

        program = part_eval_program(program, static_env.clone());

        println!("\nReduced program:\n{}", show_program(&program));
        println!("--------------------------------------------\n");
        type_program(&program);

        let ops = synth_program(&program, static_env.clone());
        println!("Synthesied circuit:");
        for op in ops.iter() {
            println!("{:?}", op);
        }
        println!("--------------------------------------------\n");

        // Example circuit
        // let ops = vec![
        //     Op::Gate1(Gate1::X, 0),
        //     Op::Gate2(Gate2::CX, 0, 2), // not neighbors on line
        //     Op::Gate1(Gate1::Rz(1.0), 2),
        //     Op::Gate2(Gate2::CX, 1, 3), // neighbor on line
        // ];

        // let hardware = line_hardware();
        let hardware = t_shape_hardware();
        let routed_ops = route_circuit(ops, &hardware);
        println!("Routed circuit:");
        let mut cnot_count = 0;
        for op in routed_ops.iter() {
            if matches!(op, Op::Gate2(Gate2::CX, _, _)) {
                cnot_count += 1;
            }
            println!("{:?}", op);
        }
        println!("CNOT count: {}", cnot_count);
        println!("--------------------------------------------\n");
    }

    Ok(())
}
