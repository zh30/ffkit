use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Deserialize;

use crate::cli::Globals;
use crate::contract::Contract;
use crate::doctor;
use crate::engine::{self, ffmpeg_base};
use crate::error::Error;
use crate::paths;

#[derive(Deserialize)]
struct Plan {
    inputs: Vec<PathBuf>,
    #[serde(default)]
    filter_complex: Vec<Node>,
    #[serde(default)]
    map: Vec<String>,
    output: PathBuf,
    #[serde(default)]
    args: BTreeMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct Node {
    filter: String,
    #[serde(default, rename = "in")]
    ins: Vec<String>,
    #[serde(default)]
    opts: BTreeMap<String, serde_json::Value>,
    #[serde(default, rename = "out")]
    outs: Vec<String>,
}

pub fn run(plan_path: PathBuf, g: &Globals) -> Result<Contract, Error> {
    paths::ensure_input(&plan_path)?;
    let raw = std::fs::read_to_string(&plan_path)?;
    let plan: Plan =
        serde_json::from_str(&raw).map_err(|e| Error::input(format!("graph plan: {e}")))?;
    if plan.inputs.is_empty() {
        return Err(Error::input("graph plan needs at least one input"));
    }

    let known = doctor::list_filters()?;
    for node in &plan.filter_complex {
        if !known.contains(&node.filter) {
            return Err(Error::input(format!(
                "unknown filter '{}'; not in this ffmpeg build",
                node.filter
            )));
        }
    }

    let mut argv = ffmpeg_base(g.progress);
    for input in &plan.inputs {
        paths::ensure_input(input)?;
        argv.push("-i");
        argv.push(input);
    }

    if !plan.filter_complex.is_empty() {
        let fc = render_filter_complex(&plan.filter_complex);
        argv.extend(["-filter_complex", &fc]);
    }
    for m in &plan.map {
        argv.push("-map");
        if m.starts_with('[') || m.contains(':') {
            argv.push(m);
        } else {
            argv.push(format!("[{m}]"));
        }
    }
    for (k, v) in &plan.args {
        let flag = if k.starts_with('-') {
            k.clone()
        } else {
            format!("-{k}")
        };
        argv.push(flag);
        argv.push(json_to_arg(v));
    }
    argv.push(&plan.output);

    let refs: Vec<&std::path::Path> = plan.inputs.iter().map(|p| p.as_path()).collect();
    engine::write_job("graph", &refs, &plan.output, vec![argv], g)
}

fn render_filter_complex(nodes: &[Node]) -> String {
    let mut parts = Vec::new();
    for node in nodes {
        let mut s = String::new();
        for i in &node.ins {
            s.push_str(&label(i));
        }
        s.push_str(&node.filter);
        if !node.opts.is_empty() {
            s.push('=');
            let opts: Vec<String> = node
                .opts
                .iter()
                .map(|(k, v)| format!("{k}={}", json_to_arg(v)))
                .collect();
            s.push_str(&opts.join(":"));
        }
        for o in &node.outs {
            s.push_str(&label(o));
        }
        parts.push(s);
    }
    parts.join(";")
}

fn label(s: &str) -> String {
    if s.starts_with('[') {
        s.to_string()
    } else {
        format!("[{s}]")
    }
}

fn json_to_arg(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        other => other.to_string(),
    }
}
