//! Expression parser: compiles a string `f(x[, p, q, r])` into RPN
//! (reverse Polish notation) and evaluates it fast given `x` (and parameters).
//! No external dependencies.
//!
//! Supports:
//!   - arithmetic `+ - * /`, power `^`, parentheses, unary minus
//!   - implicit multiplication: `2x`, `x(1-x)`, `)(`
//!   - functions: sin cos tan asin acos atan sinh cosh tanh exp log(=ln) log10 log2
//!     sqrt cbrt abs sign floor ceil round / pow(a,b) atan2 min max mod
//!   - constants: pi e tau,  variable: x,  parameters: p, q, r

use std::f64::consts::{E, PI, TAU};
use std::fmt;

/// Adjustable parameter names (only those present in the expression matter).
/// The parameter array passed to `eval` and friends follows this order.
pub const PARAM_NAMES: [&str; 3] = ["p", "q", "r"];
/// Number of parameters.
pub const N_PARAMS: usize = PARAM_NAMES.len();

/// Parse error.
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    /// Unrecognized character.
    UnexpectedChar(char),
    /// Unknown identifier (allowed: x, p, q, r, pi, e, tau).
    UnknownName(String),
    /// Mismatched parentheses.
    MismatchedParen,
    /// Comma in an invalid position.
    BadComma,
    /// Empty expression.
    Empty,
    /// Expression too complex (exceeds the evaluation stack).
    TooComplex,
    /// Not parseable as a numeric literal.
    BadNumber(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnexpectedChar(c) => write!(f, "認識できない文字: '{c}'"),
            ParseError::UnknownName(s) => {
                write!(
                    f,
                    "未知の変数/関数: '{s}'  (使えるのは x, p, q, r, pi, e と各関数)"
                )
            }
            ParseError::MismatchedParen => write!(f, "括弧の対応が取れていません"),
            ParseError::BadComma => write!(f, "カンマの位置が不正です"),
            ParseError::Empty => write!(f, "式が空です"),
            ParseError::TooComplex => write!(f, "式が複雑すぎます"),
            ParseError::BadNumber(s) => write!(f, "数値として解釈できません: '{s}'"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Built-in functions (1-arg / 2-arg).
#[derive(Clone, Copy)]
enum FuncDef {
    F1(fn(f64) -> f64),
    F2(fn(f64, f64) -> f64),
}

fn fmod(a: f64, b: f64) -> f64 {
    a - b * (a / b).floor()
}

fn lookup_func(name: &str) -> Option<FuncDef> {
    use FuncDef::{F1, F2};
    Some(match name {
        "sin" => F1(f64::sin),
        "cos" => F1(f64::cos),
        "tan" => F1(f64::tan),
        "asin" => F1(f64::asin),
        "acos" => F1(f64::acos),
        "atan" => F1(f64::atan),
        "sinh" => F1(f64::sinh),
        "cosh" => F1(f64::cosh),
        "tanh" => F1(f64::tanh),
        "exp" => F1(f64::exp),
        "log" | "ln" => F1(f64::ln),
        "log10" => F1(f64::log10),
        "log2" => F1(f64::log2),
        "sqrt" => F1(f64::sqrt),
        "cbrt" => F1(f64::cbrt),
        "abs" => F1(f64::abs),
        "sign" => F1(f64::signum),
        "floor" => F1(f64::floor),
        "ceil" => F1(f64::ceil),
        "round" => F1(f64::round),
        "pow" => F2(f64::powf),
        "atan2" => F2(f64::atan2),
        "min" => F2(f64::min),
        "max" => F2(f64::max),
        "mod" => F2(fmod),
        _ => return None,
    })
}

/// Token.
#[derive(Clone)]
enum Tok {
    Num(f64),
    Name(String),
    Func(FuncDef),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    LParen,
    RParen,
    Comma,
}

fn tokenize(s: &str) -> Result<Vec<Tok>, ParseError> {
    let b = s.as_bytes();
    let n = b.len();
    let mut i = 0;
    let mut out = Vec::new();
    while i < n {
        let c = b[i];
        if c >= 128 {
            return Err(ParseError::UnexpectedChar(
                s[i..].chars().next().unwrap_or('?'),
            ));
        }
        let ch = c as char;
        if ch.is_whitespace() {
            i += 1;
            continue;
        }
        if ch.is_ascii_digit() || (ch == '.' && i + 1 < n && (b[i + 1] as char).is_ascii_digit()) {
            let start = i;
            i += 1;
            while i < n && ((b[i] as char).is_ascii_digit() || b[i] == b'.') {
                i += 1;
            }
            // exponent part (1e3, 2.5E-2, etc.)
            if i < n && (b[i] == b'e' || b[i] == b'E') {
                let mut j = i + 1;
                if j < n && (b[j] == b'+' || b[j] == b'-') {
                    j += 1;
                }
                if j < n && (b[j] as char).is_ascii_digit() {
                    i = j;
                    while i < n && (b[i] as char).is_ascii_digit() {
                        i += 1;
                    }
                }
            }
            let lit = &s[start..i];
            let v: f64 = lit
                .parse()
                .map_err(|_| ParseError::BadNumber(lit.to_string()))?;
            out.push(Tok::Num(v));
            continue;
        }
        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = i;
            i += 1;
            while i < n && ((b[i] as char).is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            let name = s[start..i].to_ascii_lowercase();
            match lookup_func(&name) {
                Some(fd) => out.push(Tok::Func(fd)),
                None => out.push(Tok::Name(name)),
            }
            continue;
        }
        let tok = match ch {
            '+' => Tok::Plus,
            '-' => Tok::Minus,
            '*' => Tok::Star,
            '/' => Tok::Slash,
            '^' => Tok::Caret,
            '(' => Tok::LParen,
            ')' => Tok::RParen,
            ',' => Tok::Comma,
            _ => return Err(ParseError::UnexpectedChar(ch)),
        };
        out.push(tok);
        i += 1;
    }
    Ok(out)
}

fn is_value_end(t: &Tok) -> bool {
    matches!(t, Tok::Num(_) | Tok::Name(_) | Tok::RParen)
}
fn is_value_start(t: &Tok) -> bool {
    matches!(t, Tok::Num(_) | Tok::Name(_) | Tok::Func(_) | Tok::LParen)
}

/// Insert implicit multiplication: `2x`→`2*x`, `x(1-x)`→`x*(1-x)`, `)(`→`)*(`.
fn insert_implicit_mul(toks: &[Tok]) -> Vec<Tok> {
    let mut out: Vec<Tok> = Vec::with_capacity(toks.len() * 2);
    for t in toks {
        if let Some(prev) = out.last()
            && is_value_end(prev)
            && is_value_start(t)
        {
            out.push(Tok::Star);
        }
        out.push(t.clone());
    }
    out
}

/// Operator-stack element.
enum SOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Neg,
    Func(FuncDef),
    LParen,
}

fn sop_prec(s: &SOp) -> u8 {
    match s {
        SOp::Add | SOp::Sub => 2,
        SOp::Mul | SOp::Div | SOp::Neg => 3,
        SOp::Pow => 4,
        SOp::Func(_) => 9,
        SOp::LParen => 0,
    }
}

fn sop_to_op(s: SOp) -> Op {
    match s {
        SOp::Add => Op::Add,
        SOp::Sub => Op::Sub,
        SOp::Mul => Op::Mul,
        SOp::Div => Op::Div,
        SOp::Pow => Op::Pow,
        SOp::Neg => Op::Neg,
        SOp::Func(FuncDef::F1(f)) => Op::F1(f),
        SOp::Func(FuncDef::F2(f)) => Op::F2(f),
        SOp::LParen => unreachable!("LParen never reaches the output"),
    }
}

/// Returns `(SOp, precedence, right-associative?)`.
fn bin_info(t: &Tok) -> (SOp, u8, bool) {
    match t {
        Tok::Plus => (SOp::Add, 2, false),
        Tok::Minus => (SOp::Sub, 2, false),
        Tok::Star => (SOp::Mul, 3, false),
        Tok::Slash => (SOp::Div, 3, false),
        Tok::Caret => (SOp::Pow, 4, true),
        _ => unreachable!("binary operators only"),
    }
}

/// Compiled instruction (one RPN element).
#[derive(Clone, Copy)]
pub enum Op {
    Num(f64),
    X,
    /// Parameter (index into PARAM_NAMES).
    Param(u8),
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Neg,
    F1(fn(f64) -> f64),
    F2(fn(f64, f64) -> f64),
}

fn to_rpn(toks: &[Tok]) -> Result<Vec<Op>, ParseError> {
    let mut out: Vec<Op> = Vec::new();
    let mut ops: Vec<SOp> = Vec::new();
    let mut prev_is_value = false;

    for t in toks {
        match t {
            Tok::Num(v) => {
                out.push(Op::Num(*v));
                prev_is_value = true;
            }
            Tok::Name(name) => {
                let op = match name.as_str() {
                    "x" => Op::X,
                    "pi" => Op::Num(PI),
                    "e" => Op::Num(E),
                    "tau" => Op::Num(TAU),
                    s => match PARAM_NAMES.iter().position(|&p| p == s) {
                        Some(i) => Op::Param(i as u8),
                        None => return Err(ParseError::UnknownName(name.clone())),
                    },
                };
                out.push(op);
                prev_is_value = true;
            }
            Tok::Func(fd) => {
                ops.push(SOp::Func(*fd));
                prev_is_value = false;
            }
            Tok::Comma => {
                while !matches!(ops.last(), Some(SOp::LParen) | None) {
                    out.push(sop_to_op(ops.pop().unwrap()));
                }
                if ops.is_empty() {
                    return Err(ParseError::BadComma);
                }
                prev_is_value = false;
            }
            Tok::Plus | Tok::Minus | Tok::Star | Tok::Slash | Tok::Caret => {
                let unary = !prev_is_value;
                if unary && matches!(t, Tok::Plus) {
                    continue; // ignore unary plus
                }
                if unary && matches!(t, Tok::Minus) {
                    ops.push(SOp::Neg); // prefix unary minus: push without popping
                    prev_is_value = false;
                    continue;
                }
                let (cur, p1, right) = bin_info(t);
                while let Some(top) = ops.last() {
                    if matches!(top, SOp::LParen) {
                        break;
                    }
                    let p2 = sop_prec(top);
                    if p2 > p1 || (p2 == p1 && !right) {
                        out.push(sop_to_op(ops.pop().unwrap()));
                    } else {
                        break;
                    }
                }
                ops.push(cur);
                prev_is_value = false;
            }
            Tok::LParen => {
                ops.push(SOp::LParen);
                prev_is_value = false;
            }
            Tok::RParen => {
                loop {
                    match ops.pop() {
                        Some(SOp::LParen) => break,
                        Some(op) => out.push(sop_to_op(op)),
                        None => return Err(ParseError::MismatchedParen),
                    }
                }
                if let Some(SOp::Func(_)) = ops.last() {
                    out.push(sop_to_op(ops.pop().unwrap()));
                }
                prev_is_value = true;
            }
        }
    }
    while let Some(op) = ops.pop() {
        if matches!(op, SOp::LParen) {
            return Err(ParseError::MismatchedParen);
        }
        out.push(sop_to_op(op));
    }
    if out.is_empty() {
        return Err(ParseError::Empty);
    }
    Ok(out)
}

/// Evaluation-stack capacity (fixed array for fast evaluation).
const STACK_CAP: usize = 64;

/// A compiled expression.
pub struct Expr {
    rpn: Vec<Op>,
    used: [bool; N_PARAMS],
    src: String,
}

impl Expr {
    /// Compile a source string.
    pub fn compile(src: &str) -> Result<Expr, ParseError> {
        let toks = insert_implicit_mul(&tokenize(src)?);
        let rpn = to_rpn(&toks)?;

        // Check the maximum evaluation-stack depth.
        let (mut depth, mut maxd) = (0i32, 0i32);
        for op in &rpn {
            depth += match op {
                Op::Num(_) | Op::X | Op::Param(_) => 1,
                Op::Neg | Op::F1(_) => 0,
                _ => -1, // binary operator / F2
            };
            maxd = maxd.max(depth);
        }
        if maxd as usize > STACK_CAP {
            return Err(ParseError::TooComplex);
        }

        let mut used = [false; N_PARAMS];
        for op in &rpn {
            if let Op::Param(i) = op {
                used[*i as usize] = true;
            }
        }
        Ok(Expr {
            rpn,
            used,
            src: src.to_string(),
        })
    }

    /// Whether each parameter (in PARAM_NAMES order) appears in the expression.
    #[inline]
    pub fn used_params(&self) -> [bool; N_PARAMS] {
        self.used
    }

    /// Whether parameter `i` appears in the expression.
    #[inline]
    pub fn uses_param(&self, i: usize) -> bool {
        self.used.get(i).copied().unwrap_or(false)
    }

    /// The original source string.
    #[inline]
    pub fn source(&self) -> &str {
        &self.src
    }

    /// Evaluate `f(x; params)`. `params` is in PARAM_NAMES order (p, q, r).
    #[inline]
    pub fn eval(&self, x: f64, params: &[f64; N_PARAMS]) -> f64 {
        let mut st = [0.0f64; STACK_CAP];
        let mut sp = 0usize;
        for op in &self.rpn {
            match *op {
                Op::Num(v) => {
                    st[sp] = v;
                    sp += 1;
                }
                Op::X => {
                    st[sp] = x;
                    sp += 1;
                }
                Op::Param(i) => {
                    st[sp] = params[i as usize];
                    sp += 1;
                }
                Op::Neg => st[sp - 1] = -st[sp - 1],
                Op::F1(f) => st[sp - 1] = f(st[sp - 1]),
                Op::F2(f) => {
                    sp -= 1;
                    let b = st[sp];
                    st[sp - 1] = f(st[sp - 1], b);
                }
                Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Pow => {
                    sp -= 1;
                    let b = st[sp];
                    let a = st[sp - 1];
                    st[sp - 1] = match *op {
                        Op::Add => a + b,
                        Op::Sub => a - b,
                        Op::Mul => a * b,
                        Op::Div => a / b,
                        Op::Pow => a.powf(b),
                        _ => unreachable!(),
                    };
                }
            }
        }
        st[0]
    }
}
