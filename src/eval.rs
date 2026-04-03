use crate::ast::*;
use crate::helper::*;

pub fn eval_exp(e: &Exp, env: &ValueEnv) -> Option<Scalar> {
    match e {
        Exp::Int(i) => Some(Scalar::Int(*i)),
        Exp::Float(f) => Some(Scalar::Float(*f)),
        Exp::NamedConst(s) => Some(eval_const(s)),
        Exp::Lval(l) => eval_lval(l, env),
        Exp::Unary(op, e1) => {
            let v = eval_exp(e1, env)?;
            Some(eval_unop(op, v))
        }
        Exp::Binary(e1, op, e2) => {
            let lhs = eval_exp(e1, env)?;
            let rhs = eval_exp(e2, env)?;
            Some(eval_binop(op, lhs, rhs))
        }
        Exp::Builtin1(f, e1) => {
            let v = eval_exp(e1, env)?;
            Some(eval_fun_1(f, v))
        }
        Exp::Builtin2(f, e1, e2) => {
            let lhs = eval_exp(e1, env)?;
            let rhs = eval_exp(e2, env)?;
            Some(eval_fun_2(f, lhs, rhs))
        }
    }
}

pub fn eval_lval(l: &Lval, env: &ValueEnv) -> Option<Scalar> {
    match l {
        Lval::Var(name) => match env.get(name) {
            &Some(Value::Scalar(val)) => Some(val),
            _ => None,
        },
        Lval::Array(name, idx_exp) => {
            let idx = scalar_to_usize(eval_exp(idx_exp, env)?);
            match env.get(name) {
                Some(Value::Array(Array::Int(arr))) => arr.get(idx).cloned().map(Scalar::Int),
                Some(Value::Array(Array::Float(arr))) => arr.get(idx).cloned().map(Scalar::Float),
                Some(Value::Array(Array::Cbit(arr))) => arr.get(idx).cloned().map(Scalar::Cbit),
                _ => None,
            }
        }
    }
}
