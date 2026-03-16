use crate::ast::*;
use crate::helpers::*;
use crate::intrepret::interpret_exp;
use crate::intrepret::interpret_lval;
use crate::vars::used_vars_stmt;

const MAX_UNROLL: usize = 5;

fn decl_name(d: &Declaration) -> &String {
    match d {
        Declaration::Uninit { lval, .. } => match lval {
            Lval::Var(name) => name,
            Lval::Array(name, _) => name,
        },
        Declaration::InitScalar { name, .. } => name,
        Declaration::InitArray { name, .. } => name,
    }
}

fn has_decl_collision(outer: &[Declaration], inner: &[Declaration]) -> bool {
    use std::collections::HashSet;

    let outer_names: HashSet<&String> = outer.iter().map(decl_name).collect();

    inner.iter().any(|d| outer_names.contains(decl_name(d)))
}

pub fn reduce_program(p: Program, val_env: Env<Value>) -> Program {
    let mut fun_env = FunEnv::new();
    let mut procedures = p.procedures;

    for _ in 0..2 {
        procedures = procedures
            .into_iter()
            .map(|proc| reduce_procedure(proc, &mut val_env.clone(), &mut fun_env))
            .collect();
    }

    Program {
        procedures: vec![procedures[0].clone()],
    }
}

pub fn reduce_procedure(p: Procedure, val_env: &mut Env<Value>, fun_env: &mut FunEnv) -> Procedure {
    // let names: Vec<String> = p
    //     .params
    //     .iter()
    //     .map(|p| match p {
    //         ParameterDeclaration::Scalar { name, .. } => name.clone(),
    //         ParameterDeclaration::ArrayConst { name, .. } => name.clone(),
    //         ParameterDeclaration::ArrayVar { name, .. } => name.clone(),
    //     })
    //     .collect();
    //
    // let entries: HashMap<String, Option<Value>> =
    //     names.iter().map(|name| (name.clone(), None)).collect();
    //
    // let push_scope = !entries.is_empty();
    // if push_scope {
    //     val_env.push_scope(entries);
    // }

    let body = reduce_statement(p.body.clone(), val_env, fun_env);

    // if push_scope {
    //     val_env.pop_scope();
    // }

    fun_env.insert(p.name.clone(), p.clone());

    Procedure {
        name: p.name,
        params: p.params,
        body,
    }
}

