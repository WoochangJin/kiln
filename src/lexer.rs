// source text to token stream
use crate::diag::{Span, Diag};
use crate::token::{Token, TokenKind};

type LexResult<T> = Result<T, Diag>;

pub struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Lexer { src: src.as_bytes(), pos: 0 }
    }

    pub fn tokenize(mut self) -> LexResult<Vec<Token>> {
        let mut out = Vec::new();
        loop {
            let tok = self.next_token()?;
            let eof = tok.kind == TokenKind::Eof;
            out.push(tok);
            if eof {
                return Ok(out)
            }
        }
    }

    // ---- cursor ----

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn peek_at(&self, n: usize) -> Option<u8> {
        self.src.get(self.pos + n).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.pos += 1;
        Some(c)
    }
    
    fn eat(&mut self, c: u8) -> bool {
        if self.peek() == Some(c) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn err<T>(&self, start: usize, msg: impl Into<String>) -> LexResult<T> {
        Err(Diag::new(start, self.pos.max(start + 1), msg))
    }

    // ---- token ----
    
    fn next_token(&mut self) -> LexResult<Token> {
        self.skip_trivia()?;
        let start = self.pos;
        let Some(c) = self.peek() else {
            return Ok(Token { kind: TokenKind::Eof, span: Span { start, end: start } });
        };
        
        let kind = match c {
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => self.ident(),
            b'0'..=b'9' => self.number(start)?,
            b'\'' => self.char_lit(start)?,
            b'"' => self.string_lit(start)?,
            _ => self.operator(start)?,
        };
        Ok(Token { kind, span: Span { start, end: self.pos } })
    }

    // skip whitespace and comments
    fn skip_trivia(&mut self) -> LexResult<()> {
        loop {
            match (self.peek(), self.peek_at(1)) {
                (Some(b' ' | b'\t' | b'\r' | b'\n'), _) => self.pos += 1,
                // line comment
                (Some(b'/'), Some(b'/')) => {
                    while !matches!(self.peek(), None | Some(b'\n')) {
                        self.pos += 1;
                    }
                }
                // block comment, does not nest
                (Some(b'/'), Some(b'*')) => {
                    let start = self.pos;
                    self.pos += 2;
                    loop {
                        match (self.peek(), self.peek_at(1)) {
                            (Some(b'*'), Some(b'/')) => {
                                self.pos += 2;
                                break;
                            }
                            (Some(_), _) => self.pos += 1,
                            (None, _) => return self.err(start, "unterminated block comment"),
                        }
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    // ---- literals ----

    // identifier or keyword
    fn ident(&mut self) -> TokenKind {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_ascii_alphanumeric() || c == b'_') {
            self.pos += 1;
        }
        // ascii only, so utf-8 conversion never fails
        let text = std::str::from_utf8(&self.src[start..self.pos]).unwrap();
        keyword(text).unwrap_or_else(|| TokenKind::Ident(text.to_string()))
    }

    fn number(&mut self, start: usize) -> LexResult<TokenKind> {
        let radix = match (self.peek(), self.peek_at(1)) {
            (Some(b'0'), Some(b'x')) => 16,
            (Some(b'0'), Some(b'b')) => 2,
            _ => 10,
        };
        if radix != 10 {
            self.pos += 2;
        }

        let digits_start = self.pos;
        self.eat_digits(radix);
        let int_digits = self.digits_since(digits_start);
        if int_digits.is_empty() {
            return self.err(start, "expected digits after radix prefix");
        }

        // float only if a digit follows the dot, `3.` is Int + Dot (parser rejects it)
        let mut is_float = false;
        if radix == 10 && self.peek() == Some(b'.') && matches!(self.peek_at(1), Some(b'0'..=b'9')) {
            is_float = true;
            self.pos += 1;
            self.eat_digits(10);
            if matches!(self.peek(), Some(b'e' | b'E')) {
                self.pos += 1;
                if matches!(self.peek(), Some(b'+' | b'-')) {
                    self.pos += 1;
                }
                let exp_start = self.pos;
                self.eat_digits(10);
                if self.pos == exp_start {
                    return self.err(start, "expected digits in exponent");
                }
            }
        }

        // reject bad suffix like 12abc, 0b102
        if matches!(self.peek(), Some(c) if c.is_ascii_alphanumeric() || c == b'_') {
            self.pos += 1;
            return self.err(start, "invalid character in number literal");
        }

        if is_float {
            let text = self.digits_since(start);
            match text.parse::<f64>() {
                Ok(v) if v.is_finite() => Ok(TokenKind::Float(v)),
                _ => self.err(start, "float literal out of range"),
            }
        } else {
            match u64::from_str_radix(&int_digits, radix) {
                Ok(v) => Ok(TokenKind::Int(v)),
                Err(_) => self.err(start, "integer literal too large"),
            }
        }
    }

    // digits of the radix and `_` separators
    fn eat_digits(&mut self, radix: u32) {
        while matches!(self.peek(), Some(c) if c == b'_' || (c as char).is_digit(radix)) {
            self.pos += 1;
        }
    }

    // text from `from` to pos without `_`
    fn digits_since(&self, from: usize) -> String {
        self.src[from..self.pos].iter().filter(|&&c| c != b'_').map(|&c| c as char).collect()
    }

    fn char_lit(&mut self, start: usize) -> LexResult<TokenKind> {
        self.pos += 1; // opening '
        let c = match self.bump() {
            None | Some(b'\n') => return self.err(start, "unterminated char literal"),
            Some(b'\'') => return self.err(start, "empty char literal"),
            Some(b'\\') => self.escape(start)?,
            Some(c) if c >= 0x80 => {
                return self.err(start, "non-ASCII char literal, use \\xNN");
            }
            Some(c) => c,
        };
        if !self.eat(b'\'') {
            return self.err(start, "unterminated char literal");
        }
        Ok(TokenKind::Char(c))
    }

    fn string_lit(&mut self, start: usize) -> LexResult<TokenKind> {
        self.pos += 1; // opening "
        let mut buf = Vec::new();
        loop {
            match self.bump() {
                None => return self.err(start, "unterminated string literal"),
                Some(b'"') => return Ok(TokenKind::Str(buf)),
                Some(b'\\') => buf.push(self.escape(start)?),
                Some(c) => buf.push(c),
            }
        }
    }

    // read one escaped byte, pos is right after the backslash
    fn escape(&mut self, lit_start: usize) -> LexResult<u8> {
        let esc_start = self.pos - 1;
        let Some(c) = self.bump() else {
            return self.err(lit_start, "unterminated literal");
        };
        Ok(match c {
            b'n' => b'\n',
            b't' => b'\t',
            b'r' => b'\r',
            b'0' => 0,
            b'\\' => b'\\',
            b'\'' => b'\'',
            b'"' => b'"',
            b'x' => {
                let hi = self.hex_digit(esc_start)?;
                let lo = self.hex_digit(esc_start)?;
                hi << 4 | lo
            }
            _ => return self.err(esc_start, "unknown escape sequence"),
        })
    }

    fn hex_digit(&mut self, esc_start: usize) -> LexResult<u8> {
        match self.peek().and_then(|c| (c as char).to_digit(16)) {
            Some(d) => {
                self.pos += 1;
                Ok(d as u8)
            }
            None => self.err(esc_start, "expected two hex digits after \\x"),
        }
    }

    // ---- operators ----

    // operators and punctuation, longest match first
    fn operator(&mut self, start: usize) -> LexResult<TokenKind> {
        use TokenKind::*;
        let c = self.bump().unwrap();
        Ok(match c {
            b'(' => LParen,
            b')' => RParen,
            b'{' => LBrace,
            b'}' => RBrace,
            b'[' => LBracket,
            b']' => RBracket,
            b',' => Comma,
            b';' => Semicolon,
            b'~' => Tilde,
            b'.' => {
                if !self.eat(b'.') {
                    Dot
                } else if self.eat(b'.') {
                    Ellipsis
                } else {
                    return self.err(start, "unexpected `..`");
                }
            }
            b':' => if self.eat(b':') { ColonColon } else { Colon },
            b'=' => if self.eat(b'=') { EqEq } else if self.eat(b'>') { FatArrow } else { Eq },
            b'!' => if self.eat(b'=') { NotEq } else { Bang },
            b'+' => if self.eat(b'=') { PlusEq } else { Plus },
            b'-' => if self.eat(b'>') { Arrow } else if self.eat(b'=') { MinusEq } else { Minus },
            b'*' => if self.eat(b'=') { StarEq } else { Star },
            b'/' => if self.eat(b'=') { SlashEq } else { Slash },
            b'%' => if self.eat(b'=') { PercentEq } else { Percent },
            b'&' => if self.eat(b'&') { AndAnd } else if self.eat(b'=') { AmpEq } else { Amp },
            b'|' => if self.eat(b'|') { OrOr } else if self.eat(b'=') { PipeEq } else { Pipe },
            b'^' => if self.eat(b'=') { CaretEq } else { Caret },
            b'<' => {
                if self.eat(b'<') {
                    if self.eat(b'=') { ShlEq } else { Shl }
                } else if self.eat(b'=') {
                    LtEq
                } else {
                    Lt
                }
            }
            b'>' => {
                if self.eat(b'>') {
                    if self.eat(b'=') { ShrEq } else { Shr }
                } else if self.eat(b'=') {
                    GtEq
                } else {
                    Gt
                }
            }
            _ => {
                let shown = if c.is_ascii_graphic() { format!("`{}`", c as char) } else { format!("0x{c:02X}") };
                return self.err(start, format!("unexpected character {shown}"));
            }
        })
    }
}

// keyword table, `_` is the wildcard token
fn keyword(s: &str) -> Option<TokenKind> {
    use TokenKind::*;
    Some(match s {
        "_" => Underscore,
        "fn" => Fn,
        "let" => Let,
        "mut" => Mut,
        "struct" => Struct,
        "enum" => Enum,
        "impl" => Impl,
        "match" => Match,
        "if" => If,
        "else" => Else,
        "while" => While,
        "loop" => Loop,
        "break" => Break,
        "continue" => Continue,
        "return" => Return,
        "true" => True,
        "false" => False,
        "null" => Null,
        "extern" => Extern,
        "const" => Const,
        "as" => As,
        "sizeof" => Sizeof,
        "Self" => SelfType,
        "static" => Static,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use TokenKind::*;

    // token kinds without the trailing Eof
    fn kinds(src: &str) -> Vec<TokenKind> {
        let mut v: Vec<_> = Lexer::new(src).tokenize().unwrap().into_iter().map(|t| t.kind).collect();
        assert_eq!(v.pop(), Some(Eof));
        v
    }

    fn err_msg(src: &str) -> String {
        Lexer::new(src).tokenize().unwrap_err().msg
    }

    #[test]
    fn empty_source_is_just_eof() {
        assert_eq!(kinds(""), vec![]);
        assert_eq!(kinds("  // c\n /* c */ "), vec![]);
    }

    #[test]
    fn keywords_and_identifiers() {
        assert_eq!(
            kinds("fn main _ _x Self self static"),
            vec![Fn, Ident("main".into()), Underscore, Ident("_x".into()), SelfType, Ident("self".into()), Static]
        );
    }

    #[test]
    fn integer_radix_and_separators() {
        assert_eq!(kinds("42 0xFF 0b1010 1_000_000 0"), vec![Int(42), Int(255), Int(10), Int(1_000_000), Int(0)]);
        assert_eq!(kinds("18446744073709551615"), vec![Int(u64::MAX)]);
    }

    #[test]
    fn integer_errors() {
        assert_eq!(err_msg("18446744073709551616"), "integer literal too large");
        assert_eq!(err_msg("0x"), "expected digits after radix prefix");
        assert_eq!(err_msg("0b102"), "invalid character in number literal");
        assert_eq!(err_msg("12abc"), "invalid character in number literal");
    }

    #[test]
    fn float_literals() {
        assert_eq!(kinds("1.5 2.0e-3 1_0.2_5 3.0E+2"), vec![Float(1.5), Float(2.0e-3), Float(10.25), Float(300.0)]);
        assert_eq!(err_msg("1.5e"), "expected digits in exponent");
        assert_eq!(err_msg("1e5"), "invalid character in number literal");
    }

    #[test]
    fn trailing_dot_is_not_a_float() {
        assert_eq!(kinds("3."), vec![Int(3), Dot]);
    }

    #[test]
    fn char_literals() {
        assert_eq!(
            kinds(r"'a' '\n' '\0' '\x41' '\'' '\\'"),
            vec![Char(b'a'), Char(b'\n'), Char(0), Char(0x41), Char(b'\''), Char(b'\\')]
        );
        assert_eq!(err_msg("''"), "empty char literal");
        assert_eq!(err_msg("'ab'"), "unterminated char literal");
        assert_eq!(err_msg("'a"), "unterminated char literal");
        assert_eq!(err_msg("'é'"), "non-ASCII char literal, use \\xNN");
    }

    #[test]
    fn string_literals_are_bytes() {
        assert_eq!(kinds(r#""hello\n""#), vec![Str(b"hello\n".to_vec())]);
        assert_eq!(kinds(r#""\xFF\x00a\"""#), vec![Str(vec![0xFF, 0x00, b'a', b'"'])]);
        assert_eq!(kinds(r#""""#), vec![Str(vec![])]);
    }

    #[test]
    fn string_errors() {
        assert_eq!(err_msg(r#""abc"#), "unterminated string literal");
        assert_eq!(err_msg(r#""\q""#), "unknown escape sequence");
        assert_eq!(err_msg(r#""\xZ1""#), "expected two hex digits after \\x");
        assert_eq!(err_msg(r#""\x4"#), "expected two hex digits after \\x");
    }

    #[test]
    fn operators_longest_match() {
        assert_eq!(kinds("<<= << <= < >>= >> >= >"), vec![ShlEq, Shl, LtEq, Lt, ShrEq, Shr, GtEq, Gt]);
        assert_eq!(
            kinds("... . :: : => == = -> -= - != !"),
            vec![Ellipsis, Dot, ColonColon, Colon, FatArrow, EqEq, Eq, Arrow, MinusEq, Minus, NotEq, Bang]
        );
        assert_eq!(kinds("&& &= & || |= | ^= ^ ~"), vec![AndAnd, AmpEq, Amp, OrOr, PipeEq, Pipe, CaretEq, Caret, Tilde]);
        assert_eq!(kinds("+= + *= * /= / %= %"), vec![PlusEq, Plus, StarEq, Star, SlashEq, Slash, PercentEq, Percent]);
        assert_eq!(kinds("( ) { } [ ] , ;"), vec![LParen, RParen, LBrace, RBrace, LBracket, RBracket, Comma, Semicolon]);
    }

    #[test]
    fn comments() {
        assert_eq!(kinds("a // x\nb /* y\n z */ c"), vec![Ident("a".into()), Ident("b".into()), Ident("c".into())]);
        assert_eq!(err_msg("/* open"), "unterminated block comment");
        // no nesting, ends at the first `*/` and the rest are tokens
        assert_eq!(kinds("/* /* */ x */"), vec![Ident("x".into()), Star, Slash]);
    }

    #[test]
    fn negative_literal_is_minus_then_int() {
        assert_eq!(kinds("-9223372036854775808"), vec![Minus, Int(9223372036854775808)]);
    }

    #[test]
    fn unexpected_character() {
        assert_eq!(err_msg("a # b"), "unexpected character `#`");
        assert_eq!(err_msg("é"), "unexpected character 0xC3");
        assert_eq!(err_msg(".."), "unexpected `..`");
    }

    #[test]
    fn spans() {
        let toks = Lexer::new("let  x = 0xFF;").tokenize().unwrap();
        let spans: Vec<_> = toks.iter().map(|t| (t.span.start, t.span.end)).collect();
        assert_eq!(spans, vec![(0, 3), (5, 6), (7, 8), (9, 13), (13, 14), (14, 14)]);
    }

    #[test]
    fn small_program() {
        let src = r#"extern fn printf(fmt: *u8, ...) -> i32;
fn main() -> i32 { printf("hi %d\n", 1 + 2); return 0; }"#;
        let ks = kinds(src);
        assert_eq!(ks.len(), 34);
        assert!(ks.contains(&Ellipsis) && ks.contains(&Arrow) && ks.contains(&Str(b"hi %d\n".to_vec())));
    }
}