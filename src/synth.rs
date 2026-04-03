use crate::ast::*;
use crate::eval::eval_exp;
use crate::helper::*;
use std::collections::HashMap;
use std::f64::consts::{FRAC_PI_2, PI};

#[derive(Debug, Clone)]
pub enum Gate1 {
    X,
    SX,
    Rz(f64),
}

#[derive(Debug, Clone)]
pub enum Gate2 {
    CX,
}

#[derive(Debug, Clone)]
pub enum Op {
    Gate1(Gate1, usize),
    Gate2(Gate2, usize, usize),
    Measure(usize, usize),
}

pub struct Bits<'a> {
    bits: HashMap<String, usize>,
    next_bit: usize,
    env: &'a ValueEnv,
}

impl<'a> Bits<'a> {
    pub fn new(env: &'a ValueEnv) -> Self {
        Self {
            bits: HashMap::new(),
            next_bit: 0,
            env,
        }
    }

    pub fn insert_scalar(&mut self, name: String) {
        let b = self.next_bit;
        self.next_bit += 1;
        self.bits.insert(name, b);
    }

    pub fn insert_array(&mut self, name: String, size: usize) {
        let b = self.next_bit;
        self.next_bit += size;
        self.bits.insert(name, b);
    }

    pub fn insert_lval(&mut self, l: &Lval) {
        match l {
            Lval::Var(name) => self.insert_scalar(name.clone()),
            Lval::Array(name, size) => self.insert_array(
                name.clone(),
                scalar_to_usize(eval_exp(size, self.env).unwrap()),
            ),
        }
    }

    pub fn get_lval(&self, l: &Lval) -> usize {
        match l {
            Lval::Var(name) => self.bits[name],
            Lval::Array(name, idx) => {
                let i = scalar_to_usize(eval_exp(idx, self.env).unwrap());
                let start = self.bits[name];
                start + i
            }
        }
    }
}

pub fn synth_program(p: &Program, env: ValueEnv) -> Vec<Op> {
    let mut ops = Vec::new();
    let mut qbits = Bits::new(&env);
    let mut cbits = Bits::new(&env);

    for proc in &p.procedures {
        synth_statement(&proc.body, &mut ops, &mut qbits, &mut cbits, &env);
    }

    ops
}

pub fn synth_parameter_declaration(
    p: &ParameterDeclaration,
    qbits: &mut Bits,
    cbits: &mut Bits,
    env: &ValueEnv,
) {
    match p {
        ParameterDeclaration::Scalar { ty, name } => match ty {
            Type::Qbit => qbits.insert_scalar(name.clone()),
            Type::Cbit => cbits.insert_scalar(name.clone()),
            _ => {}
        },
        ParameterDeclaration::ArrayConst { ty, name, size } => match ty {
            Type::Qbit => qbits.insert_array(name.clone(), *size as usize),
            Type::Cbit => cbits.insert_array(name.clone(), *size as usize),
            _ => {}
        },
        ParameterDeclaration::ArrayVar { ty, name, size } => {
            let size_exp = Exp::NamedConst(size.clone());
            let size_val = eval_exp(&size_exp, env).unwrap(); // Evaluate to Value
            let size_usize = scalar_to_usize(size_val);

            match ty {
                Type::Qbit => qbits.insert_array(name.clone(), size_usize),
                Type::Cbit => cbits.insert_array(name.clone(), size_usize),
                _ => {}
            }
        }
    }
}

fn synth_statement(
    s: &Statement,
    ops: &mut Vec<Op>,
    qbits: &mut Bits,
    cbits: &mut Bits,
    env: &ValueEnv,
) {
    match s {
        Statement::Block(decls, stmts) => {
            for d in decls {
                synth_declaration(d, qbits, cbits, env);
            }
            for s in stmts {
                synth_statement(s, ops, qbits, cbits, env);
            }
        }

        Statement::QUpdate(q) => {
            synth_qupdate(q, ops, qbits, env);
        }

        Statement::ConditionalQUpdate(q, c) => {
            let c = qbits.get_lval(c);
            synth_cond_qupdate(q, c, ops, qbits, env);
        }

        Statement::Measure(q, c) => {
            let q = qbits.get_lval(q);
            let c = qbits.get_lval(c);
            ops.push(Op::Measure(q, c));
        }
        _ => {}
    }
}

pub fn synth_declaration(d: &Declaration, qbits: &mut Bits, cbits: &mut Bits, env: &ValueEnv) {
    match d {
        Declaration::Uninit { ty, lval } => match ty {
            Type::Qbit => qbits.insert_lval(lval),
            Type::Cbit => cbits.insert_lval(lval),
            _ => {}
        },
        Declaration::InitScalar { ty, name, value } => match ty {
            Type::Qbit => qbits.insert_scalar(name.clone()),
            Type::Cbit => cbits.insert_scalar(name.clone()),
            _ => {}
        },
        Declaration::InitArray {
            ty,
            name,
            size,
            values,
        } => {
            let size = scalar_to_usize(eval_exp(size, env).unwrap());
            match ty {
                Type::Qbit => qbits.insert_array(name.clone(), size),
                Type::Cbit => cbits.insert_array(name.clone(), size),
                _ => {}
            }
        }
    }
}

