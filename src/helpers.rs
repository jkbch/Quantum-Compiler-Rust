use crate::ast::*;
use std::{collections::HashMap, f64::consts::PI};

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Array(Vec<Value>),
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

pub fn value_is_true(v: Value) -> bool {
    match v {
        Value::Int(i) => i != 0,
        Value::Float(f) => f != 0.0,
        Value::Bool(b) => b,
        _ => true, // default for other types (arrays, named constants, etc.)
    }
}

pub fn eval_const(s: &str) -> Value {
    match s {
        "pi" => Value::Float(PI),
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        _ => panic!("Unknown named constant: {}", s),
    }
}

/// Evaluate a unary operator
pub fn eval_unop(op: &str, x: Value) -> Value {
    match (op, &x) {
        ("-", Value::Int(i)) => Value::Int(-i),
        ("-", Value::Float(f)) => Value::Float(-f),
        ("~", Value::Int(i)) => Value::Int(!i),
        ("not", Value::Bool(b)) => Value::Bool(!b),
        _ => panic!("Unsupported unary operation: {} {:?}", op, x),
    }
}

/// Evaluate a binary operator
pub fn eval_binop(op: &str, lhs: Value, rhs: Value) -> Value {
    match (&lhs, &rhs) {
        (Value::Int(a), Value::Int(b)) => match op {
            "+" => Value::Int(a + b),
            "-" => Value::Int(a - b),
            "*" => Value::Int(a * b),
            "/" => Value::Int(a / b),
            "%" => Value::Int(a % b),
            "&" => Value::Int(a & b),
            "|" => Value::Int(a | b),
            "^" | "xor" => Value::Int(a ^ b),
            "<" => Value::Bool(a < b),
            "==" => Value::Bool(a == b),
            "**" => Value::Int(a.pow(*b as u32)),
            _ => panic!("Unsupported binary op {} for ints", op),
        },
        (Value::Float(a), Value::Float(b)) => match op {
            "+" => Value::Float(a + b),
            "-" => Value::Float(a - b),
            "*" => Value::Float(a * b),
            "/" => Value::Float(a / b),
            "**" => Value::Float(a.powf(*b)),
            _ => panic!("Unsupported binary op {} for floats", op),
        },
        (Value::Int(a), Value::Float(b)) => {
            eval_binop(op, Value::Float(*a as f64), Value::Float(*b))
        }
        (Value::Float(a), Value::Int(b)) => {
            eval_binop(op, Value::Float(*a), Value::Float(*b as f64))
        }
        _ => panic!("Unsupported binary op {} for {:?} and {:?}", op, lhs, rhs),
    }
}

/// Evaluate built-in functions
pub fn eval_fun_1(name: &str, arg: Value) -> Value {
    match (name, &arg) {
        ("sin", Value::Float(f)) => Value::Float(f.sin()),
        ("cos", Value::Float(f)) => Value::Float(f.cos()),
        ("tan", Value::Float(f)) => Value::Float(f.tan()),
        ("arcsin", Value::Float(f)) => Value::Float(f.asin()),
        ("arccos", Value::Float(f)) => Value::Float(f.acos()),
        ("exp", Value::Float(f)) => Value::Float(f.exp()),
        ("sqrt", Value::Float(f)) => Value::Float(f.sqrt()),
        (_, Value::Int(i)) => eval_fun_1(name, Value::Float(*i as f64)),
        _ => panic!("Unsupported function {} with arg {:?}", name, arg),
    }
}

pub fn eval_fun_2(name: &str, arg1: Value, arg2: Value) -> Value {
    match (name, &arg1, &arg2) {
        ("arctan2", Value::Float(y), Value::Float(x)) => Value::Float(y.atan2(*x)),
        (_, Value::Int(x), y) => eval_fun_2(name, Value::Float(*x as f64), y.clone()),
        (_, x, Value::Int(y)) => eval_fun_2(name, x.clone(), Value::Float(*y as f64)),

        _ => panic!(
            "Unsupported function {} with args {:?}, {:?}",
            name, arg1, arg2
        ),
    }
}

/// Checks if a Value is a constant (Int, Float, NamedConst)
pub fn is_constant(exp: &Exp) -> bool {
    matches!(exp, Exp::Int(_) | Exp::Float(_) | Exp::NamedConst(_))
}

/// Constructs a constant expression node from a Value
pub fn make_const_node(val: Value) -> Exp {
    match val {
        Value::Int(i) => Exp::Int(i),
        Value::Float(f) => Exp::Float(f),
        Value::Bool(b) => Exp::NamedConst(if b {
            "true".to_string()
        } else {
            "false".to_string()
        }),
        _ => panic!("Unsupported constant type in make_const_node: {:?}", val),
    }
}
