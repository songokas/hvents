use handlebars::{
    Context, Handlebars, Helper, HelperResult, JsonRender, Output, RenderContext, RenderErrorReason,
};
use human_date_parser::{from_human_time, ParseResult};
use std::fmt::Write;

pub fn load_handlebars() -> Handlebars<'static> {
    let mut handlebars = Handlebars::new();
    handlebars.register_helper("date-time-format", Box::new(date_time_helper));
    handlebars
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
        match from_human_time(&time).map_err(|e| RenderErrorReason::Other(e.to_string()))? {
            ParseResult::Date(d) => d.format(&format),
            ParseResult::Time(d) => d.format(&format),
            ParseResult::DateTime(d) => d.format(&format),
        };
    let mut time = String::new();
    write!(time, "{}", time_format).map_err(|e| RenderErrorReason::Other(e.to_string()))?;
    out.write(&time)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::config::now;

    use super::*;

    #[test]
    fn test_handle_bars() {
        let now = now();
        let handlebars = load_handlebars();
        let template = "Air temperature {{#each forecastTimestamps}}{{#if (eq forecastTimeUtc (date-time-format ../expectedKey \"%Y-%m-%d %H:%M:%S\"))}}{{airTemperature}}{{/if}}{{/each}}";
        let mut data: Value = serde_json::from_str(
            &format!(r#"{{"forecastTimestamps":[{{"forecastTimeUtc":"{} 00:00:00", "airTemperature":"22.1"}}]}}"#, now.naive_local().date()),
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
}