pub fn reduce_statement(s: Statement, val_env: &mut Env<Value>, fun_env: &FunEnv) -> Statement {
    match s {
        // Statement::ProcedureCall(name, args) => {
        //     let args = args.into_iter().map(|a| reduce_lval(a, val_env)).collect();
        //
        //     if let Some(fun) = fun_env.get(&name) {
        //         // todo rename varibles
        //         reduce_statement(fun.body.clone(), val_env, fun_env)
        //     } else {
        //         Statement::ProcedureCall(name, args)
        //     }
        // }
        Statement::ProcedureCall(name, args) => {
            let args: Vec<Lval> = args.into_iter().map(|a| reduce_lval(a, val_env)).collect();

            if let Some(proc) = fun_env.get(&name) {
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
                    reduce_statement(proc.body.clone(), val_env, fun_env)
                } else {
                    stats.push(proc.body.clone());
                    let block = Statement::Block(decls, stats);
                    reduce_statement(block, val_env, fun_env)
                }
            } else {
                Statement::ProcedureCall(name, args)
            }
        }

        Statement::Assignment(l, e) => {
            let l = reduce_lval(l, val_env);
            let e = reduce_exp(e, val_env);

            if is_const_node(&e) {
                let val = interpret_exp(&e, val_env).unwrap();

                match &l {
                    Lval::Var(name) => {
                        val_env.update(name, Value::Scalar(val));
                    }
                    Lval::Array(name, idx) => {
                        if let Some(idx) = interpret_exp(idx, val_env)
                            && let Some(Value::Array(arr)) = val_env.get_mut(name)
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
                    Lval::Var(name) => val_env.update_none(name),
                    Lval::Array(name, _) => val_env.update_none(name),
                };
            };

            Statement::Assignment(l, e)
        }

        Statement::QUpdate(q) => Statement::QUpdate(reduce_qupdate(q, val_env)),

        Statement::ConditionalQUpdate(q, c) => {
            let q = reduce_qupdate(q, val_env);
            let c = reduce_lval(c, val_env);
            Statement::ConditionalQUpdate(q, c)
        }

        Statement::Measure(q, c) => {
            let q = reduce_lval(q, val_env);
            let c = reduce_lval(c, val_env);
            Statement::Measure(q, c)
        }

        Statement::Block(decls, stats) => {
            // println!("{:?}", decls);
            // println!("{:?}", val_env);
            // println!(
            //     "{}",
            //     show_statement(&Statement::Block(decls.clone(), stats.clone()), 0)
            // );

            let push_scope = !decls.is_empty();

            if push_scope {
                val_env.push_empty_scope();
            }

            let mut flat_decls = Vec::new();
            let mut flat_stats = Vec::new();

            for d in decls {
                let reduced = reduce_declaration(d, val_env);
                flat_decls.push(reduced);
            }

            for st in stats {
                let reduced = reduce_statement(st, val_env, fun_env);

                match reduced {
                    Statement::Block(inner_decls, inner_stats) => {
                        if !has_decl_collision(&flat_decls, &inner_decls) {
                            flat_decls.extend(inner_decls);
                            flat_stats.extend(inner_stats);
                        } else {
                            flat_stats.push(Statement::Block(inner_decls, inner_stats));
                        }
                    }
                    _ => flat_stats.push(reduced),
                }
            }

            if push_scope {
                val_env.pop_scope();
            }

            let used_vars =
                used_vars_stmt(&Statement::Block(flat_decls.clone(), flat_stats.clone()));

            flat_decls.retain(|d| used_vars.contains(decl_name(d)));
            flat_stats.retain(|st| match st {
                Statement::Assignment(l, _) => match l {
                    Lval::Var(name) => used_vars.contains(name),
                    Lval::Array(name, _) => used_vars.contains(name),
                },
                _ => true,
            });

            if flat_decls.is_empty() && flat_stats.len() == 1 {
                return flat_stats.pop().unwrap();
            }

            Statement::Block(flat_decls, flat_stats)
        }

        Statement::If(e, t, f) => {
            let cond = reduce_exp(e, val_env);
            if is_const_node(&cond) {
                let v = interpret_exp(&cond, val_env).unwrap();
                match v {
                    Scalar::Cbit(true) => reduce_statement(*t, val_env, fun_env),
                    Scalar::Cbit(false) => reduce_statement(*f, val_env, fun_env),
                    _ => panic!("If condition must be boolean"),
                }
            } else {
                let t_res = reduce_statement(*t, val_env, fun_env);
                let f_res = reduce_statement(*f, val_env, fun_env);
                Statement::If(cond, Box::new(t_res), Box::new(f_res))
            }
        }

        Statement::While(e, body) => {
            let mut iter_env = val_env.clone();
            let mut iters = Vec::new();

            for _ in 0..MAX_UNROLL {
                let v = interpret_exp(&e, &iter_env);
                // println!("{:?}", v);
                match v {
                    Some(Scalar::Cbit(true)) => {
                        iters.push(reduce_statement(*body.clone(), &mut iter_env, fun_env));
                    }
                    Some(Scalar::Cbit(false)) => {
                        *val_env = iter_env;
                        return Statement::Block(Vec::new(), iters);
                    }
                    _ => break,
                }
            }

            Statement::While(e, body)
        }
    }
}

