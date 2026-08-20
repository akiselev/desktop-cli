use super::model::{CapabilityKind, RelationKind, Role, State, StateValue};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Combinator { Descendant, Child }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MatchOp { Exact, Contains, StartsWith, EndsWith, Wildcard }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyPredicate { pub name: String, pub op: MatchOp, pub value: String }
impl PropertyPredicate { pub fn matches(&self, actual: &str) -> bool { match self.op { MatchOp::Exact => actual == self.value, MatchOp::Contains => actual.contains(&self.value), MatchOp::StartsWith => actual.starts_with(&self.value), MatchOp::EndsWith => actual.ends_with(&self.value), MatchOp::Wildcard => wildcard_match(&self.value, actual) } } }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatePredicate { pub state: State, pub expected: StateValue }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpatialRelation { Near, Below, Above, Left, Right, Inside }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FunctionalPredicate { Is(Vec<Selector>), Not(Box<Selector>), Has(Box<Selector>), Relation { kind: RelationKind, selector: Box<Selector> }, Spatial { relation: SpatialRelation, selector: Box<Selector> } }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectorTest { Role(Role), StableId(String), Property(PropertyPredicate), State(StatePredicate), Capability(CapabilityKind), Functional(FunctionalPredicate), Alias(String) }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectorStep { pub combinator: Option<Combinator>, pub tests: Vec<SelectorTest> }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selector { pub steps: Vec<SelectorStep>, pub source: String }

impl Selector {
    pub fn parse(input: &str) -> Result<Self, String> {
        let input = input.trim(); if input.is_empty() { return Err("empty selector".into()); }
        let parts = split_steps(input)?; let mut steps = Vec::new();
        for (index, (combinator, text)) in parts.into_iter().enumerate() { let tests = parse_compound(&text)?; if tests.is_empty() { return Err(format!("selector step '{}' has no tests", text)); } steps.push(SelectorStep { combinator: if index == 0 { None } else { combinator }, tests }); }
        Ok(Self { steps, source: input.to_string() })
    }
    pub fn is_alias_only(&self) -> Option<&str> { if self.steps.len() == 1 && self.steps[0].tests.len() == 1 { if let SelectorTest::Alias(name) = &self.steps[0].tests[0] { return Some(name); } } None }
}

fn split_steps(input: &str) -> Result<Vec<(Option<Combinator>, String)>, String> {
    let mut result=Vec::new();let mut current=String::new();let mut bracket=0i32;let mut paren=0i32;let mut quote:Option<char>=None;let mut next_comb=None;let chars:Vec<char>=input.chars().collect();let mut i=0;
    while i<chars.len(){let c=chars[i];if let Some(q)=quote{current.push(c);if c==q&&(i==0||chars[i-1]!='\\'){quote=None;}i+=1;continue;}match c{'\''|'"'=>{quote=Some(c);current.push(c);},'['=>{bracket+=1;current.push(c);},']'=>{bracket-=1;if bracket<0{return Err("unbalanced ']'".into());}current.push(c);},'('=>{paren+=1;current.push(c);},')'=>{paren-=1;if paren<0{return Err("unbalanced ')'".into());}current.push(c);},'>' if bracket==0&&paren==0=>{if !current.trim().is_empty(){result.push((next_comb.take(),current.trim().to_string()));current.clear();}next_comb=Some(Combinator::Child);},c if c.is_whitespace()&&bracket==0&&paren==0=>{if !current.trim().is_empty(){result.push((next_comb.take(),current.trim().to_string()));current.clear();if next_comb.is_none(){next_comb=Some(Combinator::Descendant);}}while i+1<chars.len()&&chars[i+1].is_whitespace(){i+=1;}},_=>current.push(c)}i+=1;}
    if quote.is_some()||bracket!=0||paren!=0{return Err("unbalanced selector delimiters".into());}if !current.trim().is_empty(){result.push((next_comb.take(),current.trim().to_string()));}if result.is_empty(){return Err("empty selector".into());}Ok(result)
}

