use crate::ast::*;
use crate::eval::{eval_exp, eval_lval};
use crate::helper::*;
use crate::vars::used_vars;
use std::collections::HashSet;

const MAX_UNROLL: usize = 1000;

pub fn part_eval_program(p: Program, mut env: ValueEnv) -> Program {
    let proc: Vec<Procedure> = p
        .procedures
        .into_iter()
        .enumerate()
        .map(|(i, proc)| part_eval_procedure(proc, &mut env.clone(), None, i == 0))
        .collect();

    let main = proc[0].clone();
    let map = proc.into_iter().map(|p| (p.name.clone(), p)).collect();

    Program {
        procedures: vec![part_eval_procedure(main, &mut env, Some(&map), true)],
    }
}

pub fn part_eval_procedure(
    p: Procedure,
    env: &mut ValueEnv,
    map: Option<&ProcMap>,
    is_main: bool,
) -> Procedure {
    if !is_main {
        let entries = p
            .params
            .iter()
            .map(|p| (param_decl_name(p).clone(), None))
            .collect();
        env.push_scope(entries);
    };

    let body = part_eval_statement(p.body.clone(), env, map);

    if !is_main {
        env.pop_scope();
    }

    Procedure {
        name: p.name,
        params: p.params,
        body,
    }
}

