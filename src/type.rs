use crate::ast::*;
use crate::helpers::*;

pub fn typecheck_program(p: Program, env: &mut Env) -> Result<(), String> {
    for proc in p.procedures {
        typecheck_procedure(proc, env)?;
    }
    Ok(())
}

pub fn typecheck_procedure(p: Procedure, env: &mut Env) -> Result<(), String> {
    let mut local_env = env.clone();
    for param in p.params {
        let ty = type_of_parameter_declaration(&param)?;
        insert_type(&mut local_env, param_name(&param), ty);
    }
    typecheck_statement(p.body, &mut local_env)
}

pub fn type_of_parameter_declaration(p: &ParameterDeclaration) -> Result<Type, String> {
    match p {
        ParameterDeclaration::Scalar { ty, .. } => Ok(*ty),
        ParameterDeclaration::ArrayConst { ty, .. } => Ok(Type::Array(Box::new(*ty))),
        ParameterDeclaration::ArrayVar { ty, .. } => Ok(Type::Array(Box::new(*ty))),
    }
}

pub fn param_name(p: &ParameterDeclaration) -> String {
    match p {
        ParameterDeclaration::Scalar { name, .. } => name.clone(),
        ParameterDeclaration::ArrayConst { name, .. } => name.clone(),
        ParameterDeclaration::ArrayVar { name, .. } => name.clone(),
    }
}

pub fn typecheck_statement(s: Statement, env: &mut Env) -> Result<(), String> {
    match s {
        Statement::Assignment(l, e) => {
            let l_ty = typecheck_lval(l.clone(), env)?;
            if matches!(l_ty, Type::Qbit | Type::Array(box Type::Qbit)) {
                return Err("Cannot assign to a qbit".to_string());
            }

            let e_ty = typecheck_exp(e, env)?;
            if matches!(e_ty, Type::Qbit) {
                return Err("Cannot assign a qbit in classical assignment".to_string());
            }

            if !type_leq(e_ty, l_ty) {
                return Err(format!(
                    "Type mismatch in assignment: {} = {}",
                    show_type(l_ty),
                    show_type(e_ty)
                ));
            }
            Ok(())
        }

        Statement::QUpdate(q) => typecheck_qupdate(q, env),

        Statement::ConditionalQUpdate(q, c) => {
            let c_ty = typecheck_lval(c, env)?;
            if !matches!(c_ty, Type::Cbit) {
                return Err("Conditional qupdate must use a cbit".to_string());
            }
            typecheck_qupdate(q, env)
        }

        Statement::ProcedureCall(_, _) => Ok(()), // Procedure call signatures not checked here

        Statement::Measure(q, c) => {
            let q_ty = typecheck_lval(q, env)?;
            if !matches!(q_ty, Type::Qbit) {
                return Err("Measure source must be a qbit".to_string());
            }
            let c_ty = typecheck_lval(c, env)?;
            if !matches!(c_ty, Type::Cbit) {
                return Err("Measure target must be a cbit".to_string());
            }
            Ok(())
        }

        Statement::If(cond, t, f) => {
            let cond_ty = typecheck_exp(cond, env)?;
            if !matches!(cond_ty, Type::Cbit | Type::Int | Type::Float) {
                return Err("If condition must be classical scalar".to_string());
            }
            typecheck_statement(*t, env)?;
            typecheck_statement(*f, env)
        }

        Statement::While(cond, body) => {
            let cond_ty = typecheck_exp(cond, env)?;
            if !matches!(cond_ty, Type::Cbit | Type::Int | Type::Float) {
                return Err("While condition must be classical scalar".to_string());
            }
            typecheck_statement(*body, env)
        }

        Statement::Block(decls, stats) => {
            let mut local_env = env.clone();
            for d in decls {
                typecheck_declaration(d, &mut local_env)?;
            }
            for s in stats {
                typecheck_statement(s, &mut local_env)?;
            }
            Ok(())
        }
    }
}

