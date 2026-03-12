use core::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PrintEvent {
    pub template: String,
    pub output: Output,
}

impl PrintEvent {
    pub fn run(&self, data: &str) {
        match self.output {
            Output::Stdout => println!("{data}"),
            Output::Stderr => eprintln!("{data}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Output {
    #[default]
    Stdout,
    Stderr,
}

impl FromStr for Output {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "stdout" => Output::Stdout,
            "stderr" => Output::Stderr,
            _ => return Err(format!("Invalid output {s}")),
        })
    }
}
