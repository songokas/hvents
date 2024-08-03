use serde::{Deserialize, Serialize};

use super::data::Data;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintEvent(Output);

impl PrintEvent {
    pub fn run(&self, data: &Data) {
        match self.0 {
            Output::Stdout => println!("{data:?}"),
            Output::Stderr => eprintln!("{data:?}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Output {
    Stdout,
    Stderr,
}
