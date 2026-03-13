use crate::ast::*;
use crate::helpers::*;
use crate::intrepret::interpret_exp;

const MAX_UNROLL: usize = 10;

pub fn reduce_program(p: Program, static_input: Env<Value>) -> Program {
    let procedures: Vec<Procedure> = p
        .procedures
        .into_iter()
        .map(|proc| reduce_procedure(proc, &mut static_input.clone()))
        .collect();

    Program { procedures }
}

pub fn reduce_procedure(p: Procedure, env: &mut Env<Value>) -> Procedure {
    let (body, _) = reduce_statement(p.body, env);

    Procedure {
        name: p.name,
        params: p.params,
        body,
    }
}

fn param_name(param: &ParameterDeclaration) -> String {
    match param {
        ParameterDeclaration::Scalar { name, .. } => name.clone(),
        ParameterDeclaration::ArrayConst { name, .. } => name.clone(),
        ParameterDeclaration::ArrayVar { name, .. } => name.clone(),
    }
}

pub fn reduce_statement(s: Statement, env: &mut Env<Value>) -> (Statement, bool) {
    match s {
        Statement::ProcedureCall(name, args) => {
            let args = args.into_iter().map(|a| reduce_lval(a, env).0).collect();
            (Statement::ProcedureCall(name, args), false)
        }

        Statement::Assignment(l, e) => {
            let (l, l_static) = reduce_lval(l, env);
            let (e, e_static) = reduce_exp(e, env);

            let stmt_static = e_static && l_static;

            if stmt_static {
                let val = interpret_exp(e.clone(), env).unwrap();
                match &l {
                    Lval::Var(name) => env.insert(name.clone(), val),
                    Lval::Array(name, idx_exp) => {
                        if let Exp::Int(idx) = **idx_exp
                            && let Some(Value::Array(arr)) = env.get_mut(name)
                            && let Some(slot) = arr.get_mut(idx as usize)
                        {
                            *slot = val;
                        }
                    }
                }
            }

            (Statement::Assignment(l, e), stmt_static)
        }

        Statement::QUpdate(q) => (Statement::QUpdate(reduce_qupdate(q, env)), false),

        Statement::ConditionalQUpdate(q, c) => {
            let q = reduce_qupdate(q, env);
            let (c, _) = reduce_lval(c, env);
            (Statement::ConditionalQUpdate(q, c), false)
        }

        Statement::Measure(q, c) => {
            let (q, _) = reduce_lval(q, env);
            let (c, _) = reduce_lval(c, env);
            (Statement::Measure(q, c), false)
        }

        Statement::Block(decls, stats) => {
            env.push_scope();

            let mut flat_decls = Vec::new();
            let mut flat_stats = Vec::new();
            let mut all_static = true;

            for d in decls {
                let (d_res, d_static) = reduce_declaration(d, env);
                all_static &= d_static;
                flat_decls.push(d_res);
            }

            for st in stats {
                let (st_res, st_static) = reduce_statement(st, env);
                all_static &= st_static;

                match st_res {
                    Statement::Block(inner_decls, inner_stats) => {
                        flat_decls.extend(inner_decls);
                        flat_stats.extend(inner_stats);
                    }
                    _ => flat_stats.push(st_res),
                }
            }

            env.pop_scope();

            if flat_decls.is_empty() && flat_stats.len() == 1 {
                return (flat_stats.pop().unwrap(), all_static);
            }

            (Statement::Block(flat_decls, flat_stats), all_static)
        }

        Statement::If(e, t, f) => {
            let (cond, cond_static) = reduce_exp(e, env);
            if cond_static {
                let v = interpret_exp(cond.clone(), env).unwrap();
                match v {
                    Value::Bool(true) => reduce_statement(*t, env),
                    Value::Bool(false) => reduce_statement(*f, env),
                    _ => panic!("If condition must be boolean"),
                }
            } else {
                let (t_res, _) = reduce_statement(*t, env);
                let (f_res, _) = reduce_statement(*f, env);
                (Statement::If(cond, Box::new(t_res), Box::new(f_res)), false)
            }
        }

        Statement::While(e, body) => {
            let mut test_env = env.clone();
            let mut res_body = Vec::new();
            let mut unroll_count = 0;

            loop {
                let (cond_exp, cond_static) = reduce_exp(e.clone(), &test_env);

                if !cond_static || unroll_count >= MAX_UNROLL {
                    let (body_res, _) = reduce_statement(*body.clone(), env);
                    return (Statement::While(cond_exp, Box::new(body_res)), false);
                }

                let v = interpret_exp(cond_exp.clone(), &test_env).unwrap();

                match v {
                    Value::Bool(true) => {
                        let (body_res, _) = reduce_statement(*body.clone(), &mut test_env);
                        res_body.push(body_res);
                        unroll_count += 1;
                    }
                    Value::Bool(false) => break,
                    _ => panic!("While condition must be boolean"),
                }
            }

            *env = test_env;
            (Statement::Block(Vec::new(), res_body), true)
        }
    }
}

