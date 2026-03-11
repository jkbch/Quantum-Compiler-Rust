use crate::{ast::*, intrepret::interpret_exp};
use std::{collections::HashMap, f64::consts::PI};

#[derive(Debug, Clone)]
pub enum Scalar {
    Int(i64),
    Float(f64),
    Bool(bool),
}

#[derive(Debug, Clone)]
pub enum Array {
    Int(Vec<i64>),
    Float(Vec<f64>),
    Bool(Vec<bool>),
}

#[derive(Debug, Clone)]
pub enum Value {
    Scalar(Scalar),
    Array(Array),
}

pub type Env = Vec<HashMap<String, Value>>;

/// Lookup a variable or array element in the environment
pub fn lookup_val(name: &String, env: &Env) -> Option<Value> {
    for scope in env.iter().rev() {
        if let Some(val) = scope.get(name) {
            return Some(val.clone());
        }
    }
    None
}

/// Lookup a variable or array element in the environment
// pub fn lookup_lval(l: &Lval, env: &Env) -> Option<Value> {
//     match l {
//         Lval::Var(name) => {
//             for scope in env.iter().rev() {
//                 if let Some(val) = scope.get(name) {
//                     return Some(val.clone());
//                 }
//             }
//             None
//         }
//
//         Lval::Array(name, idx_exp) => {
//             // evaluate index
//             let idx = match interpret_exp(idx_exp, env)? {
//                 Value::Int(i) => i as usize,
//                 _ => return None,
//             };
//
//             for scope in env.iter().rev() {
//                 if let Some(Value::Array(arr)) = scope.get(name) {
//                     return arr.get(idx).cloned();
//                 }
//             }
//
//             None
//         }
//     }
// }

pub fn eval_const(s: &String) -> Value {
    match s.as_str() {
        "pi" => Value::Scalar(Scalar::Float(PI)),
        "true" => Value::Scalar(Scalar::Bool(true)),
        "false" => Value::Scalar(Scalar::Bool(false)),
        _ => panic!("Unknown named constant: {}", s),
    }
}

/// Evaluate a unary operator
pub fn eval_unop(op: &str, x: Value) -> Value {
    match (op, x) {
        ("-", Value::Scalar(Scalar::Int(i))) => Value::Scalar(Scalar::Int(-i)),
        ("-", Value::Scalar(Scalar::Float(f))) => Value::Scalar(Scalar::Float(-f)),
        ("~", Value::Scalar(Scalar::Int(i))) => Value::Scalar(Scalar::Int(!i)),
        ("not", Value::Scalar(Scalar::Bool(b))) => Value::Scalar(Scalar::Bool(!b)),
        _ => panic!("Unsupported unary operation"),
    }
}

/// Evaluate a binary operator
pub fn eval_binop(op: &str, lhs: Value, rhs: Value) -> Value {
    match (lhs, rhs) {
        (Value::Scalar(Scalar::Int(a)), Value::Scalar(Scalar::Int(b))) => match op {
            "+" => Value::Scalar(Scalar::Int(a + b)),
            "-" => Value::Scalar(Scalar::Int(a - b)),
            "*" => Value::Scalar(Scalar::Int(a * b)),
            "/" => Value::Scalar(Scalar::Int(a / b)),
            "%" => Value::Scalar(Scalar::Int(a % b)),
            "&" => Value::Scalar(Scalar::Int(a & b)),
            "|" => Value::Scalar(Scalar::Int(a | b)),
            "^" | "xor" => Value::Scalar(Scalar::Int(a ^ b)),
            "<" => Value::Scalar(Scalar::Bool(a < b)),
            "==" => Value::Scalar(Scalar::Bool(a == b)),
            "**" => Value::Scalar(Scalar::Int(a.pow(b as u32))),
            _ => panic!("Unsupported binary op {} for ints", op),
        },

        (Value::Scalar(Scalar::Float(a)), Value::Scalar(Scalar::Float(b))) => match op {
            "+" => Value::Scalar(Scalar::Float(a + b)),
            "-" => Value::Scalar(Scalar::Float(a - b)),
            "*" => Value::Scalar(Scalar::Float(a * b)),
            "/" => Value::Scalar(Scalar::Float(a / b)),
            "**" => Value::Scalar(Scalar::Float(a.powf(b))),
            _ => panic!("Unsupported binary op {} for floats", op),
        },

        (Value::Scalar(Scalar::Int(a)), Value::Scalar(Scalar::Float(b))) => eval_binop(
            op,
            Value::Scalar(Scalar::Float(a as f64)),
            Value::Scalar(Scalar::Float(b)),
        ),

        (Value::Scalar(Scalar::Float(a)), Value::Scalar(Scalar::Int(b))) => eval_binop(
            op,
            Value::Scalar(Scalar::Float(a)),
            Value::Scalar(Scalar::Float(b as f64)),
        ),

        (lhs, rhs) => panic!("Unsupported binary op {} for {:?} and {:?}", op, lhs, rhs),
    }
}

/// Evaluate built-in functions
pub fn eval_fun_1(name: &str, arg: Value) -> Value {
    match (name, arg) {
        ("sin", Value::Scalar(Scalar::Float(f))) => Value::Scalar(Scalar::Float(f.sin())),
        ("cos", Value::Scalar(Scalar::Float(f))) => Value::Scalar(Scalar::Float(f.cos())),
        ("tan", Value::Scalar(Scalar::Float(f))) => Value::Scalar(Scalar::Float(f.tan())),
        ("arcsin", Value::Scalar(Scalar::Float(f))) => Value::Scalar(Scalar::Float(f.asin())),
        ("arccos", Value::Scalar(Scalar::Float(f))) => Value::Scalar(Scalar::Float(f.acos())),
        ("exp", Value::Scalar(Scalar::Float(f))) => Value::Scalar(Scalar::Float(f.exp())),
        ("sqrt", Value::Scalar(Scalar::Float(f))) => Value::Scalar(Scalar::Float(f.sqrt())),

        (name, Value::Scalar(Scalar::Int(i))) => {
            eval_fun_1(name, Value::Scalar(Scalar::Float(i as f64)))
        }

        _ => panic!("Unsupported function {}", name),
    }
}

pub fn eval_fun_2(name: &str, arg1: Value, arg2: Value) -> Value {
    match (name, arg1, arg2) {
        ("arctan2", Value::Scalar(Scalar::Float(y)), Value::Scalar(Scalar::Float(x))) => {
            Value::Scalar(Scalar::Float(y.atan2(x)))
        }

        (name, Value::Scalar(Scalar::Int(x)), y) => {
            eval_fun_2(name, Value::Scalar(Scalar::Float(x as f64)), y)
        }

        (name, x, Value::Scalar(Scalar::Int(y))) => {
            eval_fun_2(name, x, Value::Scalar(Scalar::Float(y as f64)))
        }

        _ => panic!("Unsupported function {}", name),
    }
}

/// Checks if a Value is a constant (Int, Float, NamedConst)
pub fn is_constant(exp: &Exp) -> bool {
    matches!(exp, Exp::Int(_) | Exp::Float(_) | Exp::NamedConst(_))
}

/// Constructs a constant expression node from a Value
pub fn make_const_node(val: Value) -> Exp {
    match val {
        Value::Scalar(Scalar::Int(i)) => Exp::Int(i),

        Value::Scalar(Scalar::Float(f)) => Exp::Float(f),

        Value::Scalar(Scalar::Bool(b)) => Exp::NamedConst(if b {
            "true".to_string()
        } else {
            "false".to_string()
        }),

        _ => panic!("Unsupported constant type"),
    }
}
