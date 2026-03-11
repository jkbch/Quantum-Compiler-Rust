mod ast;
mod helpers;
mod intrepret;
mod reduce;
mod show;

use crate::intrepret::test_expressions;
use crate::show::*;
use lalrpop_util::lalrpop_mod;
use std::fs;

use crate::cq::ProgramParser;

lalrpop_mod!(cq);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let code = fs::read_to_string("initialize.cq")?;
    let code: &'static str = Box::leak(code.into_boxed_str());
    let parser = ProgramParser::new();
    let program = parser.parse(code)?;

    println!("{}", show_program(&program));

    test_expressions();

    Ok(())
}
