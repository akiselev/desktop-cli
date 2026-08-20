use super::{parse_manifest, parse_stylesheet, ApplicationPack, DiagnosticLevel, PackDiagnostic};
use crate::semantic::{AccessibilityGraph, ObservationBudget};
use super::project;
use std::fs;
use std::path::Path;

impl ApplicationPack {
    pub fn from_strings(manifest:&str,stylesheet:&str)->Result<Self,Vec<PackDiagnostic>>{
        let manifest=parse_manifest(manifest)?;let stylesheet=parse_stylesheet(stylesheet)?;Ok(Self{manifest,stylesheet})
    }
    pub fn load_dir(path:impl AsRef<Path>)->Result<Self,Vec<PackDiagnostic>>{
        let path=path.as_ref();let manifest=fs::read_to_string(path.join("pack.toml")).map_err(|e|vec![PackDiagnostic{level:DiagnosticLevel::Error,message:format!("failed to read pack.toml: {}",e),span:None}])?;
        let stylesheet=fs::read_to_string(path.join("views.dcss")).map_err(|e|vec![PackDiagnostic{level:DiagnosticLevel::Error,message:format!("failed to read views.dcss: {}",e),span:None}])?;
        Self::from_strings(&manifest,&stylesheet)
    }
}

pub fn builtin_altium()->Result<ApplicationPack,Vec<PackDiagnostic>>{ApplicationPack::from_strings(include_str!("../../packs/altium/pack.toml"),include_str!("../../packs/altium/views.dcss"))}

#[derive(Debug,Clone,serde::Serialize,serde::Deserialize)]
pub struct ValidationReport{pub pack:String,pub active_view:String,pub source_nodes:usize,pub exposed_nodes:usize,pub aliases:usize,pub warnings:Vec<String>}

pub fn validate_against_graph(graph:&mut AccessibilityGraph,pack:&ApplicationPack,budget:ObservationBudget)->Result<ValidationReport,String>{let result=project(graph,pack,budget)?;let warnings=result.diagnostics.iter().filter(|d|d.level!=DiagnosticLevel::Info).map(|d|d.message.clone()).collect();Ok(ValidationReport{pack:pack.manifest.id.clone(),active_view:result.active_view,source_nodes:result.observation.stats.source_nodes,exposed_nodes:result.observation.stats.exposed_nodes,aliases:result.aliases.len(),warnings})}
