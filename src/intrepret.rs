use crate::ast::*;
use crate::cq::ExpParser;
use crate::helpers::*;
use crate::reduce::reduce_exp;
use std::collections::HashMap;

/// Main expression interpreter
pub fn interpret_exp(e: Exp, env: &Env) -> Option<Value> {
    match e {
        Exp::Int(i) => Some(Value::Int(i)),
        Exp::Float(f) => Some(Value::Float(f)),
        Exp::NamedConst(s) => Some(eval_const(&s)),
        Exp::Lval(l) => interpret_lval(l, env),
        Exp::Unary(op, e1) => {
            let v = interpret_exp(*e1, env)?;
            Some(eval_unop(&op, v))
        }
        Exp::Binary(e1, op, e2) => {
            let lhs = interpret_exp(*e1, env)?;
            let rhs = interpret_exp(*e2, env)?;
            Some(eval_binop(&op, lhs, rhs))
        }
        Exp::Builtin1(f, e1) => {
            let v = interpret_exp(*e1, env)?;
            Some(eval_fun_1(&f, v))
        }
        Exp::Builtin2(f, e1, e2) => {
            let lhs = interpret_exp(*e1, env)?;
            let rhs = interpret_exp(*e2, env)?;
            Some(eval_fun_2(&f, lhs, rhs))
        }
    }
}

pub fn interpret_lval(l: Lval, env: &Env) -> Option<Value> {
    match l {
        Lval::Var(name) => lookup_val(&name, env),
        Lval::Array(name, idx_exp) => {
            let idx_val = interpret_exp(*idx_exp, env)?;
            let idx = match idx_val {
                Value::Int(i) => i as usize,
                _ => return None,
            };

            if let Some(Value::Array(arr)) = lookup_val(&name, env) {
                arr.get(idx).cloned()
            } else {
                None
            }
        }
    }
}

pub fn test_expressions() {
    // Expressions to test
    let expressions = vec![
        "0",
        "42",
        "0x10",
        "0b1010",
        "3.14",
        "-2.5",
        "1.0e3",
        "-4.2e-1",
        "pi",
        "1+2",
        "5-3",
        "4*7",
        "8/2",
        "7%3",
        "2**3",
        "(1+2)*3",
        "2*(3+4)",
        "(2+3)*(4+5)",
        "-5",
        "~5",
        "not false",
        "5&3",
        "5|2",
        "5^1",
        "5 xor 1",
        "3<5",
        "4==4",
        "1+2*3",
        "(1+2)*(3-1)",
        "2**3+4",
        "-(3+2)",
        "~(2+1)",
        "sin(0)",
        "cos(pi)",
        "tan(0)",
        "sqrt(4)",
        "exp(1)",
        "arcsin(1)",
        "arccos(1)",
        "arctan2(0,1)",
        "sin(cos(0))",
        "sqrt(1+3)",
        "exp(sin(0))",
        "(sin(pi/2)+cos(0))*2",
        "2**(1+2)",
        "((3+5)*2)/4",
        "a + 10",
        "b[1 + 1] * 4",
        "c + a * 10",
    ];

    let parser = ExpParser::new();

    for expr in expressions {
        let ast = parser.parse(expr).expect("Failed to parse expression");

        let mut env = Vec::new();
        let mut global_scope = HashMap::new();
        global_scope.insert("a".to_string(), Value::Int(10));
        global_scope.insert(
            "b".to_string(),
            Value::Array(vec![Value::Int(0), Value::Int(1), Value::Int(2)]),
        );
        env.push(global_scope);

        let reduced = reduce_exp(ast, &env);

        println!("{} => {:?}", expr, reduced);
    }
}
