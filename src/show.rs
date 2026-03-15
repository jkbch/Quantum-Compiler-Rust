use crate::ast::*;

const INDENT: &str = "  "; // two spaces per level

pub fn show_program(p: &Program) -> String {
    p.procedures
        .iter()
        .map(|proc| show_procedure(proc, 0))
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub fn show_procedure(p: &Procedure, depth: usize) -> String {
    let params = p
        .params
        .iter()
        .map(show_parameter_declaration)
        .collect::<Vec<_>>()
        .join(", ");

    // Ensure top-level body is always a block
    let body_str = match &p.body {
        Statement::Block(_, _) => show_statement(&p.body, depth),
        other => format!("{{\n{}\n}}", show_statement(other, depth + 1)),
    };

    format!("{}({}){}", p.name, params, body_str)
}

pub fn show_parameter_declaration(p: &ParameterDeclaration) -> String {
    match p {
        ParameterDeclaration::Scalar { ty, name } => format!("{} {}", show_type(ty), name),
        ParameterDeclaration::ArrayConst { ty, name, size } => {
            format!("{} {}[{}]", show_type(ty), name, size)
        }
        ParameterDeclaration::ArrayVar { ty, name, size } => {
            format!("{} {}[{}]", show_type(ty), name, size)
        }
    }
}

pub fn show_statement(s: &Statement, depth: usize) -> String {
    let prefix = INDENT.repeat(depth);
    match s {
        Statement::Assignment(l, e) => format!("{}{} = {} ;", prefix, show_lval(l), show_exp(e)),
        Statement::QUpdate(q) => format!("{}{} ;", prefix, show_qupdate(q)),
        Statement::ConditionalQUpdate(q, c) => {
            format!("{}{} if {} ;", prefix, show_qupdate(q), show_lval(c))
        }
        Statement::ProcedureCall(name, args) => {
            let args_s = args.iter().map(show_lval).collect::<Vec<_>>().join(", ");
            format!("{}call '{}' ({}) ;", prefix, name, args_s)
        }
        Statement::Measure(q, c) => {
            format!("{}measure {} -> {} ;", prefix, show_lval(q), show_lval(c))
        }
        Statement::If(e, t, f) => {
            let t_s = show_statement(t, depth + 1);
            let f_s = show_statement(f, depth + 1);
            format!(
                "{}if({}) {{\n{}\n{}}} else {{\n{}\n{}}}",
                prefix,
                show_exp(e),
                t_s,
                prefix,
                f_s,
                prefix
            )
        }
        Statement::While(e, body) => {
            let body_s = show_statement(body, depth + 1);
            format!(
                "{}while({}) {{\n{}\n{}}}",
                prefix,
                show_exp(e),
                body_s,
                prefix
            )
        }
        Statement::Block(decls, stats) => {
            let mut parts = Vec::new();
            for decl in decls {
                parts.push(format!(
                    "{}{}",
                    INDENT.repeat(depth + 1),
                    show_declaration(decl)
                ));
            }
            for stat in stats {
                parts.push(show_statement(stat, depth + 1));
            }
            format!("{}{{\n{}\n{}}}", prefix, parts.join("\n"), prefix)
        }
    }
}

pub fn show_declaration(d: &Declaration) -> String {
    match d {
        Declaration::Uninit { ty, lval } => format!("{} {} ;", show_type(ty), show_lval(lval)),
        Declaration::InitScalar { ty, name, value } => {
            format!("{} {} = {} ;", show_type(ty), name, show_exp(value))
        }
        Declaration::InitArray {
            ty,
            name,
            size,
            values,
        } => {
            let vals_s = values.iter().map(show_exp).collect::<Vec<_>>().join(", ");
            format!(
                "{} {}[{}] = [{}] ;",
                show_type(ty),
                name,
                show_exp(size),
                vals_s
            )
        }
    }
}

pub fn show_exp(e: &Exp) -> String {
    match e {
        Exp::Int(v) => v.to_string(),
        Exp::Float(v) => v.to_string(),
        Exp::NamedConst(c) => c.clone(),
        Exp::Lval(l) => show_lval(l),
        Exp::Unary(op, e1) => format!("({} {})", op, show_exp(e1)),
        Exp::Binary(l, op, r) => format!("({} {} {})", show_exp(l), op, show_exp(r)),
        Exp::Builtin1(f, e1) => format!("{}({})", f, show_exp(e1)),
        Exp::Builtin2(f, e1, e2) => format!("{}({},{})", f, show_exp(e1), show_exp(e2)),
    }
}

pub fn show_lval(l: &Lval) -> String {
    match l {
        Lval::Var(name) => name.clone(),
        Lval::Array(name, idx) => format!("{}[{}]", name, show_exp(idx)),
    }
}

pub fn show_qupdate(q: &QUpdate) -> String {
    match q {
        QUpdate::Gate(g, l) => format!("{} {}", show_gate(g), show_lval(l)),
        QUpdate::Swap(a, b) => format!("{} <> {}", show_lval(a), show_lval(b)),
    }
}

pub fn show_gate(g: &Gate) -> String {
    match g {
        Gate::Not => "not".to_string(),
        Gate::H => "H".to_string(),
        Gate::Rx(e) => format!("Rx({})", show_exp(e)),
        Gate::Ry(e) => format!("Ry({})", show_exp(e)),
        Gate::Rz(e) => format!("Rz({})", show_exp(e)),
        Gate::P(e) => format!("P({})", show_exp(e)),
    }
}

pub fn show_type(t: &Type) -> String {
    match t {
        Type::Int => "int".to_string(),
        Type::Float => "float".to_string(),
        Type::Cbit => "cbit".to_string(),
        Type::Qbit => "qbit".to_string(),
    }
}
