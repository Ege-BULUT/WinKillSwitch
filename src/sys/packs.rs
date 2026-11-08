//! Rule packs: alerts and limits in one JSON file you can share. Importing merges a pack into
//! your rules; removing a pack takes out exactly the rules it added and you have not changed.
//! The built-in profiles are the files in `packs/`, embedded at build time.

use serde::{Deserialize, Serialize};

use crate::sys::alerts::Alerts;
use crate::sys::limits::Limit;

pub const FORMAT: &str = "winkillswitch-rules";
const MAX_RULES: usize = 200;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pack {
    pub format: String,
    pub version: u32,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub limits: Vec<Limit>,
    #[serde(default)]
    pub watch: Vec<String>,
    /// Some(true) turns the Downloads/Temp alert on; None leaves it as it is.
    #[serde(default)]
    pub risky_folders: Option<bool>,
}

pub fn builtin() -> Vec<Pack> {
    [include_str!("../../packs/gaming-mode.json"), include_str!("../../packs/bloat-blocker.json"), include_str!("../../packs/quiet-laptop.json")]
        .iter()
        .map(|j| parse(j).expect("built-in packs are valid"))
        .collect()
}

/// An image name a rule may hold: no path, no control characters, `*` allowed for watches.
fn clean_name(s: &str) -> Option<String> {
    let t = s.trim();
    let ok = !t.is_empty() && t.len() <= 128 && !t.chars().any(|c| c.is_control() || matches!(c, '\\' | '/' | ':' | '"' | '<' | '>' | '|'));
    ok.then(|| t.to_string())
}

/// Read and check a pack. Limits are clamped to sane bounds; bad names are an error, so a pack
/// never half applies.
pub fn parse(json: &str) -> Result<Pack, String> {
    let mut p: Pack = serde_json::from_str(json).map_err(|e| format!("This is not a rule pack ({e})."))?;
    if p.format != FORMAT {
        return Err("This JSON file is not a WinKillSwitch rule pack.".into());
    }
    if p.version > 1 {
        return Err(format!("This pack needs a newer WinKillSwitch (format version {}).", p.version));
    }
    if p.limits.len() + p.watch.len() > MAX_RULES {
        return Err(format!("This pack has more than {MAX_RULES} rules."));
    }
    p.name = p.name.trim().chars().take(60).collect();
    if p.name.is_empty() {
        p.name = "Imported rules".into();
    }
    for l in &mut p.limits {
        l.name = clean_name(&l.name).filter(|n| !n.contains('*')).ok_or_else(|| format!("A limit has an invalid program name: \"{}\".", l.name))?;
        l.cpu = l.cpu.map(|c| c.clamp(1, 100));
        l.mem_mb = l.mem_mb.map(|m| m.clamp(128, 1 << 20));
    }
    p.limits.retain(|l| l.cpu.is_some() || l.mem_mb.is_some());
    for w in &mut p.watch {
        *w = clean_name(w).ok_or_else(|| format!("A watch rule is invalid: \"{w}\"."))?;
    }
    Ok(p)
}

/// Your current rules as a pack.
pub fn export(name: &str, alerts: &Alerts, limits: &[Limit]) -> Pack {
    Pack {
        format: FORMAT.into(),
        version: 1,
        name: name.into(),
        description: "Exported from WinKillSwitch.".into(),
        limits: limits.to_vec(),
        watch: alerts.watch.clone(),
        risky_folders: alerts.risky_folders.then_some(true),
    }
}

/// Merge `p` in: its limits replace yours for the same program, its watches are added.
/// Returns how many rules it added or changed.
pub fn apply(p: &Pack, alerts: &mut Alerts, limits: &mut Vec<Limit>) -> usize {
    let mut n = 0;
    for l in &p.limits {
        match limits.iter_mut().find(|x| x.name.eq_ignore_ascii_case(&l.name)) {
            Some(x) if x == l => {}
            Some(x) => {
                *x = l.clone();
                n += 1;
            }
            None => {
                limits.push(l.clone());
                n += 1;
            }
        }
    }
    for w in &p.watch {
        if !alerts.watch.iter().any(|x| x.eq_ignore_ascii_case(w)) {
            alerts.watch.push(w.clone());
            n += 1;
        }
    }
    if p.risky_folders == Some(true) && !alerts.risky_folders {
        alerts.risky_folders = true;
        n += 1;
    }
    n
}

