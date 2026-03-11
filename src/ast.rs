#[derive(Debug, Clone)]
pub enum Exp {
    Int(i64),
    Float(f64),
    NamedConst(String),
    Lval(Lval),
    Unary(String, Box<Exp>),
    Binary(Box<Exp>, String, Box<Exp>),
    Builtin1(String, Box<Exp>),
    Builtin2(String, Box<Exp>, Box<Exp>),
}

#[derive(Debug, Clone)]
pub enum Lval {
    Var(String),
    Array(String, Box<Exp>),
}

#[derive(Debug, Clone)]
pub enum Statement {
    Assignment(Lval, Exp),
    QUpdate(QUpdate),
    ConditionalQUpdate(QUpdate, Lval),
    ProcedureCall(String, Vec<Lval>),
    Measure(Lval, Lval),
    If(Exp, Box<Statement>, Box<Statement>),
    While(Exp, Box<Statement>),
    Block(Vec<Declaration>, Vec<Statement>),
}

#[derive(Debug, Clone)]
pub enum Declaration {
    Uninit { ty: String, lval: Lval },
    InitScalar { ty: String, name: String, value: Exp },
    InitArray { ty: String, name: String, size: Exp, values: Vec<Exp> },
}

#[derive(Debug, Clone)]
pub enum QUpdate {
    Gate(Gate, Lval),
    Swap(Lval, Lval),
}

#[derive(Debug, Clone)]
pub enum Gate {
    Not,
    H,
    Rx(Exp),
    Ry(Exp),
    Rz(Exp),
    P(Exp)
}

#[derive(Debug, Clone)]
pub struct Procedure {
    pub name: String,
    pub params: Vec<ParameterDeclaration>,
    pub body: Statement,
}

#[derive(Debug, Clone)]
pub enum ParameterDeclaration {
    Scalar { ty: String, name: String },
    ArrayConst { ty: String, name: String, size: i64 },
    ArrayVar { ty: String, name: String, size: String },
}

#[derive(Debug, Clone)]
pub struct Program {
    pub procedures: Vec<Procedure>,
}