pub fn reduce_declaration(d: Declaration, value_env: &mut Env<Value>) -> Declaration {
    match d {
        Declaration::Uninit { ty, lval } => {
            let lval = reduce_lval(lval, value_env);
            match &lval {
                Lval::Var(name) => match ty {
                    Type::Cbit => {
                        value_env.insert(name.clone(), Value::Scalar(Scalar::Cbit(false)))
                    }
                    Type::Int => value_env.insert(name.clone(), Value::Scalar(Scalar::Int(0))),
                    Type::Float => {
                        value_env.insert(name.clone(), Value::Scalar(Scalar::Float(0.0)))
                    }
                    Type::Qbit => value_env.insert_none(name.clone()),
                },
                Lval::Array(name, e) => {
                    if let Some(idx) = interpret_exp(e, value_env) {
                        let n = scalar_to_usize(idx);
                        match ty {
                            Type::Cbit => value_env
                                .insert(name.clone(), Value::Array(Array::Cbit(vec![false; n]))),
                            Type::Int => {
                                value_env.insert(name.clone(), Value::Array(Array::Int(vec![0; n])))
                            }
                            Type::Float => value_env
                                .insert(name.clone(), Value::Array(Array::Float(vec![0.0; n]))),
                            Type::Qbit => value_env.insert_none(name.clone()),
                        }
                    } else {
                        match ty {
                            Type::Cbit => {
                                value_env.insert(name.clone(), Value::Array(Array::Cbit(vec![])))
                            }
                            Type::Int => {
                                value_env.insert(name.clone(), Value::Array(Array::Int(vec![])))
                            }
                            Type::Float => {
                                value_env.insert(name.clone(), Value::Array(Array::Float(vec![])))
                            }
                            Type::Qbit => value_env.insert_none(name.clone()),
                        }
                    }
                }
            }
            Declaration::Uninit { ty, lval }
        }

        Declaration::InitScalar { ty, name, value } => {
            let value = reduce_exp(value, value_env);
            if is_const_node(&value) {
                let val = interpret_exp(&value, value_env).unwrap();
                value_env.insert(name.clone(), Value::Scalar(scalar_to_scalar(val, ty)));
            } else {
                value_env.insert_none(name.clone());
            }
            Declaration::InitScalar { ty, name, value }
        }

        Declaration::InitArray {
            ty,
            name,
            size,
            values,
        } => {
            let size = reduce_exp(size, value_env);
            let exps: Vec<Exp> = values
                .into_iter()
                .map(|v| reduce_exp(v, value_env))
                .collect();

            if exps.iter().all(is_const_node) {
                let vals: Vec<Scalar> = exps
                    .iter()
                    .map(|v| interpret_exp(v, value_env).unwrap())
                    .collect();

                // todo handle array sizes

                value_env.insert(name.clone(), Value::Array(scalars_to_array(vals, ty)));
            } else {
                value_env.insert_none(name.clone());
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

pub fn reduce_exp(exp: Exp, value_env: &Env<Value>) -> Exp {
    match exp {
        Exp::Int(_) | Exp::Float(_) | Exp::NamedConst(_) => exp,

        Exp::Lval(l) => {
            let l = reduce_lval(l, value_env);
            if let Some(val) = interpret_lval(&l, value_env) {
                make_const_node(val)
            } else {
                Exp::Lval(l)
            }
        }

        Exp::Unary(op, e1) => {
            let e1 = reduce_exp(*e1, value_env);
            if is_const_node(&e1) {
                make_const_node(eval_unop(&op, interpret_exp(&e1, value_env).unwrap()))
            } else {
                Exp::Unary(op, Box::new(e1))
            }
        }

        Exp::Binary(e1, op, e2) => {
            let e1 = reduce_exp(*e1, value_env);
            let e2 = reduce_exp(*e2, value_env);
            if is_const_node(&e1) && is_const_node(&e2) {
                make_const_node(eval_binop(
                    &op,
                    interpret_exp(&e1, value_env).unwrap(),
                    interpret_exp(&e2, value_env).unwrap(),
                ))
            } else {
                Exp::Binary(Box::new(e1), op, Box::new(e2))
            }
        }

        Exp::Builtin1(f, e1) => {
            let e1 = reduce_exp(*e1, value_env);
            if is_const_node(&e1) {
                make_const_node(eval_fun_1(&f, interpret_exp(&e1, value_env).unwrap()))
            } else {
                Exp::Builtin1(f, Box::new(e1))
            }
        }

        Exp::Builtin2(f, e1, e2) => {
            let e1 = reduce_exp(*e1, value_env);
            let e2 = reduce_exp(*e2, value_env);
            if is_const_node(&e1) && is_const_node(&e2) {
                make_const_node(eval_fun_2(
                    &f,
                    interpret_exp(&e1, value_env).unwrap(),
                    interpret_exp(&e2, value_env).unwrap(),
                ))
            } else {
                Exp::Builtin2(f, Box::new(e1), Box::new(e2))
            }
        }
    }
}

pub fn reduce_lval(l: Lval, value_env: &Env<Value>) -> Lval {
    // todo scope prefix
    match l {
        Lval::Var(name) => Lval::Var(name),
        Lval::Array(name, idx) => {
            let idx = reduce_exp(*idx, value_env);
            Lval::Array(name, Box::new(idx))
        }
    }
}

pub fn reduce_qupdate(q: QUpdate, value_env: &Env<Value>) -> QUpdate {
    match q {
        QUpdate::Gate(g, l) => QUpdate::Gate(reduce_gate(g, value_env), reduce_lval(l, value_env)),
        QUpdate::Swap(a, b) => QUpdate::Swap(reduce_lval(a, value_env), reduce_lval(b, value_env)),
    }
}

pub fn reduce_gate(g: Gate, value_env: &Env<Value>) -> Gate {
    match g {
        Gate::Not => Gate::Not,
        Gate::H => Gate::H,
        Gate::Rx(e) => Gate::Rx(reduce_exp(e, value_env)),
        Gate::Ry(e) => Gate::Ry(reduce_exp(e, value_env)),
        Gate::Rz(e) => Gate::Rz(reduce_exp(e, value_env)),
        Gate::P(e) => Gate::P(reduce_exp(e, value_env)),
    }
}