pub fn part_eval_statement(s: Statement, env: &mut ValueEnv, map: Option<&ProcMap>) -> Statement {
    match s {
        Statement::ProcedureCall(name, args) => {
            let args: Vec<Lval> = args.into_iter().map(|a| part_eval_lval(a, env)).collect();

            if let Some(map) = map {
                let proc = map
                    .get(&name)
                    .unwrap_or_else(|| panic!("Undefined procedure {}", name));

                let mut decls = Vec::new();
                let mut stats = Vec::new();

                for (param, arg) in proc.params.iter().zip(args) {
                    let param_name = match param {
                        ParameterDeclaration::Scalar { name, .. } => name,
                        ParameterDeclaration::ArrayConst { name, .. } => name,
                        ParameterDeclaration::ArrayVar { name, .. } => name,
                    };

                    let arg_name = match &arg {
                        Lval::Var(name) => name,
                        Lval::Array(name, _) => name,
                    };

                    if param_name != arg_name {
                        match param {
                            ParameterDeclaration::Scalar { ty, name } => {
                                decls.push(Declaration::InitScalar {
                                    ty: *ty,
                                    name: name.clone(),
                                    value: Exp::Lval(arg),
                                });
                            }

                            ParameterDeclaration::ArrayConst { ty, name, size } => {
                                decls.push(Declaration::Uninit {
                                    ty: *ty,
                                    lval: Lval::Array(name.clone(), Box::new(Exp::Int(*size))),
                                });

                                stats.push(Statement::Assignment(
                                    Lval::Var(name.clone()),
                                    Exp::Lval(arg),
                                ));
                            }

                            ParameterDeclaration::ArrayVar { ty, name, size } => {
                                decls.push(Declaration::Uninit {
                                    ty: *ty,
                                    lval: Lval::Array(
                                        name.clone(),
                                        Box::new(Exp::Lval(Lval::Var(size.clone()))),
                                    ),
                                });

                                stats.push(Statement::Assignment(
                                    Lval::Var(name.clone()),
                                    Exp::Lval(arg),
                                ));
                            }
                        }
                    }
                }

                if decls.is_empty() {
                    part_eval_statement(proc.body.clone(), env, Some(map))
                } else {
                    stats.push(proc.body.clone());
                    let block = Statement::Block(decls, stats);
                    part_eval_statement(block, env, Some(map))
                }
            } else {
                Statement::ProcedureCall(name, args)
            }
        }

        Statement::Assignment(l, e) => {
            let l = part_eval_lval(l, env);
            let e = part_eval_exp(e, env);

            if is_const_node(&e) {
                let val = eval_exp(&e, env).unwrap();

                match &l {
                    Lval::Var(name) => {
                        env.update(name, Some(Value::Scalar(val)));
                    }
                    Lval::Array(name, idx) => {
                        if let Some(idx) = eval_exp(idx, env)
                            && let Some(Value::Array(arr)) = env.get_mut(name)
                        {
                            let i = scalar_to_usize(idx);
                            match arr {
                                Array::Int(vec) => vec[i] = scalar_to_i64(val),
                                Array::Float(vec) => vec[i] = scalar_to_f64(val),
                                Array::Cbit(vec) => vec[i] = scalar_to_bool(val),
                            }
                        };
                    }
                }
            } else {
                match &l {
                    Lval::Var(name) => env.update(name, None),
                    Lval::Array(name, _) => env.update(name, None),
                };
            };

            Statement::Assignment(l, e)
        }

        Statement::QUpdate(q) => Statement::QUpdate(part_eval_qupdate(q, env)),

        Statement::ConditionalQUpdate(q, c) => {
            let q = part_eval_qupdate(q, env);
            let c = part_eval_lval(c, env);
            Statement::ConditionalQUpdate(q, c)
        }

        Statement::Measure(q, c) => {
            let q = part_eval_lval(q, env);
            let c = part_eval_lval(c, env);
            Statement::Measure(q, c)
        }

        Statement::Block(decls, stats) => {
            let push_scope = !decls.is_empty();
            if push_scope {
                env.push_empty_scope();
            }

            let mut flat_decls = Vec::new();
            let mut flat_stats = Vec::new();

            for d in decls {
                let reduced = part_eval_declaration(d, env);
                flat_decls.push(reduced);
            }
            let outer_names: HashSet<String> = flat_decls.iter().map(decl_name).cloned().collect();

            for st in stats {
                let reduced = part_eval_statement(st, env, map);

                match reduced {
                    Statement::Block(inner_decls, inner_stats) => {
                        let has_collision = inner_decls
                            .iter()
                            .any(|d| outer_names.contains(decl_name(d)));
                        if !has_collision {
                            flat_decls.extend(inner_decls);
                            flat_stats.extend(inner_stats);
                        } else {
                            flat_stats.push(Statement::Block(inner_decls, inner_stats));
                        }
                    }
                    Statement::Assignment(l, e) => {
                        if !is_const_node(&e) || map.is_none() {
                            flat_stats.push(Statement::Assignment(l, e))
                        }
                    }
                    _ => flat_stats.push(reduced),
                }
            }

            if push_scope {
                env.pop_scope();
            }

            if map.is_some() {
                let used_vars: HashSet<String> = flat_stats.iter().flat_map(used_vars).collect();
                flat_decls.retain(|d| used_vars.contains(decl_name(d)));
            }

            if flat_decls.is_empty() && flat_stats.len() == 1 {
                return flat_stats.pop().unwrap();
            }

            Statement::Block(flat_decls, flat_stats)
        }

        Statement::If(e, t, f) => {
            let cond = part_eval_exp(e, env);
            if is_const_node(&cond) {
                let v = eval_exp(&cond, env).unwrap();
                match v {
                    Scalar::Cbit(true) => part_eval_statement(*t, env, map),
                    Scalar::Cbit(false) => part_eval_statement(*f, env, map),
                    _ => panic!("If condition must be boolean"),
                }
            } else {
                let t_res = part_eval_statement(*t, env, map);
                let f_res = part_eval_statement(*f, env, map);
                Statement::If(cond, Box::new(t_res), Box::new(f_res))
            }
        }

        Statement::While(e, body) => {
            let mut iter_env = env.clone();
            let mut iters = Vec::new();

            for _ in 0..MAX_UNROLL {
                match eval_exp(&e, &iter_env) {
                    Some(Scalar::Cbit(true)) => {
                        iters.push(part_eval_statement(*body.clone(), &mut iter_env, map));
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

pub fn part_eval_declaration(d: Declaration, env: &mut ValueEnv) -> Declaration {
    match d {
        Declaration::Uninit { ty, lval } => {
            let lval = part_eval_lval(lval, env);
            match &lval {
                Lval::Var(name) => match ty {
                    Type::Cbit => {
                        env.insert(name.clone(), Some(Value::Scalar(Scalar::Cbit(false))))
                    }
                    Type::Int => env.insert(name.clone(), Some(Value::Scalar(Scalar::Int(0)))),
                    Type::Float => {
                        env.insert(name.clone(), Some(Value::Scalar(Scalar::Float(0.0))))
                    }
                    Type::Qbit => env.insert(name.clone(), None),
                },
                Lval::Array(name, e) => {
                    if let Some(idx) = eval_exp(e, env) {
                        let n = scalar_to_usize(idx);
                        match ty {
                            Type::Cbit => env.insert(
                                name.clone(),
                                Some(Value::Array(Array::Cbit(vec![false; n]))),
                            ),
                            Type::Int => {
                                env.insert(name.clone(), Some(Value::Array(Array::Int(vec![0; n]))))
                            }
                            Type::Float => env.insert(
                                name.clone(),
                                Some(Value::Array(Array::Float(vec![0.0; n]))),
                            ),
                            Type::Qbit => env.insert(name.clone(), None),
                        }
                    } else {
                        match ty {
                            Type::Cbit => {
                                env.insert(name.clone(), Some(Value::Array(Array::Cbit(vec![]))))
                            }
                            Type::Int => {
                                env.insert(name.clone(), Some(Value::Array(Array::Int(vec![]))))
                            }
                            Type::Float => {
                                env.insert(name.clone(), Some(Value::Array(Array::Float(vec![]))))
                            }
                            Type::Qbit => env.insert(name.clone(), None),
                        }
                    }
                }
            }
            Declaration::Uninit { ty, lval }
        }

        Declaration::InitScalar { ty, name, value } => {
            let value = part_eval_exp(value, env);
            if is_const_node(&value) {
                let val = eval_exp(&value, env).unwrap();
                env.insert(name.clone(), Some(Value::Scalar(scalar_to_scalar(val, ty))));
            } else {
                env.insert(name.clone(), None);
            }
            Declaration::InitScalar { ty, name, value }
        }

        Declaration::InitArray {
            ty,
            name,
            size,
            values,
        } => {
            let size = part_eval_exp(size, env);
            let exps: Vec<Exp> = values.into_iter().map(|v| part_eval_exp(v, env)).collect();

            if exps.iter().all(is_const_node) {
                let vals: Vec<Scalar> = exps.iter().map(|v| eval_exp(v, env).unwrap()).collect();

                // todo handle array sizes

                env.insert(name.clone(), Some(Value::Array(scalars_to_array(vals, ty))));
            } else {
                env.insert(name.clone(), None);
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

pub fn part_eval_exp(exp: Exp, env: &ValueEnv) -> Exp {
    match exp {
        Exp::Int(_) | Exp::Float(_) | Exp::NamedConst(_) => exp,

        Exp::Lval(l) => {
            let l = part_eval_lval(l, env);
            if let Some(val) = eval_lval(&l, env) {
                make_const_node(val)
            } else {
                Exp::Lval(l)
            }
        }

        Exp::Unary(op, e1) => {
            let e1 = part_eval_exp(*e1, env);
            if is_const_node(&e1) {
                make_const_node(eval_unop(&op, eval_exp(&e1, env).unwrap()))
            } else {
                Exp::Unary(op, Box::new(e1))
            }
        }

        Exp::Binary(e1, op, e2) => {
            let e1 = part_eval_exp(*e1, env);
            let e2 = part_eval_exp(*e2, env);
            if is_const_node(&e1) && is_const_node(&e2) {
                make_const_node(eval_binop(
                    &op,
                    eval_exp(&e1, env).unwrap(),
                    eval_exp(&e2, env).unwrap(),
                ))
            } else {
                Exp::Binary(Box::new(e1), op, Box::new(e2))
            }
        }

        Exp::Builtin1(f, e1) => {
            let e1 = part_eval_exp(*e1, env);
            if is_const_node(&e1) {
                make_const_node(eval_fun_1(&f, eval_exp(&e1, env).unwrap()))
            } else {
                Exp::Builtin1(f, Box::new(e1))
            }
        }

        Exp::Builtin2(f, e1, e2) => {
            let e1 = part_eval_exp(*e1, env);
            let e2 = part_eval_exp(*e2, env);
            if is_const_node(&e1) && is_const_node(&e2) {
                make_const_node(eval_fun_2(
                    &f,
                    eval_exp(&e1, env).unwrap(),
                    eval_exp(&e2, env).unwrap(),
                ))
            } else {
                Exp::Builtin2(f, Box::new(e1), Box::new(e2))
            }
        }
    }
}

pub fn part_eval_lval(l: Lval, env: &ValueEnv) -> Lval {
    match l {
        Lval::Var(name) => Lval::Var(name),
        Lval::Array(name, idx) => {
            let idx = part_eval_exp(*idx, env);
            Lval::Array(name, Box::new(idx))
        }
    }
}

pub fn part_eval_qupdate(q: QUpdate, env: &ValueEnv) -> QUpdate {
    match q {
        QUpdate::Gate(g, l) => QUpdate::Gate(part_eval_gate(g, env), part_eval_lval(l, env)),
        QUpdate::Swap(a, b) => QUpdate::Swap(part_eval_lval(a, env), part_eval_lval(b, env)),
    }
}

pub fn part_eval_gate(g: Gate, env: &ValueEnv) -> Gate {
    match g {
        Gate::Not => Gate::Not,
        Gate::H => Gate::H,
        Gate::Rx(e) => Gate::Rx(part_eval_exp(e, env)),
        Gate::Ry(e) => Gate::Ry(part_eval_exp(e, env)),
        Gate::Rz(e) => Gate::Rz(part_eval_exp(e, env)),
        Gate::P(e) => Gate::P(part_eval_exp(e, env)),
    }
}