pub fn reduce_declaration(d: Declaration, env: &mut Env<Value>) -> (Declaration, bool) {
    match d {
        Declaration::Uninit { ty, lval } => {
            let (lval, _) = reduce_lval(lval, env);
            (Declaration::Uninit { ty, lval }, false)
        }

        Declaration::InitScalar { ty, name, value } => {
            let (value, is_static) = reduce_exp(value, env);
            if is_static {
                let val = interpret_exp(value.clone(), env).unwrap();
                env.insert(name.clone(), val);
            }
            (Declaration::InitScalar { ty, name, value }, false)
        }

        Declaration::InitArray {
            ty,
            name,
            size,
            values,
        } => {
            let (size, _) = reduce_exp(size, env);

            let reduced_values: Vec<Exp> =
                values.into_iter().map(|v| reduce_exp(v, env).0).collect();

            if reduced_values
                .iter()
                .all(|v| matches!(v, Exp::Int(_) | Exp::Float(_) | Exp::NamedConst(_)))
            {
                let vals = reduced_values
                    .iter()
                    .map(|v| interpret_exp(v.clone(), env).unwrap())
                    .collect();

                env.insert(name.clone(), Value::Array(vals));
            }

            (
                Declaration::InitArray {
                    ty,
                    name,
                    size,
                    values: reduced_values,
                },
                false,
            )
        }
    }
}

pub fn reduce_exp(exp: Exp, env: &Env<Value>) -> (Exp, bool) {
    match exp {
        Exp::Int(_) | Exp::Float(_) | Exp::NamedConst(_) => (exp, true),

        Exp::Lval(l) => {
            let (l, l_static) = reduce_lval(l, env);

            if l_static {
                match &l {
                    Lval::Var(name) => {
                        if let Some(val) = env.get(name).cloned() {
                            return (make_const_node(val), true);
                        }
                    }
                    Lval::Array(name, idx_exp) => {
                        if let Exp::Int(idx) = **idx_exp
                            && let Some(Value::Array(arr)) = env.get(name)
                            && let Some(val) = arr.get(idx as usize).cloned()
                        {
                            return (make_const_node(val), true);
                        }
                    }
                }
            }

            (Exp::Lval(l), false)
        }

        Exp::Unary(op, e1) => {
            let (e1, s1) = reduce_exp(*e1, env);
            if s1 {
                let val = interpret_exp(e1.clone(), env).unwrap();
                (make_const_node(eval_unop(&op, val)), true)
            } else {
                (Exp::Unary(op, Box::new(e1)), false)
            }
        }

        Exp::Binary(e1, op, e2) => {
            let (left, s1) = reduce_exp(*e1, env);
            let (right, s2) = reduce_exp(*e2, env);
            if s1 && s2 {
                let lval = interpret_exp(left.clone(), env).unwrap();
                let rval = interpret_exp(right.clone(), env).unwrap();
                (make_const_node(eval_binop(&op, lval, rval)), true)
            } else {
                (Exp::Binary(Box::new(left), op, Box::new(right)), false)
            }
        }

        Exp::Builtin1(f, e1) => {
            let (e1, s1) = reduce_exp(*e1, env);
            if s1 {
                let val = interpret_exp(e1.clone(), env).unwrap();
                (make_const_node(eval_fun_1(&f, val)), true)
            } else {
                (Exp::Builtin1(f, Box::new(e1)), false)
            }
        }

        Exp::Builtin2(f, e1, e2) => {
            let (left, s1) = reduce_exp(*e1, env);
            let (right, s2) = reduce_exp(*e2, env);
            if s1 && s2 {
                let lval = interpret_exp(left.clone(), env).unwrap();
                let rval = interpret_exp(right.clone(), env).unwrap();
                (make_const_node(eval_fun_2(&f, lval, rval)), true)
            } else {
                (Exp::Builtin2(f, Box::new(left), Box::new(right)), false)
            }
        }
    }
}

pub fn reduce_lval(l: Lval, env: &Env<Value>) -> (Lval, bool) {
    match l {
        Lval::Var(name) => (Lval::Var(name), true),
        Lval::Array(name, idx) => {
            let (idx, idx_static) = reduce_exp(*idx, env);
            (Lval::Array(name, Box::new(idx)), idx_static)
        }
    }
}

pub fn reduce_qupdate(q: QUpdate, env: &Env<Value>) -> QUpdate {
    match q {
        QUpdate::Gate(g, l) => QUpdate::Gate(reduce_gate(g, env), reduce_lval(l, env).0),
        QUpdate::Swap(a, b) => QUpdate::Swap(reduce_lval(a, env).0, reduce_lval(b, env).0),
    }
}

pub fn reduce_gate(g: Gate, env: &Env<Value>) -> Gate {
    match g {
        Gate::Not => Gate::Not,
        Gate::H => Gate::H,
        Gate::Rx(e) => Gate::Rx(reduce_exp(e, env).0),
        Gate::Ry(e) => Gate::Ry(reduce_exp(e, env).0),
        Gate::Rz(e) => Gate::Rz(reduce_exp(e, env).0),
        Gate::P(e) => Gate::P(reduce_exp(e, env).0),
    }
}
