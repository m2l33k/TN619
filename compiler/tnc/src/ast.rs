//! The Abstract Syntax Tree — language-neutral (no trace of EN/AR/FR remains).

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}

/// A type as written in source. Structured (not a bare string) so composite
/// types like `[int]` and `Result<int, str>` nest.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    /// A named type: primitive (`int`/`عدد`/`entier`) or user-defined.
    Name(String),
    /// `[T]` — a growable array of `T`.
    Array(Box<TypeExpr>),
    /// `Result<T, E>` (any surface spelling of `Result`).
    Result(Box<TypeExpr>, Box<TypeExpr>),
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
    /// `&self` shared/read-only borrow.
    Ref,
    /// `&mut self` — the method mutates the receiver in place; callable only
    /// on a mutable binding.
    MutRef,
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: String,
    /// (field name, field type) pairs.
    pub fields: Vec<(String, TypeExpr)>,
}

#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<VariantDecl>,
}

#[derive(Debug, Clone)]
pub struct VariantDecl {
    pub name: String,
    /// Payload types (empty = unit variant `Red`; `[int]` = `Circle(int)`).
    pub payloads: Vec<TypeExpr>,
}

#[derive(Debug, Clone)]
pub struct FnDecl {
    pub name: String,
    pub params: Vec<Param>,
    /// Declared return type; `None` means unit `()`.
    pub ret: Option<TypeExpr>,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: TypeExpr,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    /// `let`/`var` binding. `mutable` distinguishes var from let.
    /// `ty_ann` is an optional explicit type annotation (`let x: int = ...`).
    Let {
        name: String,
        mutable: bool,
        ty_ann: Option<TypeExpr>,
        init: Expr,
    },
    Assign {
        target: AssignTarget,
        value: Expr,
    },
    While {
        cond: Expr,
        body: Block,
    },
    /// `for i in a..b` — integer range iteration.
    For {
        var: String,
        start: Expr,
        end: Expr,
        body: Block,
    },
    /// `for x in arr` — array element iteration.
    ForEach {
        var: String,
        iter: Expr,
        body: Block,
    },
    Return(Option<Expr>),
    Expr(Expr),
}

/// The left side of an assignment: a variable, an indexed element, or a field.
#[derive(Debug, Clone)]
pub enum AssignTarget {
    /// `x = v`
    Var(String),
    /// `a[i] = v`
    Index { name: String, index: Expr },
    /// `p.field = v` (incl. `self.field` inside `&mut self` methods)
    Field { name: String, field: String },
}

/// An expression plus its source line, so diagnostics can point at it.
#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Int(i64),
    Float(f64),
    /// `expr as Type` numeric cast.
    Cast {
        expr: Box<Expr>,
        ty: TypeExpr,
    },
    Str(String),
    /// Interpolated string: `"hi {name}"` → [Lit("hi "), Expr(name)].
    StrInterp(Vec<StrPart>),
    Bool(bool),
    Ident(String),
    Unary {
        op: UnOp,
        rhs: Box<Expr>,
    },
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Call {
        callee: String,
        args: Vec<Expr>,
    },
    /// `if` is an EXPRESSION (yields a value). `els` is another Expr
    /// (either Expr::Block or a nested Expr::If) to support else-if chains.
    If {
        cond: Box<Expr>,
        then_b: Block,
        els: Option<Box<Expr>>,
    },
    Block(Block),
    /// `User { name: ..., age: ... }`
    StructLit {
        name: String,
        fields: Vec<(String, Expr)>,
    },
    /// `base.field`
    Field {
        base: Box<Expr>,
        field: String,
    },
    /// A `Type::member(args)` / `Type::member` path. Resolved at check time to
    /// either enum construction (`Shape::Circle(2)`, `Color::Red`) or an
    /// associated-function call (`Point::new(3, 4)`).
    Path {
        ty: String,
        member: String,
        args: Vec<Expr>,
    },
    /// `receiver.method(args)`
    MethodCall {
        receiver: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
    /// `match scrutinee { pat => expr, ... }` — an expression.
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    /// `[a, b, c]` — array literal.
    ArrayLit(Vec<Expr>),
    /// `base[index]` — array element access.
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },
    /// `expr?` — unwrap `Ok`, or propagate `Err` to the caller.
    Try(Box<Expr>),
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
    Variant {
        enum_name: Option<String>,
        name: String,
        subs: Vec<Pattern>,
    },
}

#[derive(Debug, Clone)]
pub enum StrPart {
    Lit(String),
    Expr(Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}
