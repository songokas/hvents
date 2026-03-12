use handlebars::{
    Context, Handlebars, Helper, HelperResult, JsonRender, Output, RenderContext, RenderErrorReason,
};
use human_date_parser::{ParseResult, from_human_time};
use indexmap::IndexMap;
use serde::Serialize;
use std::fmt::Write;

use crate::{
    config::now,
    events::data::{Data, Metadata},
};

pub fn load_handlebars() -> Handlebars<'static> {
    let mut handlebars = Handlebars::new();
    handlebars.register_helper("date-time-format", Box::new(date_time_helper));
    handlebars.register_helper("lookup-word", Box::new(lookup_word));
    handlebars.register_helper("concat", Box::new(concat));
    handlebars
}

#[cfg(feature = "metrics")]
pub fn add_metrics(
    handlebars: &mut Handlebars<'static>,
    recorder: std::sync::Arc<otlp_metrics_exporter::otlp_recorder::OtlpRecorder>,
) {
    use core::time::Duration;

    handlebars.register_helper(
        "otlp-metrics",
        Box::new(
            move |h: &Helper,
                  _: &Handlebars,
                  _: &Context,
                  _: &mut RenderContext,
                  out: &mut dyn Output|
                  -> HelperResult {
                let period = h
                    .param(0)
                    .map(|p| {
                        p.value()
                            .as_u64()
                            .ok_or(RenderErrorReason::InvalidParamType("number"))
                    })
                    .transpose()?
                    .map(Duration::from_secs);
                out.write(&recorder.to_json(period))?;
                Ok(())
            },
        ),
    );
}

#[derive(Serialize)]
pub struct TemplateData<'a> {
    pub data: &'a Data,
    pub metadata: &'a Metadata,
    pub state: &'a IndexMap<String, String>,
}

fn date_time_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let time = h
        .param(0)
        .ok_or(RenderErrorReason::ParamNotFoundForIndex(
            "date-time-format",
            0,
        ))?
        .value()
        .render();
    let format = h
        .param(1)
        .ok_or(RenderErrorReason::ParamNotFoundForIndex(
            "date-time-format",
            1,
        ))?
        .value()
        .render();

    let time_format =
        match from_human_time(&time, now()).map_err(|e| RenderErrorReason::Other(e.to_string()))? {
            ParseResult::Date(d) => d.format(&format),
            ParseResult::Time(d) => d.format(&format),
            ParseResult::DateTime(d) => d.format(&format),
        };
    let mut time = String::new();
    write!(time, "{time_format}").map_err(|e| RenderErrorReason::Other(e.to_string()))?;
    out.write(&time)?;
    Ok(())
}

fn lookup_word(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let data = h
        .param(0)
        .ok_or(RenderErrorReason::ParamNotFoundForIndex("lookup-word", 0))?
        .value()
        .render();

    let index =
        h.param(1)
            .ok_or(RenderErrorReason::ParamNotFoundForIndex("lookup-word", 1))?
            .value()
            .as_u64()
            .ok_or(RenderErrorReason::ParamNotFoundForIndex("lookup-word", 1))? as usize;

    let delimiter = h
        .param(2)
        .ok_or(RenderErrorReason::ParamNotFoundForIndex("lookup-word", 2))?
        .value()
        .render();

    for (windex, word) in data.split(&delimiter).enumerate() {
        if windex == index {
            out.write(word)?;
        }
    }
    Ok(())
}

