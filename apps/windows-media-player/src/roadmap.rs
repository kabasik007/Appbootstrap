//! Offline, deterministic task/roadmap model embedded from roadmap/roadmap.json.
//! This does not claim the work is done: "implemented_unverified" != "verified".
use std::collections::HashSet;
use serde_json::Value;

#[derive(Debug)]
pub struct RoadmapView {
    pub milestones: Vec<String>,
    pub focus_tasks: Vec<String>,
    pub milestone_summary: String,
    pub task_summary: String,
    pub next_gate: String,
}

const ALLOWED: &[&str] = &["todo", "planned", "in_progress", "implemented_unverified", "blocked", "verified"];

pub fn bundled() -> Result<RoadmapView, String> {
    parse(include_str!("../roadmap/roadmap.json"))
}

fn field<'a>(value: &'a Value, name: &str) -> Result<&'a str, String> {
    let text = value.get(name).and_then(Value::as_str)
        .ok_or_else(|| format!("Missing string field {name}"))?;
    if text.is_empty() || text.len() > 240 {
        return Err(format!("Invalid {name} length"));
    }
    Ok(text)
}

pub fn parse(source: &str) -> Result<RoadmapView, String> {
    let json: Value = serde_json::from_str(source).map_err(|e| e.to_string())?;
    if json.get("schema_version").and_then(Value::as_u64) != Some(1) {
        return Err("Unsupported roadmap schema version".into());
    }
    let phases = json.get("phases").and_then(Value::as_array)
        .ok_or("Missing phases array")?;
    if phases.is_empty() || phases.len() > 20 {
        return Err("Invalid phase count".into());
    }

    let mut phase_ids = HashSet::new();
    let mut task_ids = HashSet::new();
    let mut milestones = Vec::with_capacity(phases.len());
    let mut tasks = Vec::new();
    let mut coded = 0usize;
    let mut verified = 0usize;
    let mut blocked = 0usize;
    let mut active = 0usize;

    for phase in phases {
        let phase_id = field(phase, "id")?;
        let name = field(phase, "name")?;
        let status = field(phase, "status")?;
        if !ALLOWED.contains(&status) || !phase_ids.insert(phase_id.to_owned()) {
            return Err(format!("Invalid or duplicate phase: {phase_id}"));
        }
        let label = status_label(status);
        if status == "in_progress" { active += 1; }
        milestones.push(format!("{phase_id}    {name}  —  {label}"));

        let entries = phase.get("tasks").and_then(Value::as_array)
            .ok_or_else(|| format!("{phase_id}: missing tasks"))?;
        if entries.is_empty() || entries.len() > 40 {
            return Err(format!("{phase_id}: invalid task count"));
        }
        for task in entries {
            let id = field(task, "id")?;
            let title = field(task, "title")?;
            let priority = field(task, "priority")?;
            let status = field(task, "status")?;
            let _verification = field(task, "verification")?;
            if !["blocker","high","medium","low"].contains(&priority) ||
                !ALLOWED.contains(&status) || !task_ids.insert(id.to_owned())
            {
                return Err(format!("Invalid/duplicate task {id}"));
            }
            if status == "implemented_unverified" { coded += 1; }
            if status == "verified" { verified += 1; }
            if status == "blocked" { blocked += 1; }

            // Put the urgent unverified build / real-time tasks first.
            let score = match priority {
                "blocker" => 0,
                "high" => 1,
                "medium" => 2,
                _ => 3,
            } + match status {
                "verified" => 100,
                "implemented_unverified" => 6,
                _ => 0,
            };
            tasks.push((score, id.to_owned(), format!(
                "{id}   {title}  ·  {}  ·  {}",
                priority.to_uppercase(), status_label(status)
            )));
        }
    }
    if tasks.len() > 600 {
        return Err("Too many tasks".into());
    }
    tasks.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    // Show only 8 focus items on the fixed-height Tasks screen; all remain in JSON.
    let focus_tasks = tasks.into_iter().take(8).map(|(_, _, text)| text).collect();
    Ok(RoadmapView {
        milestone_summary: format!("{} PHASES  ·  {active} ACTIVE  ·  {verified} VERIFIED",phases.len()),
        task_summary: format!("{} TASKS  ·  {coded} CODE-ONLY  ·  {blocked} BLOCKED",task_ids.len()),
        next_gate: "NEXT GATE  ·  Windows cargo check + cargo test + audible playback".into(),
        milestones,
        focus_tasks,
    })
}

fn status_label(status: &str) -> &'static str {
    match status {
        "todo" | "planned" => "PLANNED",
        "in_progress" => "IN PROGRESS",
        "implemented_unverified" => "CODE / UNVERIFIED",
        "blocked" => "BLOCKED",
        "verified" => "VERIFIED",
        _ => "INVALID",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_plan_is_well_formed() {
        let plan = bundled().unwrap();
        assert_eq!(plan.milestones.len(), 8);
        assert_eq!(plan.focus_tasks.len(), 8);
        assert!(plan.task_summary.contains("34 TASKS"));
        assert!(plan.next_gate.contains("cargo"));
    }

    #[test]
    fn reject_duplicate_identifiers() {
        let bad = r#"{"schema_version":1,"phases":[{"id":"P0","name":"One","status":"todo","tasks":[{"id":"T1","title":"Task","priority":"high","status":"todo","verification":"test"},{"id":"T1","title":"Again","priority":"high","status":"todo","verification":"test"}]}]}"#;
        assert!(parse(bad).is_err());
    }

    #[test]
    fn reject_unknown_status_or_schema() {
        let unknown = r#"{"schema_version":5,"phases":[]}"#;
        assert!(parse(unknown).is_err());
        let bad_status = r#"{"schema_version":1,"phases":[{"id":"P0","name":"One","status":"shipped","tasks":[{"id":"T1","title":"Task","priority":"high","status":"todo","verification":"test"}]}]}"#;
        assert!(parse(bad_status).is_err());
    }
}
