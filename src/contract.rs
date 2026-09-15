use serde::Serialize;
use serde_json::Value;

use crate::error::{Error, ErrorKind};
use crate::probe::Probe;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ok,
    Failed,
    DryRun,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorBody {
    pub kind: ErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Contract {
    pub status: Status,
    pub tool: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub probe: Option<Probe>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<Value>,
}

impl Contract {
    pub fn ok(tool: impl Into<String>, output: Option<String>, probe: Option<Probe>) -> Self {
        let summary = probe.as_ref().map(Probe::summary);
        Self {
            status: Status::Ok,
            tool: tool.into(),
            output,
            probe,
            commands: Vec::new(),
            error: None,
            summary,
            verified: Some(true),
            extra: None,
        }
    }

    pub fn dry_run(tool: impl Into<String>, output: Option<String>, probe: Option<Probe>) -> Self {
        let mut c = Self::ok(tool, output, probe);
        c.status = Status::DryRun;
        c.verified = None;
        c
    }

    pub fn failed(tool: impl Into<String>, err: &Error) -> Self {
        Self {
            status: Status::Failed,
            tool: tool.into(),
            output: None,
            probe: None,
            commands: Vec::new(),
            error: Some(ErrorBody {
                kind: err.kind_of(),
                message: err.message(),
            }),
            summary: None,
            verified: Some(false),
            extra: None,
        }
    }

    pub fn with_commands(mut self, commands: Vec<Vec<String>>) -> Self {
        self.commands = commands;
        self
    }

    pub fn with_extra(mut self, extra: Value) -> Self {
        self.extra = Some(extra);
        self
    }

    pub fn brief(&self) -> BriefContract<'_> {
        BriefContract {
            status: self.status,
            tool: &self.tool,
            output: self.output.as_deref(),
            summary: self.summary.as_deref(),
            verified: self.verified,
            error: self.error.as_ref(),
        }
    }
}

#[derive(Serialize)]
pub struct BriefContract<'a> {
    pub status: Status,
    pub tool: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<&'a ErrorBody>,
}

#[derive(Clone, Copy)]
pub enum EmitStyle {
    Human,
    Json,
    JsonBrief,
}

impl EmitStyle {
    pub fn emit(self, contract: &Contract) {
        match self {
            Self::Json => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(contract).expect("contract serializes")
                );
            }
            Self::JsonBrief => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&contract.brief()).expect("brief serializes")
                );
            }
            Self::Human => match contract.status {
                Status::Failed => {
                    if let Some(err) = &contract.error {
                        eprintln!("failed {}: {}", contract.tool, err.message);
                    } else {
                        eprintln!("failed {}", contract.tool);
                    }
                }
                Status::DryRun => {
                    let dest = contract.output.as_deref().unwrap_or("-");
                    let sum = contract.summary.as_deref().unwrap_or("");
                    println!("dry-run {} → {dest}  {sum}", contract.tool);
                    for cmd in &contract.commands {
                        println!("  {}", cmd.join(" "));
                    }
                }
                Status::Ok => {
                    let dest = contract.output.as_deref().unwrap_or("-");
                    let sum = contract.summary.as_deref().unwrap_or("");
                    println!("ok {} → {dest}  {sum}", contract.tool);
                }
            },
        }
    }
}
