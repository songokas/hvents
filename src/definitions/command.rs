use crate::{
    events::{EventType, ReferencingEvent, command::CommandEvent},
    gherkin::{
        expression_parser::FieldExpression,
        field_captures::FieldCaptures,
        macros::{impl_create_event, impl_from_config},
    },
};

pub struct CommandDefinition {
    expressions: Vec<FieldExpression>,
}

impl_from_config!(CommandDefinition);
impl_create_event!(CommandDefinition);

fn create_event(c: FieldCaptures) -> Result<ReferencingEvent, anyhow::Error> {
    Ok(ReferencingEvent {
        event_type: EventType::Execute(CommandEvent {
            command: c.get_required("command")?,
            args: c
                .get("args")
                .map(|s| s.split_whitespace().map(ToOwned::to_owned).collect())
                .unwrap_or_default(),
            replace_args: Default::default(),
            vars: Default::default(),
            data_type: Default::default(),
        }),
        ..Default::default()
    })
}
