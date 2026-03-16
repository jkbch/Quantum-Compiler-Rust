use crate::ast::*;
use std::{collections::HashMap, f64::consts::PI};

#[derive(Debug, Clone, Copy)]
pub enum Scalar {
    Int(i64),
    Float(f64),
    Cbit(bool),
}

#[derive(Debug, Clone)]
pub enum Array {
    Int(Vec<i64>),
    Float(Vec<f64>),
    Cbit(Vec<bool>),
}

#[derive(Debug, Clone)]
pub enum Value {
    Scalar(Scalar),
    Array(Array),
}

pub struct Function {
    pub param_names: Vec<String>,
    pub body: Statement,
}
pub type FunEnv = HashMap<String, Function>;

pub fn scalar_to_usize(scalar: Scalar) -> usize {
    match scalar {
        Scalar::Cbit(b) => b as usize,
        Scalar::Int(i) => i as usize,
        Scalar::Float(f) => f as usize,
    }
}

pub fn scalar_to_bool(s: Scalar) -> bool {
    match s {
        Scalar::Cbit(b) => b,
        Scalar::Int(i) => i != 0,
        Scalar::Float(f) => f != 0.0,
    }
}

pub fn scalar_to_i64(s: Scalar) -> i64 {
    match s {
        Scalar::Cbit(b) => b as i64,
        Scalar::Int(i) => i,
        Scalar::Float(f) => f as i64,
    }
}

pub fn scalar_to_f64(s: Scalar) -> f64 {
    match s {
        Scalar::Cbit(b) => b as usize as f64,
        Scalar::Int(i) => i as f64,
        Scalar::Float(f) => f,
    }
}

pub fn scalar_to_scalar(s: Scalar, ty: Type) -> Scalar {
    match ty {
        Type::Cbit => Scalar::Cbit(scalar_to_bool(s)),
        Type::Int => Scalar::Int(scalar_to_i64(s)),
        Type::Float => Scalar::Float(scalar_to_f64(s)),
        Type::Qbit => panic!("Cannot cast classical scalar to qbit"),
    }
}

pub fn scalars_to_array(scalars: Vec<Scalar>, ty: Type) -> Array {
    match ty {
        Type::Cbit => Array::Cbit(scalars.into_iter().map(scalar_to_bool).collect()),
        Type::Int => Array::Int(scalars.into_iter().map(scalar_to_i64).collect()),
        Type::Float => Array::Float(scalars.into_iter().map(scalar_to_f64).collect()),
        Type::Qbit => panic!("Cannot create classical array from qbit type"),
    }
}

#[derive(Debug, Clone)]
pub struct Env<T> {
    scopes: Vec<HashMap<String, Option<T>>>,
}

impl<T> Env<T> {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    pub fn currect_scope(&self) -> usize {
        return self.scopes.len();
    }

    pub fn scope_prefix(&self, name: String) -> String {
        for (i, scope) in self.scopes.iter().enumerate().rev() {
            if scope.contains_key(&name) {
                return format!("{}{}", i, name);
            }
        }
        name
    }

    pub fn push_empty_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn push_scope(&mut self, entries: HashMap<String, Option<T>>) {
        self.scopes.push(entries);
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn insert(&mut self, name: String, val: T) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, Some(val));
        }
    }

    pub fn insert_none(&mut self, name: String) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, None);
        }
    }

    pub fn update(&mut self, name: &str, val: T) -> bool {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(entry) = scope.get_mut(name) {
                *entry = Some(val);
                return true;
            }
        }
        false
    }

    pub fn update_none(&mut self, name: &str) -> bool {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(entry) = scope.get_mut(name) {
                *entry = None;
                return true;
            }
        }
        false
    }

    pub fn get(&self, name: &str) -> Option<&T> {
        for scope in self.scopes.iter().rev() {
            if let Some(val) = scope.get(name).and_then(Option::as_ref) {
                return Some(val);
            }
        }
        None
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut T> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(val) = scope.get_mut(name).and_then(Option::as_mut) {
                return Some(val);
            }
        }
        None
    }

    pub fn contains(&self, name: &str) -> bool {
        for scope in self.scopes.iter().rev() {
            if scope.contains_key(name) {
                return true;
            }
        }
        false
    }

    pub fn remove(&mut self, name: &str) -> Option<T> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(val) = scope.remove(name).flatten() {
                return Some(val);
            }
        }
        None
    }
}