fn synth_qupdate(q: &QUpdate, ops: &mut Vec<Op>, qbits: &Bits, env: &ValueEnv) {
    match q {
        QUpdate::Gate(g, lval) => {
            let q = qbits.get_lval(lval);
            synth_gate(g, q, ops, env);
        }

        QUpdate::Swap(a, b) => {
            let qa = qbits.get_lval(a);
            let qb = qbits.get_lval(b);

            // SWAP = 3 CX
            ops.push(Op::Gate2(Gate2::CX, qa, qb));
            ops.push(Op::Gate2(Gate2::CX, qb, qa));
            ops.push(Op::Gate2(Gate2::CX, qa, qb));
        }
    }
}

fn synth_crz(theta: f64, c: usize, t: usize, ops: &mut Vec<Op>) {
    ops.push(Op::Gate1(Gate1::Rz(theta / 2.0), t));
    ops.push(Op::Gate2(Gate2::CX, c, t));
    ops.push(Op::Gate1(Gate1::Rz(-theta / 2.0), t));
    ops.push(Op::Gate2(Gate2::CX, c, t));
}

fn synth_cond_qupdate(q: &QUpdate, c: usize, ops: &mut Vec<Op>, qbits: &Bits, env: &ValueEnv) {
    match q {
        QUpdate::Gate(g, t) => {
            let t = qbits.get_lval(t);

            match g {
                Gate::Not => {
                    ops.push(Op::Gate2(Gate2::CX, c, t));
                }

                Gate::Rz(theta) | Gate::P(theta) => {
                    let theta = scalar_to_f64(eval_exp(theta, env).unwrap());
                    synth_crz(theta, c, t, ops);
                }

                Gate::Rx(theta) => {
                    let theta = scalar_to_f64(eval_exp(theta, env).unwrap());

                    synth_gate(&Gate::H, t, ops, env);
                    synth_crz(theta, c, t, ops);
                    synth_gate(&Gate::H, t, ops, env);
                }

                Gate::Ry(theta) => {
                    let theta = scalar_to_f64(eval_exp(theta, env).unwrap());

                    // U = Rz(pi/2) H
                    ops.push(Op::Gate1(Gate1::Rz(FRAC_PI_2), t));
                    synth_gate(&Gate::H, t, ops, env);

                    synth_crz(theta, c, t, ops);

                    // U†
                    synth_gate(&Gate::H, t, ops, env);
                    ops.push(Op::Gate1(Gate1::Rz(-FRAC_PI_2), t));
                }

                Gate::H => {
                    synth_gate(&Gate::H, t, ops, env);
                    ops.push(Op::Gate2(Gate2::CX, c, t));
                    synth_gate(&Gate::H, t, ops, env);
                }
            }
        }

        QUpdate::Swap(_, _) => {
            panic!("Controlled SWAP not implemented");
        }
    }
}

fn synth_gate(g: &Gate, q: usize, ops: &mut Vec<Op>, env: &ValueEnv) {
    match g {
        Gate::Not => {
            ops.push(Op::Gate1(Gate1::X, q));
        }

        Gate::H => {
            // H = Rz(pi) SX Rz(pi)
            ops.push(Op::Gate1(Gate1::Rz(PI), q));
            ops.push(Op::Gate1(Gate1::SX, q));
            ops.push(Op::Gate1(Gate1::Rz(PI), q));
        }

        Gate::Rz(theta) => {
            let val = scalar_to_f64(eval_exp(theta, env).unwrap());
            ops.push(Op::Gate1(Gate1::Rz(val), q));
        }

        Gate::P(theta) => {
            // P(theta) = Rz(theta) (global phase ignored)
            let val = scalar_to_f64(eval_exp(theta, env).unwrap());
            ops.push(Op::Gate1(Gate1::Rz(val), q));
        }

        Gate::Rx(theta) => {
            let val = scalar_to_f64(eval_exp(theta, env).unwrap());

            // Rx = H Rz H
            synth_gate(&Gate::H, q, ops, env);
            ops.push(Op::Gate1(Gate1::Rz(val), q));
            synth_gate(&Gate::H, q, ops, env);
        }

        Gate::Ry(theta) => {
            let val = scalar_to_f64(eval_exp(theta, env).unwrap());

            // Ry = Rz(pi/2) H Rz(theta) H Rz(-pi/2)
            ops.push(Op::Gate1(Gate1::Rz(FRAC_PI_2), q));
            synth_gate(&Gate::H, q, ops, env);
            ops.push(Op::Gate1(Gate1::Rz(val), q));
            synth_gate(&Gate::H, q, ops, env);
            ops.push(Op::Gate1(Gate1::Rz(-FRAC_PI_2), q));
        }
    }
}
