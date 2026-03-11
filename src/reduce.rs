use crate::ast::*;
use crate::helpers::*;
use crate::intrepret::interpret_exp;

pub fn reduce_program(p: Program, env: &Env) -> Program {
    Program {
        procedures: p
            .procedures
            .into_iter()
            .map(|proc| reduce_procedure(proc, env))
            .collect(),
    }
}

pub fn reduce_procedure(p: Procedure, env: &Env) -> Procedure {
    Procedure {
        name: p.name,
        params: p.params,
        body: reduce_statement(p.body, env),
    }
}

pub fn reduce_parameter_declaration(p: ParameterDeclaration) -> ParameterDeclaration {
    p
}

pub fn reduce_statement(s: Statement, env: &Env) -> Statement {
    match s {
        Statement::Assignment(l, e) => {
            Statement::Assignment(reduce_lval(l, env), reduce_exp(e, env))
        }

        Statement::QUpdate(q) => Statement::QUpdate(reduce_qupdate(q, env)),

        Statement::ConditionalQUpdate(q, c) => {
            Statement::ConditionalQUpdate(reduce_qupdate(q, env), reduce_lval(c, env))
        }

        Statement::ProcedureCall(name, args) => Statement::ProcedureCall(
            name,
            args.into_iter().map(|a| reduce_lval(a, env)).collect(),
        ),

        Statement::Measure(q, c) => Statement::Measure(reduce_lval(q, env), reduce_lval(c, env)),

        Statement::If(e, t, f) => {
            let cond = reduce_exp(e, env);

            if is_constant(&cond) {
                let v = interpret_exp(cond, env).unwrap();
                if value_is_true(v) {
                    reduce_statement(*t, env)
                } else {
                    reduce_statement(*f, env)
                }
            } else {
                Statement::If(
                    cond,
                    Box::new(reduce_statement(*t, env)),
                    Box::new(reduce_statement(*f, env)),
                )
            }
        }

        Statement::While(e, body) => {
            Statement::While(reduce_exp(e, env), Box::new(reduce_statement(*body, env)))
        }

        Statement::Block(decls, stats) => Statement::Block(
            decls
                .into_iter()
                .map(|d| reduce_declaration(d, env))
                .collect(),
            stats
                .into_iter()
                .map(|s| reduce_statement(s, env))
                .collect(),
        ),
    }
}

pub fn reduce_exp(exp: Exp, env: &Env) -> Exp {
    match exp {
        Exp::Int(_) | Exp::Float(_) | Exp::NamedConst(_) => exp,

        Exp::Lval(l) => {
            let l = reduce_lval(l, env);

            if let Lval::Array(name, idx_exp) = &l
                && let Exp::Int(idx) = **idx_exp
                && let Some(Value::Array(arr)) = lookup_val(name, env)
                && let Some(val) = arr.get(idx as usize).cloned()
            {
                return make_const_node(val);
            }

            Exp::Lval(l)
        }

        Exp::Unary(op, e1) => {
            let e1 = reduce_exp(*e1, env);

            if is_constant(&e1) {
                let val = interpret_exp(e1, env).unwrap();
                make_const_node(eval_unop(&op, val))
            } else {
                Exp::Unary(op, Box::new(e1))
            }
        }

        Exp::Binary(e1, op, e2) => {
            let left = reduce_exp(*e1, env);
            let right = reduce_exp(*e2, env);

            if is_constant(&left) && is_constant(&right) {
                let lval = interpret_exp(left, env).unwrap();
                let rval = interpret_exp(right, env).unwrap();
                make_const_node(eval_binop(&op, lval, rval))
            } else {
                Exp::Binary(Box::new(left), op, Box::new(right))
            }
        }

        Exp::Builtin1(f, e1) => {
            let e1 = reduce_exp(*e1, env);

            if is_constant(&e1) {
                let val = interpret_exp(e1, env).unwrap();
                make_const_node(eval_fun_1(&f, val))
            } else {
                Exp::Builtin1(f, Box::new(e1))
            }
        }

        Exp::Builtin2(f, e1, e2) => {
            let left = reduce_exp(*e1, env);
            let right = reduce_exp(*e2, env);

            if is_constant(&left) && is_constant(&right) {
                let lval = interpret_exp(left, env).unwrap();
                let rval = interpret_exp(right, env).unwrap();
                make_const_node(eval_fun_2(&f, lval, rval))
            } else {
                Exp::Builtin2(f, Box::new(left), Box::new(right))
            }
        }
    }
}

pub fn reduce_declaration(d: Declaration, env: &Env) -> Declaration {
    match d {
        Declaration::Uninit { ty, lval } => Declaration::Uninit {
            ty,
            lval: reduce_lval(lval, env),
        },

        Declaration::InitScalar { ty, name, value } => Declaration::InitScalar {
            ty,
            name,
            value: reduce_exp(value, env),
        },

        Declaration::InitArray {
            ty,
            name,
            size,
            values,
        } => Declaration::InitArray {
            ty,
            name,
            size: reduce_exp(size, env),
            values: values.into_iter().map(|v| reduce_exp(v, env)).collect(),
        },
    }
}

pub fn reduce_lval(l: Lval, env: &Env) -> Lval {
    match l {
        Lval::Var(name) => Lval::Var(name),
        Lval::Array(name, idx) => Lval::Array(name, Box::new(reduce_exp(*idx, env))),
    }
}

pub fn reduce_qupdate(q: QUpdate, env: &Env) -> QUpdate {
    match q {
        QUpdate::Gate(g, l) => QUpdate::Gate(reduce_gate(g, env), reduce_lval(l, env)),
        QUpdate::Swap(a, b) => QUpdate::Swap(reduce_lval(a, env), reduce_lval(b, env)),
    }
}

pub fn reduce_gate(g: Gate, env: &Env) -> Gate {
    match g {
        Gate::Not => Gate::Not,
        Gate::H => Gate::H,
        Gate::Rx(e) => Gate::Rx(reduce_exp(e, env)),
        Gate::Ry(e) => Gate::Ry(reduce_exp(e, env)),
        Gate::Rz(e) => Gate::Rz(reduce_exp(e, env)),
        Gate::P(e) => Gate::P(reduce_exp(e, env)),
    }
}
