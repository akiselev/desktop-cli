use super::model::*;
use crate::semantic::{SemanticAction, Selector};
use std::collections::BTreeMap;

pub fn parse_manifest(input: &str) -> Result<PackManifest, Vec<PackDiagnostic>> {
    let clean = strip_hash_comments(input);
    let mut section = String::new();
    let mut values: BTreeMap<(String,String), String> = BTreeMap::new();
    for (line_index, raw) in clean.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() { continue; }
        if line.starts_with('[') && line.ends_with(']') { section = line[1..line.len()-1].trim().to_string(); continue; }
        let Some((key, value)) = line.split_once('=') else { return Err(vec![diag(DiagnosticLevel::Error, format!("invalid manifest assignment on line {}", line_index+1), line_index+1)]); };
        values.insert((section.clone(), key.trim().into()), value.trim().into());
    }
    let get = |s: &str, k: &str| values.get(&(s.to_string(), k.to_string())).cloned();
    let schema = get("", "schema").and_then(|v| v.parse().ok()).unwrap_or(1);
    let id = unquote_owned(&get("", "id").unwrap_or_default());
    let name = unquote_owned(&get("", "name").unwrap_or_default());
    let version = unquote_owned(&get("", "version").unwrap_or_else(|| "0.1.0".into()));
    if id.is_empty() || name.is_empty() { return Err(vec![diag(DiagnosticLevel::Error, "manifest requires id and name".into(), 1)]); }
    let desktop_cli = get("compat", "desktop_cli").map(|v| unquote_owned(&v));
    let default_view = get("view", "default").map(|v| unquote_owned(&v)).unwrap_or_else(|| "default".into());
    let mut detect = BTreeMap::new();
    for platform in ["windows", "linux", "macos"] {
        let sec = format!("detect.{platform}");
        let rule = DetectRule {
            executables: get(&sec, "executables").map(|v| parse_array(&v)).unwrap_or_default(),
            titles: get(&sec, "titles").map(|v| parse_array(&v)).unwrap_or_default(),
            bundle_ids: get(&sec, "bundle_ids").or_else(|| get(&sec, "bundle-ids")).map(|v| parse_array(&v)).unwrap_or_default(),
            root_selector: get(&sec, "root_selector").or_else(|| get(&sec, "root-selector")).map(|v| unquote_owned(&v)),
        };
        if !rule.executables.is_empty() || !rule.titles.is_empty() || !rule.bundle_ids.is_empty() || rule.root_selector.is_some() { detect.insert(platform.into(), rule); }
    }
    Ok(PackManifest { schema, id, name, version, desktop_cli, default_view, detect })
}

pub fn parse_stylesheet(input: &str) -> Result<PackStylesheet, Vec<PackDiagnostic>> {
    let clean = strip_css_comments(input);
    let blocks = scan_blocks(&clean).map_err(|e| vec![diag(DiagnosticLevel::Error, e, 1)])?;
    let mut sheet = PackStylesheet::default();
    for (order, block) in blocks.into_iter().enumerate() {
        let header = block.header.trim();
        if let Some(rest) = header.strip_prefix("@view ") { sheet.views.push(parse_view(rest, &block.body, order, block.line)?); }
        else if let Some(rest) = header.strip_prefix("@target ") { sheet.targets.push(parse_target(rest, &block.body, block.line)?); }
        else if let Some(rest) = header.strip_prefix("@action ") { sheet.actions.push(parse_action(rest, &block.body, block.line)?); }
        else { return Err(vec![diag(DiagnosticLevel::Error, format!("unknown top-level rule '{}'", header), block.line)]); }
    }
    validate_stylesheet(&sheet)?;
    Ok(sheet)
}