pub fn is_const_node(exp: &Exp) -> bool {
    matches!(exp, Exp::Int(_) | Exp::Float(_) | Exp::NamedConst(_))
}

pub fn make_const_node(val: Scalar) -> Exp {
    match val {
        Scalar::Int(i) => Exp::Int(i),
        Scalar::Float(f) => Exp::Float(f),
        Scalar::Cbit(b) => Exp::NamedConst(if b {
            "true".to_string()
        } else {
            "false".to_string()
        }),
    }
}

pub fn eval_const(s: &str) -> Scalar {
    match s {
        "pi" => Scalar::Float(PI),
        "true" => Scalar::Cbit(true),
        "false" => Scalar::Cbit(false),
        _ => panic!("Unknown named constant: {}", s),
    }
}

pub fn eval_unop(op: &str, x: Scalar) -> Scalar {
    match (op, &x) {
        ("-", Scalar::Int(i)) => Scalar::Int(-i),
        ("-", Scalar::Float(f)) => Scalar::Float(-f),
        ("~", Scalar::Int(i)) => Scalar::Int(!i),
        ("not", Scalar::Cbit(b)) => Scalar::Cbit(!b),
        _ => panic!("Unsupported unary operation: {} {:?}", op, x),
    }
}

pub fn eval_binop(op: &str, lhs: Scalar, rhs: Scalar) -> Scalar {
    match (&lhs, &rhs) {
        (Scalar::Int(a), Scalar::Int(b)) => match op {
            "+" => Scalar::Int(a + b),
            "-" => Scalar::Int(a - b),
            "*" => Scalar::Int(a * b),
            "/" => Scalar::Int(a / b),
            "%" => Scalar::Int(a % b),
            "&" => Scalar::Int(a & b),
            "|" => Scalar::Int(a | b),
            "^" | "xor" => Scalar::Int(a ^ b),
            "<" => Scalar::Cbit(a < b),
            "==" => Scalar::Cbit(a == b),
            "**" => Scalar::Int(a.pow(*b as u32)),
            _ => panic!("Unsupported binary op {} for ints", op),
        },
        (Scalar::Float(a), Scalar::Float(b)) => match op {
            "+" => Scalar::Float(a + b),
            "-" => Scalar::Float(a - b),
            "*" => Scalar::Float(a * b),
            "/" => Scalar::Float(a / b),
            "**" => Scalar::Float(a.powf(*b)),
            _ => panic!("Unsupported binary op {} for floats", op),
        },
        (Scalar::Int(a), Scalar::Float(b)) => {
            eval_binop(op, Scalar::Float(*a as f64), Scalar::Float(*b))
        }
        (Scalar::Float(a), Scalar::Int(b)) => {
            eval_binop(op, Scalar::Float(*a), Scalar::Float(*b as f64))
        }
        _ => panic!("Unsupported binary op {} for {:?} and {:?}", op, lhs, rhs),
    }
}

pub fn eval_fun_1(name: &str, arg: Scalar) -> Scalar {
    match (name, &arg) {
        ("sin", Scalar::Float(f)) => Scalar::Float(f.sin()),
        ("cos", Scalar::Float(f)) => Scalar::Float(f.cos()),
        ("tan", Scalar::Float(f)) => Scalar::Float(f.tan()),
        ("arcsin", Scalar::Float(f)) => Scalar::Float(f.asin()),
        ("arccos", Scalar::Float(f)) => Scalar::Float(f.acos()),
        ("exp", Scalar::Float(f)) => Scalar::Float(f.exp()),
        ("sqrt", Scalar::Float(f)) => Scalar::Float(f.sqrt()),
        (_, Scalar::Int(i)) => eval_fun_1(name, Scalar::Float(*i as f64)),
        _ => panic!("Unsupported function {} with arg {:?}", name, arg),
    }
}

pub fn eval_fun_2(name: &str, arg1: Scalar, arg2: Scalar) -> Scalar {
    match (name, &arg1, &arg2) {
        ("arctan2", Scalar::Float(y), Scalar::Float(x)) => Scalar::Float(y.atan2(*x)),
        (_, Scalar::Int(x), y) => eval_fun_2(name, Scalar::Float(*x as f64), y.clone()),
        (_, x, Scalar::Int(y)) => eval_fun_2(name, x.clone(), Scalar::Float(*y as f64)),

        _ => panic!(
            "Unsupported function {} with args {:?}, {:?}",
            name, arg1, arg2
        ),
    }
}
