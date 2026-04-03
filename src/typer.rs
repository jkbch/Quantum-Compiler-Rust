use crate::ast::*;
use crate::helper::*;

pub fn type_program(p: &Program) {
    let mut proc_env = ProcMap::new();

    for proc in &p.procedures {
        if proc_env.contains_key(&proc.name) {
            panic!("Duplicate procedure {}", proc.name);
        }
        proc_env.insert(proc.name.clone(), proc.clone());
    }

    for proc in &p.procedures {
        type_procedure(proc, &proc_env);
    }
}

fn type_procedure(p: &Procedure, procs: &ProcMap) {
    let mut env = TypeEnv::new();

    for param in &p.params {
        let name = param_decl_name(param).clone();

        let ty = match param {
            ParameterDeclaration::Scalar { ty, .. } => Ty::Scalar(*ty),
            ParameterDeclaration::ArrayConst { ty, size, .. } => Ty::Array(*ty, Some(*size)),
            ParameterDeclaration::ArrayVar { ty, size, .. } => {
                // size is symbolic → unknown at compile time
                env.insert(size.clone(), Ty::Scalar(Type::Int));
                Ty::Array(*ty, None)
            }
        };

        env.insert(name, ty);
    }

    type_statement(&p.body, &mut env, procs);
}

fn type_statement(s: &Statement, env: &mut TypeEnv, procs: &ProcMap) {
    match s {
        Statement::Assignment(l, e) => {
            let lt = type_lval(l, env);
            let et = type_exp(e, env);

            let l_scalar = match lt {
                Ty::Scalar(t) => t,
                _ => panic!("Assignment target must be scalar or array element"),
            };

            if l_scalar == Type::Qbit || et == Type::Qbit {
                panic!("Qbits not allowed in assignment");
            }

            if max_type(l_scalar, et) != l_scalar {
                panic!("Cannot assign {:?} to {:?}", et, l_scalar);
            }
        }

        Statement::QUpdate(q) => type_qupdate(q, env),

        Statement::ConditionalQUpdate(q, c) => {
            type_qupdate(q, env);

            match type_lval(c, env) {
                Ty::Scalar(Type::Qbit) => {}
                _ => panic!("Conditional QUpdate requires qbit"),
            }
        }

        Statement::ProcedureCall(name, args) => {
            let proc = procs
                .get(name)
                .unwrap_or_else(|| panic!("Unknown procedure {}", name));

            if proc.params.len() != args.len() {
                panic!("Argument count mismatch in call to {}", name);
            }

            for (param, arg) in proc.params.iter().zip(args) {
                let arg_ty = type_lval(arg, env);

                match param {
                    ParameterDeclaration::Scalar { ty, .. } => {
                        let arg_scalar = match arg_ty {
                            Ty::Scalar(t) => t,
                            _ => panic!("Expected scalar argument"),
                        };

                        if arg_scalar == Type::Qbit {
                            panic!("Qbit not allowed as scalar argument");
                        }

                        if max_type(*ty, arg_scalar) != *ty {
                            panic!("Scalar argument type mismatch");
                        }
                    }

                    ParameterDeclaration::ArrayConst { ty, size, .. } => {
                        let (elem_ty, arg_size) = match arg_ty {
                            Ty::Array(t, s) => (t, s),
                            _ => panic!("Expected array argument"),
                        };

                        if elem_ty != *ty {
                            panic!("ArrayConst type mismatch");
                        }

                        if let Some(s) = arg_size
                            && s != *size
                        {
                            panic!("ArrayConst size mismatch");
                        }
                    }

                    ParameterDeclaration::ArrayVar { ty, size, .. } => {
                        let (elem_ty, _) = match arg_ty {
                            Ty::Array(t, s) => (t, s),
                            _ => panic!("Expected array argument"),
                        };

                        if elem_ty != *ty {
                            panic!("ArrayVar type mismatch");
                        }

                        match env.get(size) {
                            Ty::Scalar(Type::Int) => {}
                            _ => panic!("ArrayVar size variable must be int"),
                        }
                    }
                }
            }
        }

        Statement::Measure(q, c) => {
            match type_lval(q, env) {
                Ty::Scalar(Type::Qbit) => {}
                _ => panic!("Measure requires qbit"),
            }

            match type_lval(c, env) {
                Ty::Scalar(Type::Cbit) => {}
                _ => panic!("Measure requires cbit"),
            }
        }

        Statement::If(e, t, f) => {
            if type_exp(e, env) != Type::Cbit {
                panic!("If condition must be cbit");
            }

            type_statement(t, env, procs);
            type_statement(f, env, procs);
        }

        Statement::While(e, body) => {
            if type_exp(e, env) != Type::Cbit {
                panic!("While condition must be cbit");
            }

            type_statement(body, env, procs);
        }

        Statement::Block(decls, stats) => {
            env.push_empty_scope();

            for d in decls {
                type_declaration(d, env);
            }

            for s in stats {
                type_statement(s, env, procs);
            }

            env.pop_scope();
        }
    }
}

