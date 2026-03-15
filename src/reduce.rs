use crate::ast::*;
use crate::helpers::*;
use crate::intrepret::interpret_exp;
use crate::intrepret::interpret_lval;

const MAX_UNROLL: usize = 5;

pub fn reduce_program(p: Program, static_input: Env<Value>) -> Program {
    let procedures: Vec<Procedure> = p
        .procedures
        .into_iter()
        .map(|proc| reduce_procedure(proc, &mut static_input.clone()))
        .collect();

    Program { procedures }
}

pub fn reduce_procedure(p: Procedure, env: &mut Env<Value>) -> Procedure {
    env.push_scope();
    let body = reduce_statement(p.body, env, true);
    env.pop_scope();

    Procedure {
        name: p.name,
        params: p.params,
        body,
    }
}

pub fn reduce_statement(s: Statement, env: &mut Env<Value>, push_scope: bool) -> Statement {
    match s {
        Statement::ProcedureCall(name, args) => {
            let args = args.into_iter().map(|a| reduce_lval(a, env)).collect();
            Statement::ProcedureCall(name, args)
        }

        Statement::Assignment(l, e) => {
            let l = reduce_lval(l, env);
            let e = reduce_exp(e, env);

            if is_const_node(&e) {
                let val = interpret_exp(&e, env).unwrap();

                match &l {
                    Lval::Var(name) => env.insert(name.clone(), Value::Scalar(val)),
                    Lval::Array(name, idx) => {
                        if let Some(idx) = interpret_exp(idx, env)
                            && let Some(Value::Array(arr)) = env.get_mut(name)
                        {
                            let i = scalar_to_usize(idx);
                            match arr {
                                Array::Int(vec) => vec[i] = scalar_to_i64(val),
                                Array::Float(vec) => vec[i] = scalar_to_f64(val),
                                Array::Cbit(vec) => vec[i] = scalar_to_bool(val),
                            }
                        }
                    }
                }
            } else {
                match &l {
                    Lval::Var(name) => env.remove(name),
                    Lval::Array(name, _) => env.remove(name),
                };
            }

            Statement::Assignment(l, e)
        }

        Statement::QUpdate(q) => Statement::QUpdate(reduce_qupdate(q, env)),

        Statement::ConditionalQUpdate(q, c) => {
            let q = reduce_qupdate(q, env);
            let c = reduce_lval(c, env);
            Statement::ConditionalQUpdate(q, c)
        }

        Statement::Measure(q, c) => {
            let q = reduce_lval(q, env);
            let c = reduce_lval(c, env);
            Statement::Measure(q, c)
        }

        Statement::Block(decls, stats) => {
            if push_scope {
                env.push_scope();
            }

            let mut flat_decls = Vec::new();
            let mut flat_stats = Vec::new();

            for d in decls {
                let reduced = reduce_declaration(d, env);
                flat_decls.push(reduced);
            }

            for st in stats {
                let reduced = reduce_statement(st, env, true);

                match reduced {
                    Statement::Block(inner_decls, inner_stats) => {
                        flat_decls.extend(inner_decls);
                        flat_stats.extend(inner_stats);
                    }
                    _ => flat_stats.push(reduced),
                }
            }

            if push_scope {
                env.pop_scope();
            }

            if flat_decls.is_empty() && flat_stats.len() == 1 {
                return flat_stats.pop().unwrap();
            }

            Statement::Block(flat_decls, flat_stats)
        }

        Statement::If(e, t, f) => {
            let cond = reduce_exp(e, env);
            if is_const_node(&cond) {
                let v = interpret_exp(&cond, env).unwrap();
                match v {
                    Scalar::Cbit(true) => reduce_statement(*t, env, push_scope),
                    Scalar::Cbit(false) => reduce_statement(*f, env, push_scope),
                    _ => panic!("If condition must be boolean"),
                }
            } else {
                let t_res = reduce_statement(*t, env, push_scope);
                let f_res = reduce_statement(*f, env, push_scope);
                Statement::If(cond, Box::new(t_res), Box::new(f_res))
            }
        }

        Statement::While(e, body) => {
            let mut iter_env = env.clone();
            let mut iters = Vec::new();

            for _ in 0..MAX_UNROLL {
                let v = interpret_exp(&e, &iter_env);
                // println!("{:?}", v);
                match v {
                    Some(Scalar::Cbit(true)) => {
                        iters.push(reduce_statement(*body.clone(), &mut iter_env, false));
                    }
                    Some(Scalar::Cbit(false)) => {
                        *env = iter_env;
                        return Statement::Block(Vec::new(), iters);
                    }
                    _ => break,
                }
            }

            Statement::While(e, body)
        }
    }
}

