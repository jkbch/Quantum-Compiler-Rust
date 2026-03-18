use crate::ast::*;
use std::collections::HashSet;

fn collect_lval_vars(l: &Lval, used: &mut HashSet<String>) {
    match l {
        Lval::Var(name) => {
            used.insert(name.clone());
        }
        Lval::Array(name, idx) => {
            used.insert(name.clone());
            collect_exp_vars(idx, used);
        }
    }
}

fn collect_exp_vars(e: &Exp, used: &mut HashSet<String>) {
    match e {
        Exp::Int(_) | Exp::Float(_) | Exp::NamedConst(_) => {}
        Exp::Lval(l) => collect_lval_vars(l, used),
        Exp::Unary(_, e1) => collect_exp_vars(e1, used),
        Exp::Binary(e1, _, e2) => {
            collect_exp_vars(e1, used);
            collect_exp_vars(e2, used);
        }
        Exp::Builtin1(_, e1) => collect_exp_vars(e1, used),
        Exp::Builtin2(_, e1, e2) => {
            collect_exp_vars(e1, used);
            collect_exp_vars(e2, used);
        }
    }
}

pub fn used_vars(s: &Statement) -> HashSet<String> {
    let mut used = HashSet::new();
    match s {
        Statement::Assignment(l, e) => {
            collect_lval_vars(l, &mut used);
            collect_exp_vars(e, &mut used);
        }
        Statement::Block(_, stats) => {
            for st in stats {
                used.extend(used_vars(st));
            }
        }
        Statement::If(cond, t, f) => {
            collect_exp_vars(cond, &mut used);
            used.extend(used_vars(t));
            used.extend(used_vars(f));
        }
        Statement::While(cond, body) => {
            collect_exp_vars(cond, &mut used);
            used.extend(used_vars(body));
        }
        Statement::ProcedureCall(_, args) => {
            for arg in args {
                collect_lval_vars(arg, &mut used);
            }
        }
        Statement::QUpdate(q) => match q {
            QUpdate::Gate(_, l) => collect_lval_vars(l, &mut used),
            QUpdate::Swap(a, b) => {
                collect_lval_vars(a, &mut used);
                collect_lval_vars(b, &mut used);
            }
        },
        Statement::ConditionalQUpdate(_, l) => collect_lval_vars(l, &mut used),
        Statement::Measure(q, c) => {
            collect_lval_vars(q, &mut used);
            collect_lval_vars(c, &mut used);
        }
    }
    used
}
