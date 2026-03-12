use std::collections::HashMap;

use anyhow::{Result, bail};
use regex::Regex;

pub type Fields = HashMap<String, Vec<usize>>;

#[derive(Debug)]
pub struct ParsedFieldExpression {
    pub fields: Fields,
    pub regex: String,
}

#[derive(Debug)]
pub struct FieldExpression {
    pub fields: Fields,
    pub regex: Regex,
}

pub fn parse_expression(
    regex: String,
    field_aliases: HashMap<String, String>,
    group_sizes: HashMap<String, u8>,
) -> Result<ParsedFieldExpression> {
    let mut output = regex.clone();
    let mut capture_index = 1;
    let mut fields = Fields::new();
    for word in regex.split('{') {
        if let Some(end_index) = word.find('}') {
            let k = &word[..end_index];
            let Some(v) = field_aliases.get(k) else {
                bail!("Please define a field for {k} in `{regex}`");
            };
            output = output.replace(&format!("{{{k}}}"), &format!("{{{v}}}"));
            let group_size = match group_sizes.get(v.as_str()) {
                Some(group_size) => *group_size as i32,
                _ => 1,
            };
            let mut group_indexes = vec![];
            for _ in 0..group_size {
                group_indexes.push(capture_index);
                capture_index += 1;
            }
            fields.insert(k.to_string(), group_indexes);
        }
    }
    Ok(ParsedFieldExpression {
        fields,
        regex: output,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn test_parse_expression() {
        let data = [
            (
                "{a} {b}",
                [("a", "string"), ("b", "string")],
                "{string} {string}",
                [("a", vec![1, 2]), ("b", vec![3, 4])],
            ),
            (
                "{b} {a}",
                [("a", "string"), ("b", "string")],
                "{string} {string}",
                [("a", vec![3, 4]), ("b", vec![1, 2])],
            ),
        ];
        for (r, fields, expected, expected_captures) in data {
            let exp = parse_expression(
                r.to_string(),
                fields
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
                [("string".to_string(), 2)].into_iter().collect(),
            )
            .unwrap();
            assert_eq!(expected, exp.regex);

            assert_eq!(
                expected_captures
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v))
                    .collect::<HashMap<_, _>>(),
                exp.fields
            );
        }
    }
}
