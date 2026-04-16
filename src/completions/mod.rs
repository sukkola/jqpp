pub mod jq_builtins;
pub mod json_context;
pub mod lsp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    pub detail: Option<String>,
    pub insert_text: String,
}