fn parse_compound(input:&str)->Result<Vec<SelectorTest>,String>{let chars:Vec<char>=input.chars().collect();let mut tests=Vec::new();let mut i=0;while i<chars.len(){match chars[i]{'@'=>{let(name,next)=read_ident(&chars,i+1);if name.is_empty(){return Err("missing role after @".into());}let role=Role::parse(&name);if role==Role::Unknown&&name!="unknown"{return Err(format!("unknown semantic role @{}",name));}tests.push(SelectorTest::Role(role));i=next;},'#'=>{let(id,next)=read_ident(&chars,i+1);if id.is_empty(){return Err("missing stable id after #".into());}tests.push(SelectorTest::StableId(id));i=next;},'$'=>{let(alias,next)=read_ident(&chars,i+1);if alias.is_empty(){return Err("missing alias after $".into());}tests.push(SelectorTest::Alias(alias));i=next;},'['=>{let(content,next)=read_balanced(&chars,i,'[',']')?;tests.push(parse_attribute(&content[1..content.len()-1])?);i=next;},':'=>{let(name,after_name)=read_ident(&chars,i+1);if name.is_empty(){return Err("missing pseudo-class name".into());}i=after_name;if i<chars.len()&&chars[i]=='(' {let(content,next)=read_balanced(&chars,i,'(',')')?;let inner=&content[1..content.len()-1];tests.push(parse_function(&name,inner)?);i=next;}else{tests.push(parse_state_pseudo(&name)?);}},c if c.is_whitespace()=>i+=1,_=>{let(name,next)=read_ident(&chars,i);if name.is_empty(){return Err(format!("unexpected character '{}' in selector",chars[i]));}let role=Role::parse(&name);if role==Role::Unknown{return Err(format!("unknown legacy control type '{}'",name));}tests.push(SelectorTest::Role(role));i=next;}}}Ok(tests)}

