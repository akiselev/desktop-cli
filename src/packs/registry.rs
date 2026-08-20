use super::{ApplicationPack, DiagnosticLevel, PackDiagnostic};
use crate::semantic::WindowInfoV2;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PackOrigin {
    BuiltIn,
    User,
    Project,
    Explicit,
}

#[derive(Debug, Clone)]
pub struct PackEntry {
    pub id: String,
    pub path: Option<PathBuf>,
    pub origin: PackOrigin,
}

#[derive(Debug, Clone)]
pub struct PackRegistry {
    project_dir: PathBuf,
    user_dir: Option<PathBuf>,
}

impl Default for PackRegistry {
    fn default() -> Self {
        let project_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(".desktop-cli").join("packs");
        let user_dir = dirs::config_dir().map(|p| p.join("desktop-cli").join("packs"));
        Self { project_dir, user_dir }
    }
}

impl PackRegistry {
    pub fn new(project_dir: PathBuf, user_dir: Option<PathBuf>) -> Self { Self { project_dir, user_dir } }

    pub fn entries(&self) -> Vec<PackEntry> {
        let mut out = Vec::new();
        out.push(PackEntry { id: "altium".into(), path: None, origin: PackOrigin::BuiltIn });
        if let Some(user) = &self.user_dir { scan_dir(user, PackOrigin::User, &mut out); }
        scan_dir(&self.project_dir, PackOrigin::Project, &mut out);
        out.sort_by(|a,b| a.id.cmp(&b.id).then(a.origin.cmp(&b.origin)));
        out
    }

    pub fn resolve(&self, requested: &str) -> Result<ApplicationPack, Vec<PackDiagnostic>> {
        let path = Path::new(requested);
        if path.exists() || requested.contains(std::path::MAIN_SEPARATOR) {
            return ApplicationPack::load_dir(path);
        }
        if let Some(path) = self.project_dir.join(requested).canonicalize().ok().filter(|p| p.is_dir()) { return ApplicationPack::load_dir(path); }
        if let Some(user) = &self.user_dir {
            if let Some(path) = user.join(requested).canonicalize().ok().filter(|p| p.is_dir()) { return ApplicationPack::load_dir(path); }
        }
        if requested.eq_ignore_ascii_case("altium") { return super::builtin_altium(); }
        Err(vec![PackDiagnostic { level: DiagnosticLevel::Error, message: format!("pack '{}' was not found in explicit, project, user, or built-in locations", requested), span: None }])
    }

    pub fn detect(&self, platform: &str, window: &WindowInfoV2) -> Result<Option<ApplicationPack>, Vec<PackDiagnostic>> {
        let mut matches = Vec::new();
        for entry in self.entries().into_iter().rev() {
            let pack = match entry.path.as_ref() { Some(path) => ApplicationPack::load_dir(path), None => super::builtin_altium() }?;
            let Some(rule) = pack.manifest.detect.get(platform) else { continue; };
            let title_match = rule.titles.is_empty() || rule.titles.iter().any(|p| wildcard(p, &window.title));
            let exe_match = rule.executables.is_empty() || rule.executables.iter().any(|p| wildcard(p, &window.executable));
            let bundle_match = rule.bundle_ids.is_empty() || window.bundle_id.as_ref().map(|id| rule.bundle_ids.iter().any(|p| wildcard(p,id))).unwrap_or(false);
            if title_match && exe_match && bundle_match { matches.push((entry.origin, pack)); }
        }
        matches.sort_by_key(|(origin,_)| *origin);
        Ok(matches.pop().map(|(_,pack)| pack))
    }
}

fn scan_dir(root: &Path, origin: PackOrigin, out: &mut Vec<PackEntry>) {
    let Ok(entries) = std::fs::read_dir(root) else { return; };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() || !path.join("pack.toml").is_file() || !path.join("views.dcss").is_file() { continue; }
        if let Ok(text) = std::fs::read_to_string(path.join("pack.toml")) {
            if let Ok(manifest) = super::parse_manifest(&text) { out.push(PackEntry { id: manifest.id, path: Some(path), origin }); }
        }
    }
}

fn wildcard(pattern:&str,value:&str)->bool {
    let pattern=pattern.to_ascii_lowercase(); let value=value.to_ascii_lowercase();
    let p:Vec<char>=pattern.chars().collect(); let v:Vec<char>=value.chars().collect(); let mut dp=vec![vec![false;v.len()+1];p.len()+1]; dp[0][0]=true;
    for i in 1..=p.len(){if p[i-1]=='*'{dp[i][0]=dp[i-1][0];}}
    for i in 1..=p.len(){for j in 1..=v.len(){dp[i][j]=match p[i-1]{'*'=>dp[i-1][j]||dp[i][j-1],'?'=>dp[i-1][j-1],c=>c==v[j-1]&&dp[i-1][j-1]};}}
    dp[p.len()][v.len()]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_path_beats_registry() {
        let tmp=std::env::temp_dir().join(format!("desktop-cli-pack-registry-{}",std::process::id()));
        let pack_dir=tmp.join("project").join("x"); std::fs::create_dir_all(&pack_dir).unwrap();
        std::fs::write(pack_dir.join("pack.toml"),"schema=1\nid=\"x\"\nname=\"X\"\n[view]\ndefault=\"main\"\n").unwrap();
        std::fs::write(pack_dir.join("views.dcss"),"@view main { default-projection: keep; }").unwrap();
        let registry=PackRegistry::new(tmp.join("project"),None); assert_eq!(registry.resolve("x").unwrap().manifest.id,"x");
        let _=std::fs::remove_dir_all(tmp);
    }
}
