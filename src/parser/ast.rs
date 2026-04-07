use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Polarity {
    Positive,
    Negative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low = 0,
    Medium = 1,
    High = 2,
    Critical = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaNode {
    pub id: NodeId,
    pub text: String,
    pub role: String,
    pub attributes: Vec<String>,
    pub span: TextSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionNode {
    pub id: NodeId,
    pub text: String,
    pub verb: String,
    pub object: String,
    pub polarity: Polarity,
    pub priority: Priority,
    pub span: TextSpan,
    pub token_count: usize,
    pub embedding: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintNode {
    pub id: NodeId,
    pub text: String,
    pub priority: Priority,
    pub span: TextSpan,
    pub token_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextNode {
    pub id: NodeId,
    pub text: String,
    pub relevance_score: f64,
    pub token_count: usize,
    pub span: TextSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleNode {
    pub id: NodeId,
    pub input: String,
    pub output: String,
    pub demonstrates: Vec<String>,
    pub diversity_score: f64,
    pub token_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatNode {
    pub id: NodeId,
    pub text: String,
    pub format_type: FormatType,
    pub span: TextSpan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormatType {
    Json,
    Xml,
    Markdown,
    Csv,
    List,
    Table,
    PlainText,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Warning {
    pub message: String,
    pub span: Option<TextSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstMetadata {
    pub total_tokens: usize,
    pub source_hash: String,
    pub parse_warnings: Vec<Warning>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptAst {
    pub persona: Option<PersonaNode>,
    pub instructions: Vec<InstructionNode>,
    pub constraints: Vec<ConstraintNode>,
    pub context: Vec<ContextNode>,
    pub examples: Vec<ExampleNode>,
    pub format_spec: Option<FormatNode>,
    pub metadata: AstMetadata,
}

impl PromptAst {
    pub fn empty(source_hash: String) -> Self {
        Self {
            persona: None,
            instructions: Vec::new(),
            constraints: Vec::new(),
            context: Vec::new(),
            examples: Vec::new(),
            format_spec: None,
            metadata: AstMetadata {
                total_tokens: 0,
                source_hash,
                parse_warnings: Vec::new(),
            },
        }
    }
}