fn parse_view(header: &str, body: &str, order: usize, line: usize) -> Result<PackView, Vec<PackDiagnostic>> {
    let (name, when) = if let Some((name, condition)) = header.split_once(" when ") {
        (name.trim(), Some(Selector::parse(condition.trim()).map_err(|e| vec![diag(DiagnosticLevel::Error, e, line)])?))
    } else { (header.trim(), None) };
    if name.is_empty() { return Err(vec![diag(DiagnosticLevel::Error, "view name is empty".into(), line)]); }
    let (statements, blocks) = scan_view_body(body).map_err(|e| vec![diag(DiagnosticLevel::Error, e, line)])?;
    let mut priority = 0i32; let mut default_projection = ProjectionOp::Prune; let mut uses = Vec::new();
    for statement in statements {
        let s = statement.trim(); if s.is_empty() { continue; }
        if let Some(base) = s.strip_prefix("@use ") { uses.push(base.trim().to_string()); continue; }
        let Some((key,value)) = s.split_once(':') else { return Err(vec![diag(DiagnosticLevel::Error, format!("invalid view declaration '{}'", s), line)]); };
        match key.trim() {
            "priority" => priority = value.trim().parse().map_err(|_| vec![diag(DiagnosticLevel::Error, "priority must be integer".into(), line)])?,
            "default-projection" => default_projection = ProjectionOp::parse(value).ok_or_else(|| vec![diag(DiagnosticLevel::Error, format!("unknown projection '{}'", value.trim()), line)])?,
            other => return Err(vec![diag(DiagnosticLevel::Error, format!("unknown view property '{}'", other), line)]),
        }
    }
    let mut rules = Vec::new();
    for block in blocks { rules.push(parse_rule(&block.header, &block.body, block.line)?); }
    Ok(PackView { name: name.into(), when, priority, default_projection, uses, rules, order, span: SourceSpan { line, column: 1 } })
}

fn parse_rule(header: &str, body: &str, line: usize) -> Result<PackRule, Vec<PackDiagnostic>> {
    let selector = Selector::parse(header.trim()).map_err(|e| vec![diag(DiagnosticLevel::Error, e, line)])?;
    let decls = parse_declarations(body);
    let mut projection = None; let mut alias = None; let mut importance = None; let mut max_items = None; let mut max_depth = None; let mut expose = Vec::new();
    for (key,value) in decls {
        match key.as_str() {
            "projection" => projection = Some(ProjectionOp::parse(&value).ok_or_else(|| vec![diag(DiagnosticLevel::Error, format!("unknown projection '{}'", value), line)])?),
            "alias" => alias = Some(value.trim_start_matches('$').to_string()),
            "importance" => importance = Some(Importance::parse(&value).ok_or_else(|| vec![diag(DiagnosticLevel::Error, format!("unknown importance '{}'", value), line)])?),
            "max-items" => max_items = Some(value.parse().map_err(|_| vec![diag(DiagnosticLevel::Error, "max-items must be integer".into(), line)])?),
            "max-depth" => max_depth = Some(value.parse().map_err(|_| vec![diag(DiagnosticLevel::Error, "max-depth must be integer".into(), line)])?),
            "expose" => expose = value.split_whitespace().map(str::to_string).collect(),
            other => return Err(vec![diag(DiagnosticLevel::Error, format!("unknown rule property '{}'", other), line)]),
        }
    }
    Ok(PackRule { selector, projection, alias, importance, max_items, max_depth, expose, span: SourceSpan { line, column: 1 } })
}

fn parse_target(header: &str, body: &str, line: usize) -> Result<PackTarget, Vec<PackDiagnostic>> {
    let name = header.trim().to_string(); let mut matches = Vec::new(); let mut multi = false;
    for (key,value) in parse_declarations(body) {
        match key.as_str() {
            "match" => matches.push(Selector::parse(&value).map_err(|e| vec![diag(DiagnosticLevel::Error, e, line)])?),
            "multi" => multi = matches!(value.as_str(), "true" | "yes" | "1"),
            other => return Err(vec![diag(DiagnosticLevel::Error, format!("unknown target property '{}'", other), line)]),
        }
    }
    if matches.is_empty() { return Err(vec![diag(DiagnosticLevel::Error, format!("target '{}' has no match", name), line)]); }
    Ok(PackTarget { name, matches, multi, span: SourceSpan { line, column: 1 } })
}

