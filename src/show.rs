use crate::ast::*;

pub fn show_program(p: &Program) -> String {
    p.procedures.iter().map(show_procedure).collect::<Vec<_>>().join("\n\n")
}

pub fn show_procedure(p: &Procedure) -> String {
    let params = p.params.iter().map(show_parameter_declaration).collect::<Vec<_>>().join(",");
    let body = show_statement(&p.body);
    format!("{}({}){}", p.name, params, body)
}

pub fn show_parameter_declaration(p: &ParameterDeclaration) -> String {
    match p {
        ParameterDeclaration::Scalar { ty, name } => format!("{} {}", ty, name),
        ParameterDeclaration::ArrayConst { ty, name, size } => format!("{} {}[{}]", ty, name, size),
        ParameterDeclaration::ArrayVar { ty, name, size } => format!("{} {}[{}]", ty, name, size),
    }
}

pub fn show_statement(s: &Statement) -> String {
    match s {
        Statement::Assignment(l, e) => format!("{} = {} ;", show_lval(l), show_exp(e)),
        Statement::QUpdate(q) => show_qupdate(q),
        Statement::ConditionalQUpdate(q, c) => format!("{} if {} ;", show_qupdate(q), show_lval(c)),
        Statement::ProcedureCall(name, args) => {
            let args = args.iter().map(show_lval).collect::<Vec<_>>().join(", ");
            format!("call '{}' ({}) ;", name, args)
        }
        Statement::Measure(q, c) => format!("measure {} -> {} ;", show_lval(q), show_lval(c)),
        Statement::If(e, t, f) => format!("if({})\n{}\nelse\n{}", show_exp(e), show_statement(t), show_statement(f)),
        Statement::While(e, body) => format!("while({})\n{}", show_exp(e), show_statement(body)),
        Statement::Block(decls, stats) => {
            let decls_s = decls.iter().map(show_declaration).collect::<Vec<_>>().join("\n");
            let stats_s = stats.iter().map(show_statement).collect::<Vec<_>>().join("\n");
            format!("{{\n{}\n{}\n}}\n", decls_s, stats_s)
        }
    }
}

pub fn show_exp(e: &Exp) -> String {
    match e {
        Exp::Int(v) => format!("{}", v),
        Exp::Float(v) => format!("{}", v),
        Exp::NamedConst(c) => c.clone(),
        Exp::Lval(l) => show_lval(l),
        Exp::Unary(op, e1) => format!("({} {})", op, show_exp(e1)),
        Exp::Binary(l, op, r) => format!("({} {} {})", show_exp(l), op, show_exp(r)),
        Exp::Builtin1(f, e1) => format!("{}({})", f, show_exp(e1)),
        Exp::Builtin2(f, e1, e2) => format!("{}({},{})", f, show_exp(e1), show_exp(e2)),
    }
}

pub fn show_declaration(d: &Declaration) -> String {
    match d {
        Declaration::Uninit { ty, lval } => format!("{} {} ;", ty, show_lval(lval)),
        Declaration::InitScalar { ty, name, value } => format!("{} {} = {} ;", ty, name, show_exp(value)),
        Declaration::InitArray { ty, name, size, values } => {
            let vals = values.iter().map(show_exp).collect::<Vec<_>>().join(",");
            format!("{} {}[{}] = [{}] ;", ty, name, show_exp(size), vals)
        }
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
        Gate::Rx(e) => format!("{}({})", "Rx", show_exp(e)),
        Gate::Ry(e) => format!("{}({})", "Ry", show_exp(e)),
        Gate::Rz(e) => format!("{}({})", "Rz", show_exp(e)),
        Gate::P(e) => format!("{}({})", "P", show_exp(e))
    }
}