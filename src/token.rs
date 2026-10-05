
use crate::diag::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // literals
    Int(u64), // '-' + literal for negative numbers. handled in parser
    Float(f64),
    Char(u8),
    Str(Vec<u8>), // not using String due to escape sequences
    Ident(String),
    
    // keywords
    Fn, Let, Mut, Struct, Enum, Impl, Match, If, Else, While, Loop,
    Break, Continue, Return, True, False, Null, Extern, Const, As,
    Sizeof, SelfType, Static,

    // operators
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    Comma, Semicolon, Colon, ColonColon, Dot, Arrow, FatArrow,
    Ellipsis,
    Underscore,
    Plus, Minus, Star, Slash, Percent,
    Amp, Pipe, Caret, Tilde, Shl, Shr,
    Eq, EqEq, NotEq, Lt, Gt, LtEq, GtEq,
    AndAnd, OrOr, Bang,
    PlusEq, MinusEq, StarEq, SlashEq, PercentEq,
    AmpEq, PipeEq, CaretEq, ShlEq, ShrEq,

    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}