fn parse_action(header: &str, body: &str, line: usize) -> Result<PackAction, Vec<PackDiagnostic>> {
    let name = header.trim().to_string(); let decls = parse_declarations(body);
    let target = decls.iter().rev().find(|(k,_)| k == "target").map(|(_,v)| v.clone()).ok_or_else(|| vec![diag(DiagnosticLevel::Error, "action requires target".into(), line)])?;
    let perform_s = decls.iter().rev().find(|(k,_)| k == "perform").map(|(_,v)| v.as_str()).ok_or_else(|| vec![diag(DiagnosticLevel::Error, "action requires perform".into(), line)])?;
    let perform = SemanticAction::parse(perform_s).ok_or_else(|| vec![diag(DiagnosticLevel::Error, format!("unknown semantic action '{}'", perform_s), line)])?;
    let argument = decls.iter().rev().find(|(k,_)| k == "argument").map(|(_,v)| v.clone());
    let fallback_keys = decls.iter().rev().find(|(k,_)| k == "fallback-keys").map(|(_,v)| unquote_owned(v));
    for (key,_) in &decls { if !matches!(key.as_str(), "target"|"perform"|"argument"|"fallback-keys") { return Err(vec![diag(DiagnosticLevel::Error, format!("unknown action property '{}'", key), line)]); } }
    Ok(PackAction { name, target, perform, argument, fallback_keys, span: SourceSpan { line, column: 1 } })
}

pub fn validate_stylesheet(sheet: &PackStylesheet) -> Result<(), Vec<PackDiagnostic>> {
    let mut diagnostics = Vec::new();
    let mut view_names = std::collections::BTreeSet::new();
    for view in &sheet.views {
        if !view_names.insert(view.name.clone()) { diagnostics.push(PackDiagnostic { level: DiagnosticLevel::Error, message: format!("duplicate view '{}'", view.name), span: Some(view.span.clone()) }); }
    }
    for view in &sheet.views { for base in &view.uses { if !sheet.views.iter().any(|v| &v.name == base) { diagnostics.push(PackDiagnostic { level: DiagnosticLevel::Error, message: format!("view '{}' uses missing view '{}'", view.name, base), span: Some(view.span.clone()) }); } } }
    let mut target_names = std::collections::BTreeSet::new();
    for target in &sheet.targets { if !target_names.insert(target.name.clone()) { diagnostics.push(PackDiagnostic { level: DiagnosticLevel::Error, message: format!("duplicate target '{}'", target.name), span: Some(target.span.clone()) }); } }
    let mut action_names = std::collections::BTreeSet::new();
    for action in &sheet.actions { if !action_names.insert(action.name.clone()) { diagnostics.push(PackDiagnostic { level: DiagnosticLevel::Error, message: format!("duplicate action '{}'", action.name), span: Some(action.span.clone()) }); } }
    if diagnostics.iter().any(|d| d.level == DiagnosticLevel::Error) { Err(diagnostics) } else { Ok(()) }
}

#[derive(Debug)]
struct Block { header: String, body: String, line: usize }

fn scan_blocks(input: &str) -> Result<Vec<Block>, String> {
    let mut result = Vec::new(); let mut pos = 0;
    while let Some(open_rel) = input[pos..].find('{') {
        let open = pos + open_rel; let header_start = input[pos..open].rfind(|c: char| c == '}' || c == ';').map(|i| pos+i+1).unwrap_or(pos);
        let header = input[header_start..open].trim(); if header.is_empty() { return Err("empty block header".into()); }
        let close = find_matching_brace(input, open)?;
        let line = 1 + input[..header_start].bytes().filter(|b| *b == b'\n').count();
        result.push(Block { header: header.to_string(), body: input[open+1..close].to_string(), line }); pos = close + 1;
        while pos < input.len() && input.as_bytes()[pos].is_ascii_whitespace() { pos += 1; }
    }
    Ok(result)
}

fn scan_view_body(input: &str) -> Result<(Vec<String>, Vec<Block>), String> {
    let mut statements = Vec::new(); let mut blocks = Vec::new(); let mut pos = 0; let bytes = input.as_bytes();
    while pos < input.len() {
        while pos < input.len() && bytes[pos].is_ascii_whitespace() { pos += 1; } if pos >= input.len() { break; }
        let semi = find_top_level(input, pos, ';'); let brace = find_top_level(input, pos, '{');
        match (semi, brace) {
            (Some(s), Some(b)) if s < b => { statements.push(input[pos..s].trim().to_string()); pos = s + 1; }
            (Some(s), None) => { statements.push(input[pos..s].trim().to_string()); pos = s + 1; }
            (_, Some(b)) => { let header = input[pos..b].trim().to_string(); let close = find_matching_brace(input, b)?; let line = 1 + input[..pos].bytes().filter(|v| *v == b'\n').count(); blocks.push(Block { header, body: input[b+1..close].to_string(), line }); pos = close + 1; }
            (None,None) => { if !input[pos..].trim().is_empty() { statements.push(input[pos..].trim().to_string()); } break; }
            _ => unreachable!(),
        }
    }
    Ok((statements, blocks))
}