fn parse_attribute(content:&str)->Result<SelectorTest,String>{let content=content.trim();if let Some(rest)=content.strip_prefix("state."){let(name,value)=rest.split_once('=').ok_or_else(||"state attribute requires '='".to_string())?;let state=State::parse(name).ok_or_else(||format!("unknown state '{}'",name))?;let expected=match unquote(value).to_ascii_lowercase().as_str(){"true"|"1"|"yes"=>StateValue::True,"false"|"0"|"no"=>StateValue::False,"unknown"|"?"=>StateValue::Unknown,other=>return Err(format!("invalid state value '{}'",other))};return Ok(SelectorTest::State(StatePredicate{state,expected}));}let operators=[("*=",MatchOp::Contains),("^=",MatchOp::StartsWith),("$=",MatchOp::EndsWith),("~=",MatchOp::Wildcard),("=",MatchOp::Exact)];for(token,op)in operators{if let Some((name,value))=content.split_once(token){return Ok(SelectorTest::Property(PropertyPredicate{name:name.trim().to_string(),op,value:unquote(value).to_string()}));}}Err(format!("invalid attribute selector [{}]",content))}
fn parse_state_pseudo(name:&str)->Result<SelectorTest,String>{if name=="disabled"{return Ok(SelectorTest::State(StatePredicate{state:State::Enabled,expected:StateValue::False}));}if name=="collapsed"{return Ok(SelectorTest::State(StatePredicate{state:State::Expanded,expected:StateValue::False}));}let state=State::parse(name).ok_or_else(||format!("unknown pseudo-class :{}",name))?;Ok(SelectorTest::State(StatePredicate{state,expected:StateValue::True}))}
fn parse_function(name:&str,inner:&str)->Result<SelectorTest,String>{match name{"supports"=>{let cap=CapabilityKind::parse(inner.trim()).ok_or_else(||format!("unknown capability '{}'",inner.trim()))?;Ok(SelectorTest::Capability(cap))},"is"=>{let mut sels=Vec::new();for part in split_top_level_commas(inner){sels.push(Selector::parse(part)?);}Ok(SelectorTest::Functional(FunctionalPredicate::Is(sels)))},"not"=>Ok(SelectorTest::Functional(FunctionalPredicate::Not(Box::new(Selector::parse(inner)?)))),"has"=>Ok(SelectorTest::Functional(FunctionalPredicate::Has(Box::new(Selector::parse(inner)?)))),"labelled-by"|"label-for"|"controlled-by"|"controls"|"described-by"|"error-for"|"popup-for"=>{let kind=match name{"labelled-by"=>RelationKind::LabelledBy,"label-for"=>RelationKind::LabelFor,"controlled-by"=>RelationKind::ControlledBy,"controls"=>RelationKind::Controls,"described-by"=>RelationKind::DescribedBy,"error-for"=>RelationKind::ErrorFor,_=>RelationKind::PopupFor};Ok(SelectorTest::Functional(FunctionalPredicate::Relation{kind,selector:Box::new(Selector::parse(inner)?)}))},"near"|"below"|"above"|"left"|"right"|"inside"=>{let relation=match name{"near"=>SpatialRelation::Near,"below"=>SpatialRelation::Below,"above"=>SpatialRelation::Above,"left"=>SpatialRelation::Left,"right"=>SpatialRelation::Right,_=>SpatialRelation::Inside};Ok(SelectorTest::Functional(FunctionalPredicate::Spatial{relation,selector:Box::new(Selector::parse(inner)?)}))},_=>Err(format!("unknown functional pseudo-class :{}()",name))}}
fn read_ident(chars:&[char],mut i:usize)->(String,usize){let start=i;while i<chars.len()&&(chars[i].is_alphanumeric()||matches!(chars[i],'-'|'_'|'.')){i+=1;}(chars[start..i].iter().collect(),i)}
fn read_balanced(chars:&[char],start:usize,open:char,close:char)->Result<(String,usize),String>{let mut depth=0;let mut quote=None;let mut i=start;while i<chars.len(){let c=chars[i];if let Some(q)=quote{if c==q&&(i==0||chars[i-1]!='\\'){quote=None;}}else if c=='\''||c=='"'{quote=Some(c);}else if c==open{depth+=1;}else if c==close{depth-=1;if depth==0{return Ok((chars[start..=i].iter().collect(),i+1));}}i+=1;}Err(format!("unclosed '{}'",open))}
fn split_top_level_commas(input:&str)->Vec<&str>{let bytes=input.as_bytes();let mut result=Vec::new();let mut start=0;let mut paren=0i32;let mut bracket=0i32;let mut quote:Option<u8>=None;for(i,b)in bytes.iter().copied().enumerate(){if let Some(q)=quote{if b==q&&(i==0||bytes[i-1]!=b'\\'){quote=None;}continue;}match b{b'\''|b'"'=>quote=Some(b),b'('=>paren+=1,b')'=>paren-=1,b'['=>bracket+=1,b']'=>bracket-=1,b',' if paren==0&&bracket==0=>{result.push(input[start..i].trim());start=i+1;},_=>{}}}result.push(input[start..].trim());result.into_iter().filter(|p|!p.is_empty()).collect()}
fn unquote(input:&str)->&str{let s=input.trim();if s.len()>=2&&((s.starts_with('"')&&s.ends_with('"'))||(s.starts_with('\'')&&s.ends_with('\''))){&s[1..s.len()-1]}else{s}}
fn wildcard_match(pattern:&str,value:&str)->bool{let p:Vec<char>=pattern.chars().collect();let v:Vec<char>=value.chars().collect();let mut dp=vec![vec![false;v.len()+1];p.len()+1];dp[0][0]=true;for i in 1..=p.len(){if p[i-1]=='*'{dp[i][0]=dp[i-1][0];}}for i in 1..=p.len(){for j in 1..=v.len(){dp[i][j]=match p[i-1]{'*'=>dp[i-1][j]||dp[i][j-1],'?'=>dp[i-1][j-1],c=>c==v[j-1]&&dp[i-1][j-1]};}}dp[p.len()][v.len()]}

#[cfg(test)]mod tests{use super::*;#[test]fn parses_semantic_selector(){let selector=Selector::parse("@button:enabled[name*=\"Save\"]").unwrap();assert_eq!(selector.steps.len(),1);assert_eq!(selector.steps[0].tests.len(),3);}#[test]fn parses_nested_function(){let selector=Selector::parse("@panel:has(@input[name=\"Width\"])").unwrap();assert_eq!(selector.steps.len(),1);}#[test]fn wildcard_works(){assert!(wildcard_match("*Save?","Quick Save!"));}}
