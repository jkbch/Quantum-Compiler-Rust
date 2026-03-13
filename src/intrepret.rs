use crate::ast::*;
use crate::cq::ExpParser;
use crate::helpers::*;
use crate::reduce::reduce_exp;
use std::collections::HashMap;

pub fn interpret_exp(e: Exp, env: &Env<Value>) -> Option<Value> {
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

pub fn interpret_lval(l: Lval, env: &Env<Value>) -> Option<Value> {
    match l {
        Lval::Var(name) => env.get(&name).cloned(),
        Lval::Array(name, idx_exp) => {
            let idx = match interpret_exp(*idx_exp, env)? {
                Value::Int(i) => i as usize,
                _ => return None,
            };

            if let Some(Value::Array(arr)) = env.get(&name) {
                arr.get(idx).cloned()
            } else {
                None
            }
        }
    }
}
