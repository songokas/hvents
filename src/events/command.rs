use std::{
    io::Write,
    process::{Command, Stdio},
};

use anyhow::Result;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::data::{Data, DataType, Metadata};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandEvent {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub replace_args: IndexMap<usize, String>,
    #[serde(default)]
    pub vars: IndexMap<String, String>,
    #[serde(default)]
    pub data_type: DataType,
}

impl CommandEvent {
    pub fn run(&self, data: &Data) -> Result<(Data, Metadata)> {
        let child = Command::new(&self.command)
            .args(&self.args)
            .envs(&self.vars)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        child.stdin.expect("stdin").write_all(&data.as_bytes()?)?;
        let reader = child.stdout.expect("stdout");
        Ok((
            Data::from_reader(reader, self.data_type)?,
            Metadata::default(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::indexmap;

    #[test]
    fn test_xargs_command() {
        let event = CommandEvent {
            command: "xargs".to_string(),
            args: ["echo".to_string(), "-n".to_string()].to_vec(),
            vars: Default::default(),
            data_type: DataType::String,
            replace_args: Default::default(),
        };

        let input = Data::String("hello".to_string());

        let (output, _) = event.run(&input).unwrap();
        assert_eq!(output, Data::String("hello".to_string()));
    }

    #[test]
    fn test_echo_command() {
        let event = CommandEvent {
            command: "echo".to_string(),
            args: ["-n".to_string(), "hello".to_string()].to_vec(),
            vars: Default::default(),
            data_type: DataType::Bytes,
            replace_args: Default::default(),
        };

        let input = Data::Empty;

        let (output, _) = event.run(&input).unwrap();
        assert_eq!(output, Data::Bytes(b"hello".to_vec()));
    }

    #[test]
    fn test_printenv_command() {
        let event = CommandEvent {
            command: "printenv".to_string(),
            args: ["TEST1".to_string()].to_vec(),
            vars: indexmap! {
                "TEST1".to_string() => "defined".to_string()
            },
            data_type: DataType::String,
            replace_args: Default::default(),
        };

        let input = Data::Empty;

        let (output, _) = event.run(&input).unwrap();
        assert_eq!(output, Data::String("defined\n".to_string()));
    }
}