pub fn reduce_declaration(d: Declaration, env: &mut Env<Value>) -> Declaration {
    match d {
        Declaration::Uninit { ty, lval } => {
            let lval = reduce_lval(lval, env);
            match &lval {
                Lval::Var(name) => match ty {
                    Type::Cbit => env.insert(name.clone(), Value::Scalar(Scalar::Cbit(false))),
                    Type::Int => env.insert(name.clone(), Value::Scalar(Scalar::Int(0))),
                    Type::Float => env.insert(name.clone(), Value::Scalar(Scalar::Float(0.0))),
                    Type::Qbit => (),
                },
                Lval::Array(name, e) => {
                    if let Some(idx) = interpret_exp(e, env) {
                        let n = scalar_to_usize(idx);
                        match ty {
                            Type::Cbit => {
                                env.insert(name.clone(), Value::Array(Array::Cbit(vec![false; n])))
                            }
                            Type::Int => {
                                env.insert(name.clone(), Value::Array(Array::Int(vec![0; n])))
                            }
                            Type::Float => {
                                env.insert(name.clone(), Value::Array(Array::Float(vec![0.0; n])))
                            }
                            Type::Qbit => (),
                        }
                    } else {
                        match ty {
                            Type::Cbit => {
                                env.insert(name.clone(), Value::Array(Array::Cbit(vec![])))
                            }
                            Type::Int => env.insert(name.clone(), Value::Array(Array::Int(vec![]))),
                            Type::Float => {
                                env.insert(name.clone(), Value::Array(Array::Float(vec![])))
                            }
                            Type::Qbit => (),
                        }
                    }
                }
            }
            Declaration::Uninit { ty, lval }
        }

        Declaration::InitScalar { ty, name, value } => {
            let value = reduce_exp(value, env);
            if is_const_node(&value) {
                let val = interpret_exp(&value, env).unwrap();
                env.insert(name.clone(), Value::Scalar(scalar_to_scalar(val, ty)));
            } else {
                env.remove(&name);
            }
            Declaration::InitScalar { ty, name, value }
        }

        Declaration::InitArray {
            ty,
            name,
            size,
            values,
        } => {
            let size = reduce_exp(size, env);
            let exps: Vec<Exp> = values.into_iter().map(|v| reduce_exp(v, env)).collect();

            if exps.iter().all(is_const_node) {
                let vals: Vec<Scalar> = exps
                    .iter()
                    .map(|v| interpret_exp(v, env).unwrap())
                    .collect();

                // todo handle array sizes

                env.insert(name.clone(), Value::Array(scalars_to_array(vals, ty)));
            } else {
                env.remove(&name);
            }

            Declaration::InitArray {
                ty,
                name,
                size,
                values: exps,
            }
        }
    }
}

pub fn reduce_exp(exp: Exp, env: &Env<Value>) -> Exp {
    match exp {
        Exp::Int(_) | Exp::Float(_) | Exp::NamedConst(_) => exp,

        Exp::Lval(l) => {
            let l = reduce_lval(l, env);
            if let Some(val) = interpret_lval(&l, env) {
                make_const_node(val)
            } else {
                Exp::Lval(l)
            }
        }

        Exp::Unary(op, e1) => {
            let e1 = reduce_exp(*e1, env);
            if is_const_node(&e1) {
                make_const_node(eval_unop(&op, interpret_exp(&e1, env).unwrap()))
            } else {
                Exp::Unary(op, Box::new(e1))
            }
        }

        Exp::Binary(e1, op, e2) => {
            let e1 = reduce_exp(*e1, env);
            let e2 = reduce_exp(*e2, env);
            if is_const_node(&e1) && is_const_node(&e2) {
                make_const_node(eval_binop(
                    &op,
                    interpret_exp(&e1, env).unwrap(),
                    interpret_exp(&e2, env).unwrap(),
                ))
            } else {
                Exp::Binary(Box::new(e1), op, Box::new(e2))
            }
        }

        Exp::Builtin1(f, e1) => {
            let e1 = reduce_exp(*e1, env);
            if is_const_node(&e1) {
                make_const_node(eval_fun_1(&f, interpret_exp(&e1, env).unwrap()))
            } else {
                Exp::Builtin1(f, Box::new(e1))
            }
        }

        Exp::Builtin2(f, e1, e2) => {
            let e1 = reduce_exp(*e1, env);
            let e2 = reduce_exp(*e2, env);
            if is_const_node(&e1) && is_const_node(&e2) {
                make_const_node(eval_fun_2(
                    &f,
                    interpret_exp(&e1, env).unwrap(),
                    interpret_exp(&e2, env).unwrap(),
                ))
            } else {
                Exp::Builtin2(f, Box::new(e1), Box::new(e2))
            }
        }
    }
}

pub fn reduce_lval(l: Lval, env: &Env<Value>) -> Lval {
    match l {
        Lval::Var(name) => Lval::Var(name),
        Lval::Array(name, idx) => {
            let idx = reduce_exp(*idx, env);
            Lval::Array(name, Box::new(idx))
        }
    }
}

pub fn reduce_qupdate(q: QUpdate, env: &Env<Value>) -> QUpdate {
    match q {
        QUpdate::Gate(g, l) => QUpdate::Gate(reduce_gate(g, env), reduce_lval(l, env)),
        QUpdate::Swap(a, b) => QUpdate::Swap(reduce_lval(a, env), reduce_lval(b, env)),
    }
}

pub fn reduce_gate(g: Gate, env: &Env<Value>) -> Gate {
    match g {
        Gate::Not => Gate::Not,
        Gate::H => Gate::H,
        Gate::Rx(e) => Gate::Rx(reduce_exp(e, env)),
        Gate::Ry(e) => Gate::Ry(reduce_exp(e, env)),
        Gate::Rz(e) => Gate::Rz(reduce_exp(e, env)),
        Gate::P(e) => Gate::P(reduce_exp(e, env)),
    }
}
