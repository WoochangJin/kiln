//! Kiln 컴파일러 라이브러리. 파이프라인: 소스 → lexer → parser → sema → ir → codegen

pub mod diag;
pub mod token;
pub mod lexer;
pub mod ast;
pub mod parser;
pub mod sema;
pub mod ir;
pub mod codegen;