fn concat(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let data = h
        .param(0)
        .ok_or(RenderErrorReason::ParamNotFoundForIndex("concat", 0))?
        .value()
        .render();

    let str = h
        .param(1)
        .ok_or(RenderErrorReason::ParamNotFoundForIndex("concat", 1))?
        .value()
        .render();
    let mut word = String::new();
    write!(word, "{data}{str}").map_err(|e| RenderErrorReason::Other(e.to_string()))?;
    out.write(&word)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use crate::config::now;

    use super::*;

    #[test]
    fn test_handle_bars() {
        let now = now();
        let handlebars = load_handlebars();
        let template = "Air temperature {{#each forecastTimestamps}}{{#if (eq forecastTimeUtc (date-time-format ../expectedKey \"%Y-%m-%d %H:%M:%S\"))}}{{airTemperature}}{{/if}}{{/each}}";
        let mut data: Value = serde_json::from_str(
            &format!(r#"{{"forecastTimestamps":[{{"forecastTimeUtc":"{} 00:00:00", "airTemperature":"22.1"}}]}}"#, now.date()),
        )
        .unwrap();
        data["expectedKey"] = Value::String("today 00:00:00".to_string());
        let result = handlebars.render_template(template, &data).unwrap();
        assert_eq!(result, "Air temperature 22.1");
    }

    #[test]
    fn test_date_time_format_helper() {
        let handlebars = load_handlebars();
        let data = json!({
            "expectedKey": "2022-02-02"
        });
        let template = r#"{{date-time-format "2022-02-02" "%Y-%m-%d"}}"#;
        let result = handlebars.render_template(template, &data).unwrap();
        assert_eq!(result, "2022-02-02");

        let template = r#"{{date-time-format expectedKey "%Y-%m-%d"}}"#;
        let result = handlebars.render_template(template, &data).unwrap();
        assert_eq!(result, "2022-02-02");

        // invalid format options provided
        let template = r#"{{date-time-format expectedKey "%Y-%m-%d %S"}}"#;
        let result = handlebars.render_template(template, &data);
        assert!(result.is_err());

        let template = r#"{{date-time-format "unknown" "%Y-%m-%d"}}"#;
        let result = handlebars.render_template(template, &data);
        assert!(result.is_err());

        let template = r#"{{date-time-format expectedKey ""}}"#;
        let result = handlebars.render_template(template, &data);
        assert_eq!(result.unwrap(), "");

        let template = r#"{{date-time-format ""}}"#;
        let result = handlebars.render_template(template, &data);
        assert!(result.is_err());

        let template = r#"{{date-time-format}}"#;
        let result = handlebars.render_template(template, &data);
        assert!(result.is_err());
    }

    #[test]
    fn test_lookup_word() {
        let handlebars = load_handlebars();
        let data = "this many words";

        let template = r#"{{lookup-word this 1 " "}}"#;
        let result = handlebars.render_template(template, &data).unwrap();
        assert_eq!(result, "many");

        let template = r#"{{lookup-word this 2 " "}}"#;
        let result = handlebars.render_template(template, &data).unwrap();
        assert_eq!(result, "words");

        let template = r#"{{lookup-word this 3 " "}}"#;
        let result = handlebars.render_template(template, &data).unwrap();
        assert_eq!(result, "");

        let template = r#"{{lookup-word this 2 "a"}}"#;
        let result = handlebars.render_template(template, &data).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_lookup() {
        let handlebars = load_handlebars();
        let data = json!({
                "data": "room",
                "state": json!({
                    "room": "20"
                })
        });

        let template = r#"{{lookup state data}}"#;
        let result = handlebars.render_template(template, &data).unwrap();
        assert_eq!(result, "20");

        let template = r#"
        {{#if (lt (lookup state data) 21)}}
            ok
        {{/if}}"#;
        let result = handlebars.render_template(template, &data).unwrap();
        assert_eq!(result, "\n            ok\n");

        let data = json!({
                "data": "room",
                "state": json!({
                    "room": "something else"
                })
        });

        let template = r#"{{#if (eq "hello" 21)}}ok{{/if}}"#;
        let result = handlebars.render_template(template, &data).unwrap();
        assert_eq!(result, "");

        let template = r#"{{#if (not (lookup state 'none'))}}yes{{/if}}"#;
        let result = handlebars.render_template(template, &data).unwrap();
        assert_eq!(result, "yes");
    }

    #[test]
    fn test_concat() {
        let handlebars = load_handlebars();
        let data = "some data";

        let template = r#"{{concat 'hello' 'mark'}}"#;
        let result = handlebars.render_template(template, &data).unwrap();
        assert_eq!(result, "hellomark");

        let template = r#"{{concat this 'mark'}}"#;
        let result = handlebars.render_template(template, &data).unwrap();
        assert_eq!(result, "some datamark");

        let template = r#"{{concat nothing 'mark'}}"#;
        let result = handlebars.render_template(template, &data).unwrap();
        assert_eq!(result, "mark");
    }
}
