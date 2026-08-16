use crate::semantic::{SemanticAction, Selector};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectionOp { Keep, Flatten, Prune, Collapse }
impl ProjectionOp { pub fn parse(v: &str) -> Option<Self> { Some(match v.trim() { "keep" => Self::Keep, "flatten" => Self::Flatten, "prune" => Self::Prune, "collapse" => Self::Collapse, _ => return None }) } }

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Importance { Low, Normal, High, Critical }
impl Importance { pub fn parse(v: &str) -> Option<Self> { Some(match v.trim() { "low" => Self::Low, "normal" => Self::Normal, "high" => Self::High, "critical" => Self::Critical, _ => return None }) } }
impl Default for Importance { fn default() -> Self { Self::Normal } }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpan { pub line: usize, pub column: usize }
impl Default for SourceSpan { fn default() -> Self { Self { line: 1, column: 1 } } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackRule {
    pub selector: Selector,
    pub projection: Option<ProjectionOp>,
    pub alias: Option<String>,
    pub importance: Option<Importance>,
    pub max_items: Option<usize>,
    pub max_depth: Option<usize>,
    pub expose: Vec<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackView {
    pub name: String,
    pub when: Option<Selector>,
    pub priority: i32,
    pub default_projection: ProjectionOp,
    pub uses: Vec<String>,
    pub rules: Vec<PackRule>,
    pub order: usize,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackTarget {
    pub name: String,
    pub matches: Vec<Selector>,
    pub multi: bool,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackAction {
    pub name: String,
    pub target: String,
    pub perform: SemanticAction,
    pub argument: Option<String>,
    pub fallback_keys: Option<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackStylesheet {
    pub views: Vec<PackView>,
    pub targets: Vec<PackTarget>,
    pub actions: Vec<PackAction>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DetectRule {
    pub executables: Vec<String>,
    pub titles: Vec<String>,
    pub bundle_ids: Vec<String>,
    pub root_selector: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackManifest {
    pub schema: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub desktop_cli: Option<String>,
    pub default_view: String,
    pub detect: BTreeMap<String, DetectRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationPack {
    pub manifest: PackManifest,
    pub stylesheet: PackStylesheet,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackDiagnostic {
    pub level: DiagnosticLevel,
    pub message: String,
    pub span: Option<SourceSpan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticLevel { Error, Warning, Info }
