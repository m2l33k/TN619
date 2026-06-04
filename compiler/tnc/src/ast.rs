//! The Abstract Syntax Tree — language-neutral (no trace of EN vs AR remains).

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Fn(FnDecl),
    Struct(StructDecl),
    Enum(EnumDecl),
    Impl(ImplBlock),
}

#[derive(Debug, Clone)]
pub struct ImplBlock {
    pub type_name: String,
    pub methods: Vec<MethodDecl>,
}

#[derive(Debug, Clone)]
pub struct MethodDecl {
    pub self_kind: SelfKind,
    pub func: FnDecl,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelfKind {
    /// No `self` parameter — an associated function, called `Type::name(..)`.
    None,
    /// `self` by value (consuming).
    Value,
    /// `&self` shared/read-only borrow. (`&mut self` is deferred until the
    /// reference model lands; `&self` is sound under value semantics.)
    Ref,
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: String,
    /// (field name, type name) pairs.
    pub fields: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<VariantDecl>,
}

#[derive(Debug, Clone)]
pub struct VariantDecl {
    pub name: String,
    /// Payload type names (empty = unit variant `Red`; `["int"]` = `Circle(int)`).
    pub payloads: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FnDecl {
    pub name: String,
    pub params: Vec<Param>,
    /// Declared return type name; `None` means unit `()`.
    pub ret: Option<String>,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: String,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    /// `let`/`var` binding. `mutable` distinguishes var from let.
    /// `ty_ann` is an optional explicit type annotation (`let x: int = ...`).
    Let { name: String, mutable: bool, ty_ann: Option<String>, init: Expr },
    Assign { name: String, value: Expr },
    While { cond: Expr, body: Block },
    For { var: String, start: Expr, end: Expr, body: Block },
    Return(Option<Expr>),
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64),
    Str(String),
    Bool(bool),
    Ident(String),
    Unary { op: UnOp, rhs: Box<Expr> },
    Binary { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
    Call { callee: String, args: Vec<Expr> },
    /// `if` is an EXPRESSION (yields a value). `els` is another Expr
    /// (either Expr::Block or a nested Expr::If) to support else-if chains.
    If { cond: Box<Expr>, then_b: Block, els: Option<Box<Expr>> },
    Block(Block),
    /// `User { name: ..., age: ... }`
    StructLit { name: String, fields: Vec<(String, Expr)> },
    /// `base.field`
    Field { base: Box<Expr>, field: String },
    /// A `Type::member(args)` / `Type::member` path. Resolved at check time to
    /// either enum construction (`Shape::Circle(2)`, `Color::Red`) or an
    /// associated-function call (`Point::new(3, 4)`).
    Path { ty: String, member: String, args: Vec<Expr> },
    /// `receiver.method(args)`
    MethodCall { receiver: Box<Expr>, method: String, args: Vec<Expr> },
    /// `match scrutinee { pat => expr, ... }` — an expression.
    Match { scrutinee: Box<Expr>, arms: Vec<MatchArm> },
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Expr,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    /// `_`
    Wildcard,
    /// A bare name: either a binding OR a unit variant — resolved at match time
    /// against the variant registry.
    Ident(String),
    Int(i64),
    Bool(bool),
    Str(String),
    /// `Circle(r)` or `Shape::Circle(r)` or `Shape::Red` — has `::` or `(...)`.
    Variant { enum_name: Option<String>, name: String, subs: Vec<Pattern> },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Rem,
    Eq, Ne, Lt, Le, Gt, Ge,
}