pub fn typecheck_exp(e: Exp, env: &Env) -> Result<Type, String> {
    match e {
        Exp::Int(_) => Ok(Type::Int),
        Exp::Float(_) => Ok(Type::Float),
        Exp::NamedConst(_) => Ok(Type::Float),
        Exp::Lval(l) => {
            let ty = typecheck_lval(l, env)?;
            if matches!(ty, Type::Qbit) {
                Err("Qbits cannot appear in classical expressions".to_string())
            } else {
                Ok(ty)
            }
        }
        Exp::Unary(_, e1) => typecheck_exp(*e1, env),
        Exp::Binary(e1, _, e2) => {
            let t1 = typecheck_exp(*e1, env)?;
            let t2 = typecheck_exp(*e2, env)?;
            if !type_leq(t1, t2) && !type_leq(t2, t1) {
                return Err(format!("Binary operands incompatible: {} and {}", show_type(t1), show_type(t2)));
            }
            Ok(max_type(t1, t2))
        }
        Exp::Builtin1(_, e1) => typecheck_exp(*e1, env),
        Exp::Builtin2(_, e1, e2) => {
            let t1 = typecheck_exp(*e1, env)?;
            let t2 = typecheck_exp(*e2, env)?;
            if !type_leq(t1, t2) && !type_leq(t2, t1) {
                return Err(format!("Builtin2 operands incompatible: {} and {}", show_type(t1), show_type(t2)));
            }
            Ok(max_type(t1, t2))
        }
    }
}

pub fn typecheck_declaration(d: Declaration, env: &mut Env) -> Result<(), String> {
    match d {
        Declaration::Uninit { ty, lval } => {
            insert_type(env, lval_name(&lval), ty);
            Ok(())
        }
        Declaration::InitScalar { ty, name, value } => {
            let e_ty = typecheck_exp(value, env)?;
            if !type_leq(e_ty, ty) {
                return Err(format!("Type mismatch in declaration {}: {} vs {}", name, show_type(ty), show_type(e_ty)));
            }
            insert_type(env, name, ty);
            Ok(())
        }
        Declaration::InitArray { ty, name, size: _, values } => {
            for v in values {
                let v_ty = typecheck_exp(v, env)?;
                if !type_leq(v_ty, ty) {
                    return Err(format!("Array element type mismatch in {}: {}", name, show_type(v_ty)));
                }
            }
            insert_type(env, name, Type::Array(Box::new(ty)));
            Ok(())
        }
    }
}

pub fn typecheck_lval(l: Lval, env: &Env) -> Result<Type, String> {
    match l {
        Lval::Var(name) => env.get(&name).cloned().ok_or(format!("Unknown variable {}", name)),
        Lval::Array(name, _) => match env.get(&name) {
            Some(Type::Array(elem_ty)) => Ok(*elem_ty.clone()),
            Some(other) => Err(format!("Variable {} is not an array (found {})", name, show_type(*other.clone()))),
            None => Err(format!("Unknown array {}", name)),
        },
    }
}

pub fn typecheck_qupdate(q: QUpdate, env: &Env) -> Result<(), String> {
    match q {
        QUpdate::Gate(_, l) => {
            let l_ty = typecheck_lval(l, env)?;
            if !matches!(l_ty, Type::Qbit) {
                return Err("Gate target must be a qbit".to_string());
            }
            Ok(())
        }
        QUpdate::Swap(a, b) => {
            let a_ty = typecheck_lval(a, env)?;
            let b_ty = typecheck_lval(b, env)?;
            if !matches!(a_ty, Type::Qbit) || !matches!(b_ty, Type::Qbit) {
                return Err("Swap operands must be qbits".to_string());
            }
            Ok(())
        }
    }
}

pub fn lval_name(l: &Lval) -> String {
    match l {
        Lval::Var(n) => n.clone(),
        Lval::Array(n, _) => n.clone(),
    }
}

pub fn show_type(t: Type) -> String {
    match t {
        Type::Cbit => "Cbit".to_string(),
        Type::Int => "Int".to_string(),
        Type::Float => "Float".to_string(),
        Type::Qbit => "Qbit".to_string(),
        Type::Array(inner) => format!("Array[{}]", show_type(*inner)),
    }
}
