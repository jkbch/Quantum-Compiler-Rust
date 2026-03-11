use crate::ast::*;
use crate::helpers::*;
use crate::intrepret::interpret_exp;

/// Partial evaluator: reduce constant subexpressions in `exp`
pub fn reduce_exp(exp: &Exp, env: &Env) -> Exp {
    match exp {
        Exp::Int(_) | Exp::Float(_) | Exp::NamedConst(_) => exp.clone(),

        Exp::Lval(l) => {
            if let Some(val) = lookup_lval(l, env) {
                make_const_node(val)
            } else {
                exp.clone()
            }
        }

        Exp::Unary(op, e1) => {
            let reduced_e1 = reduce_exp(e1, env);
            if is_constant(&reduced_e1) {
                let val = interpret_exp(&reduced_e1, env).unwrap();
                make_const_node(eval_unop(op, val))
            } else {
                Exp::Unary(op.clone(), Box::new(reduced_e1))
            }
        }

        Exp::Binary(e1, op, e2) => {
            let left = reduce_exp(e1, env);
            let right = reduce_exp(e2, env);

            if is_constant(&left) && is_constant(&right) {
                let lval = interpret_exp(&left, env).unwrap();
                let rval = interpret_exp(&right, env).unwrap();
                make_const_node(eval_binop(op, lval, rval))
            } else {
                Exp::Binary(Box::new(left), op.clone(), Box::new(right))
            }
        }

        Exp::Builtin1(f, e1) => {
            let reduced_e1 = reduce_exp(e1, env);
            if is_constant(&reduced_e1) {
                let val = interpret_exp(&reduced_e1, env).unwrap();
                make_const_node(eval_fun_1(f, val))
            } else {
                Exp::Builtin1(f.clone(), Box::new(reduced_e1))
            }
        }

        Exp::Builtin2(f, e1, e2) => {
            let left = reduce_exp(e1, env);
            let right = reduce_exp(e2, env);

            if is_constant(&left) && is_constant(&right) {
                let lval = interpret_exp(&left, env).unwrap();
                let rval = interpret_exp(&right, env).unwrap();
                make_const_node(eval_fun_2(f, lval, rval))
            } else {
                Exp::Builtin2(f.clone(), Box::new(left), Box::new(right))
            }
        }
    }
}

pub fn reduce_declaration(d: &Declaration) -> Declaration {
    match d {
        Declaration::Uninit { ty, lval } => format!("{} {} ;", ty, show_lval(lval)),
        Declaration::InitScalar { ty, name, value } => {
            format!("{} {} = {} ;", ty, name, show_exp(value))
        }
        Declaration::InitArray {
            ty,
            name,
            size,
            values,
        } => {
            let vals = values.iter().map(show_exp).collect::<Vec<_>>().join(",");
            format!("{} {}[{}] = [{}] ;", ty, name, show_exp(size), vals)
        }
    }
}

pub fn reduce_lval(l: &Lval) -> Some(Value) {
    match l {
        Lval::Var(name) => lookup_val(name, env)
        Lval::Array(name, idx) => format!("{}[{}]", name, show_exp(idx)),
    }
}

pub fn reduce_qupdate(q: &QUpdate, env: &Env) -> QUpdate {
    match q {
        QUpdate::Gate(g, l) => QUpdate::Gate(reduce_gate(g, env), l.clone()),
        QUpdate::Swap(a, b) => QUpdate::Swap(a.clone(), b.clone()),
    }
}

pub fn reduce_gate(g: &Gate, env: &Env) -> Gate {
    match g {
        Gate::Not => Gate::Not,
        Gate::H => Gate::H,
        Gate::Rx(e) => Gate::Rx(reduce_exp(e, env)),
        Gate::Ry(e) => Gate::Ry(reduce_exp(e, env)),
        Gate::Rz(e) => Gate::Rz(reduce_exp(e, env)),
        Gate::P(e) => Gate::P(reduce_exp(e, env)),
    }
}
