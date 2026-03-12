macro_rules! impl_from_config {
    ($n:ty) => {
        impl $n {
            pub fn from_config(
                config: $crate::config::DefinitionEvent,
            ) -> Result<Self, anyhow::Error> {
                Ok(Self {
                    expressions: config
                        .expressions
                        .iter()
                        .map(|s| {
                            let $crate::gherkin::expression_parser::ParsedFieldExpression {
                                regex,
                                fields,
                            } = $crate::gherkin::expression_parser::parse_expression(
                                s.clone(),
                                config.fields.clone(),
                                config.capture_group_sizes.clone(),
                            )?;
                            Ok($crate::gherkin::expression_parser::FieldExpression {
                                regex: cucumber_expressions::Expression::regex_with_parameters(
                                    &regex,
                                    &config.arguments,
                                )
                                .map_err(|e| anyhow::anyhow!("Unable to parse {regex} {e}"))?,
                                fields,
                            })
                        })
                        .collect::<Result<
                            Vec<$crate::gherkin::expression_parser::FieldExpression>,
                            anyhow::Error,
                        >>()?,
                })
            }
        }
    };
}

macro_rules! impl_create_event {
    ($n:ty) => {
        impl $crate::gherkin::event_factory::EventFactory for $n {
            fn create_event(
                &self,
                value: String,
            ) -> Result<Option<$crate::events::ReferencingEvent>, anyhow::Error> {
                for f in &self.expressions {
                    for captures in f.regex.captures_iter(&value) {
                        return Ok(Some(create_event(
                            $crate::gherkin::field_captures::FieldCaptures {
                                captures,
                                fields: f.fields.clone(),
                            },
                        )?));
                    }
                }
                Ok(None)
            }
        }
    };
}

pub(crate) use impl_create_event;
pub(crate) use impl_from_config;
