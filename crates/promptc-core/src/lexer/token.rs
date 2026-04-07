use crate::parser::ast::TextSpan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    SectionHeader,
    NumberedItem,
    Bullet,
    ExampleStart,
    FormatDirective,
    NegativeMarker,
    PriorityMarker,
    Sentence,
    Whitespace,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    pub span: TextSpan,
}
