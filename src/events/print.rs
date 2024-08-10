use serde::{Deserialize, Serialize};

use super::data::Data;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PrintEvent(Output);

impl PrintEvent {
    pub fn run(&self, data: &Data) {
        match self.0 {
            Output::Stdout => println!("{data:?}"),
            Output::Stderr => eprintln!("{data:?}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
enum Output {
    #[default]
    Stdout,
    Stderr,
}
