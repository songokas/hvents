use std::str::FromStr;

use anyhow::Result;
use anyhow::anyhow;
use regex::Captures;

use crate::gherkin::expression_parser::Fields;

#[derive(Debug)]
pub struct FieldCaptures<'a> {
    pub fields: Fields,
    pub captures: Captures<'a>,
}

impl<'a> FieldCaptures<'a> {
    pub fn get(&self, key: &str) -> Option<String> {
        let indexes = self.fields.get(key)?;
        indexes
            .iter()
            .find_map(|index| self.captures.get(*index).map(|m| m.as_str().to_string()))
    }

    pub fn get_as<T>(&self, key: &str) -> Result<Option<T>, <T as FromStr>::Err>
    where
        T: FromStr,
    {
        let Some(indexes) = self.fields.get(key) else {
            return Ok(None);
        };
        indexes
            .iter()
            .find_map(|index| {
                self.captures
                    .get(*index)
                    .map(|m| m.as_str().parse().map_err(Into::into))
            })
            .transpose()
    }

    pub(crate) fn get_required(&self, key: &str) -> Result<String> {
        self.get(key).ok_or_else(|| anyhow!("{key} is required"))
    }

    pub fn get_required_as<T>(&self, key: &str) -> Result<T>
    where
        T: FromStr,
    {
        self.get_as(key)
            .map_err(|_| anyhow!("Error while retrieving type for {key}"))?
            .ok_or_else(|| anyhow!("{key} is required"))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use regex::Regex;

    use super::*;

    #[test]
    fn test_captures() {
        let regexp = Regex::new("^(letter b) (letter a)$").unwrap();
        let captures = regexp.captures("letter b letter a").unwrap();
        let captures = FieldCaptures {
            fields: HashMap::from([("a".to_string(), vec![2]), ("b".to_string(), vec![1])]),
            captures,
        };
        assert_eq!("letter a", captures.get("a").unwrap());
        assert_eq!("letter a", captures.get_as::<String>("a").unwrap().unwrap());
        assert_eq!("letter a", captures.get_required("a").unwrap());
        assert_eq!("letter a", captures.get_required_as::<String>("a").unwrap());
        assert_eq!("letter b", captures.get("b").unwrap());
        assert!(captures.get("c").is_none());
        assert!(captures.get_required("c").is_err());
    }
}