fn type_declaration(d: &Declaration, env: &mut TypeEnv) {
    match d {
        Declaration::Uninit { ty, lval } => {
            let name = decl_name(d).clone();

            match lval {
                Lval::Var(_) => env.insert(name, Ty::Scalar(*ty)),
                Lval::Array(_, _) => env.insert(name, Ty::Array(*ty, None)),
            }
        }

        Declaration::InitScalar { ty, name, value } => {
            let vt = type_exp(value, env);

            if max_type(*ty, vt) != *ty {
                panic!("Invalid scalar initialization");
            }

            env.insert(name.clone(), Ty::Scalar(*ty));
        }

        Declaration::InitArray {
            ty,
            name,
            size,
            values,
        } => {
            let st = type_exp(size, env);
            if st != Type::Int {
                panic!("Array size must be int");
            }

            let size_val = match size {
                Exp::Int(i) => Some(*i),
                _ => None,
            };

            for v in values {
                let vt = type_exp(v, env);
                if max_type(*ty, vt) != *ty {
                    panic!("Array init type mismatch");
                }
            }

            env.insert(name.clone(), Ty::Array(*ty, size_val));
        }
    }
}

fn type_exp(e: &Exp, env: &TypeEnv) -> Type {
    match e {
        Exp::Int(_) => Type::Int,
        Exp::Float(_) => Type::Float,

        Exp::NamedConst(c) => match c.as_str() {
            "true" | "false" => Type::Cbit,
            "pi" => Type::Float,
            _ => panic!("Unknown constant"),
        },

        Exp::Lval(l) => match type_lval(l, env) {
            Ty::Scalar(t) => {
                if t == Type::Qbit {
                    panic!("Qbit not allowed in expressions");
                }
                t
            }
            Ty::Array(_, _) => panic!("Array not allowed in expressions"),
        },

        Exp::Unary(_, e1) => {
            let t = type_exp(e1, env);
            if t == Type::Qbit {
                panic!("Qbit not allowed");
            }
            t
        }

        Exp::Binary(l, op, r) => {
            let lt = type_exp(l, env);
            let rt = type_exp(r, env);

            if lt == Type::Qbit || rt == Type::Qbit {
                panic!("Qbit not allowed");
            }

            type_binop(op, lt, rt)
        }

        Exp::Builtin1(_, e1) => {
            if type_exp(e1, env) == Type::Qbit {
                panic!("Qbit not allowed");
            }
            Type::Float
        }

        Exp::Builtin2(_, e1, e2) => {
            if type_exp(e1, env) == Type::Qbit || type_exp(e2, env) == Type::Qbit {
                panic!("Qbit not allowed");
            }
            Type::Float
        }
    }
}

fn type_lval(l: &Lval, env: &TypeEnv) -> Ty {
    match l {
        Lval::Var(name) => *env.get(name),

        Lval::Array(name, idx) => {
            if type_exp(idx, env) != Type::Int {
                panic!("Array index must be int");
            }

            match env.get(name) {
                Ty::Array(t, _) => Ty::Scalar(*t),
                _ => panic!("Indexing non-array"),
            }
        }
    }
}

fn type_qupdate(q: &QUpdate, env: &TypeEnv) {
    match q {
        QUpdate::Gate(_, l) => match type_lval(l, env) {
            Ty::Scalar(Type::Qbit) => {}
            _ => panic!("Gate requires qbit"),
        },

        QUpdate::Swap(a, b) => {
            match type_lval(a, env) {
                Ty::Scalar(Type::Qbit) => {}
                _ => panic!("Swap requires qbit"),
            }
            match type_lval(b, env) {
                Ty::Scalar(Type::Qbit) => {}
                _ => panic!("Swap requires qbit"),
            }
        }
    }
}