fn find_top_level(input: &str, start: usize, needle: char) -> Option<usize> {
    let mut paren=0i32; let mut bracket=0i32; let mut quote=None; let chars: Vec<(usize,char)> = input[start..].char_indices().map(|(i,c)|(start+i,c)).collect();
    for (idx,c) in chars { if let Some(q)=quote { if c==q {quote=None;} continue; } match c { '\''|'"'=>quote=Some(c), '('=>paren+=1, ')'=>paren-=1, '['=>bracket+=1, ']'=>bracket-=1, _=>{} } if c==needle && paren==0 && bracket==0 { return Some(idx); } }
    None
}
fn find_matching_brace(input: &str, open: usize) -> Result<usize,String> { let mut depth=0i32; let mut quote=None; for (rel,c) in input[open..].char_indices() { let idx=open+rel; if let Some(q)=quote { if c==q {quote=None;} continue; } match c { '\''|'"'=>quote=Some(c), '{'=>depth+=1, '}'=>{depth-=1;if depth==0{return Ok(idx);}}, _=>{} } } Err("unclosed block".into()) }
fn parse_declarations(body: &str) -> Vec<(String,String)> { split_semicolons(body).into_iter().filter_map(|s| s.split_once(':').map(|(k,v)|(k.trim().into(), unquote_owned(v.trim())))).collect() }
fn split_semicolons(input: &str) -> Vec<&str> { let mut out=Vec::new(); let mut start=0; let mut paren=0i32; let mut bracket=0i32; let mut quote=None; for (i,c) in input.char_indices(){ if let Some(q)=quote{if c==q{quote=None;}continue;} match c{'\''|'"'=>quote=Some(c),'('=>paren+=1,')'=>paren-=1,'['=>bracket+=1,']'=>bracket-=1,';' if paren==0&&bracket==0=>{out.push(input[start..i].trim());start=i+1;},_=>{}}} if !input[start..].trim().is_empty(){out.push(input[start..].trim());} out }
fn parse_array(v:&str)->Vec<String>{let s=v.trim().trim_start_matches('[').trim_end_matches(']'); split_csv(s).into_iter().map(unquote_owned).filter(|s|!s.is_empty()).collect()}
fn split_csv(input:&str)->Vec<&str>{let mut out=Vec::new();let mut start=0;let mut quote=None;for(i,c)in input.char_indices(){if let Some(q)=quote{if c==q{quote=None;}continue;}match c{'\''|'"'=>quote=Some(c),','=>{out.push(input[start..i].trim());start=i+1;},_=>{}}}out.push(input[start..].trim());out}
fn unquote_owned(v:&str)->String{let s=v.trim();if s.len()>=2&&((s.starts_with('"')&&s.ends_with('"'))||(s.starts_with('\'')&&s.ends_with('\''))){s[1..s.len()-1].to_string()}else{s.to_string()}}
fn strip_hash_comments(input:&str)->String{input.lines().map(|l|l.split('#').next().unwrap_or("")).collect::<Vec<_>>().join("\n")}
fn strip_css_comments(input:&str)->String{let mut out=String::new();let mut rest=input;while let Some(start)=rest.find("/*"){out.push_str(&rest[..start]);if let Some(end)=rest[start+2..].find("*/"){rest=&rest[start+2+end+2..];}else{break;}}out.push_str(rest);out}
fn diag(level:DiagnosticLevel,message:String,line:usize)->PackDiagnostic{PackDiagnostic{level,message,span:Some(SourceSpan{line,column:1})}}

#[cfg(test)] mod tests { use super::*; #[test] fn parses_pack(){let css=r#"@view workspace { default-projection: prune; @button[name="Save"] { projection: keep; alias: save; } } @target save { match: @button[name="Save"]; } @action save { target: $save; perform: activate; fallback-keys: "ctrl+s"; }"#; let s=parse_stylesheet(css).unwrap();assert_eq!(s.views.len(),1);assert_eq!(s.targets.len(),1);assert_eq!(s.actions.len(),1);} }