/// Every rule of `p` is in place.
pub fn is_applied(p: &Pack, alerts: &Alerts, limits: &[Limit]) -> bool {
    p.limits.iter().all(|l| limits.iter().any(|x| x.name.eq_ignore_ascii_case(&l.name) && x.cpu == l.cpu && x.mem_mb == l.mem_mb))
        && p.watch.iter().all(|w| alerts.watch.iter().any(|x| x.eq_ignore_ascii_case(w)))
        && (p.risky_folders != Some(true) || alerts.risky_folders)
}

/// Take out the rules `p` added, leaving any you changed since.
pub fn remove(p: &Pack, alerts: &mut Alerts, limits: &mut Vec<Limit>) {
    limits.retain(|x| !p.limits.iter().any(|l| x.name.eq_ignore_ascii_case(&l.name) && x.cpu == l.cpu && x.mem_mb == l.mem_mb));
    alerts.watch.retain(|x| !p.watch.iter().any(|w| x.eq_ignore_ascii_case(w)));
    if p.risky_folders == Some(true) {
        alerts.risky_folders = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_parse() {
        let b = builtin();
        assert_eq!(b.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["Gaming mode", "Bloat blocker", "Quiet laptop"]);
        assert!(b.iter().all(|p| !p.limits.is_empty()));
    }

    #[test]
    fn rejects_and_clamps() {
        assert!(parse("{}").is_err());
        assert!(parse(r#"{"format":"other","version":1,"name":"x"}"#).is_err());
        assert!(parse(r#"{"format":"winkillswitch-rules","version":9,"name":"x"}"#).is_err());
        assert!(parse(r#"{"format":"winkillswitch-rules","version":1,"name":"x","limits":[{"name":"C:\\evil.exe","cpu":5,"mem_mb":null}]}"#).is_err());
        let p = parse(r#"{"format":"winkillswitch-rules","version":1,"name":"  ","limits":[{"name":"a.exe","cpu":250,"mem_mb":1},{"name":"b.exe","cpu":null,"mem_mb":null}]}"#).unwrap();
        assert_eq!(p.name, "Imported rules");
        assert_eq!(p.limits, vec![Limit { name: "a.exe".into(), cpu: Some(100), mem_mb: Some(128) }]);
    }

    #[test]
    fn apply_then_remove_round_trips_and_keeps_your_changes() {
        let pack = builtin().remove(1); // Bloat blocker: limits, watches and the risky switch
        let mine = Limit { name: "steam.exe".into(), cpu: Some(50), mem_mb: None };
        let mut alerts = Alerts { risky_folders: false, watch: vec!["discord".into()] };
        let mut limits = vec![mine.clone()];
        assert!(!is_applied(&pack, &alerts, &limits));
        let n = apply(&pack, &mut alerts, &mut limits);
        assert_eq!(n, pack.limits.len() + pack.watch.len() + 1);
        assert!(is_applied(&pack, &alerts, &limits));
        assert_eq!(apply(&pack, &mut alerts, &mut limits), 0, "applying twice changes nothing");
        // You change one of the pack's limits; removing the pack leaves it.
        limits.iter_mut().find(|l| l.name == "updater.exe").unwrap().cpu = Some(50);
        remove(&pack, &mut alerts, &mut limits);
        assert_eq!(alerts, Alerts { risky_folders: false, watch: vec!["discord".into()] });
        assert_eq!(limits.len(), 2);
        assert!(limits.contains(&mine));
        assert!(limits.iter().any(|l| l.name == "updater.exe" && l.cpu == Some(50)));
    }

    #[test]
    fn export_reads_back() {
        let alerts = Alerts { risky_folders: true, watch: vec!["steam*".into()] };
        let limits = vec![Limit { name: "a.exe".into(), cpu: Some(25), mem_mb: Some(1024) }];
        let json = serde_json::to_string_pretty(&export("Mine", &alerts, &limits)).unwrap();
        let p = parse(&json).unwrap();
        assert_eq!((p.limits, p.watch, p.risky_folders), (limits, alerts.watch, Some(true)));
    }
}
