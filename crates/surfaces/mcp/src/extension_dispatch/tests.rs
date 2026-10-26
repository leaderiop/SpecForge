use super::*;
use specforge_test::prelude::*;

#[specforge_test(
    behavior = "dispatch_surface_command",
    verify = "over MCP an argument the command's declaration refuses is invalid_input on that argument, with the message the CLI writes (a missing one in the sentence every MCP tool uses), and the export is not called"
)]
fn refused_args_names_the_arg_each_refusal_is_about() {
    let name = || "arg".to_string();
    for (error, argument, suggestion) in [
        (
            ArgError::Missing {
                names: vec!["a".into(), "b".into()],
            },
            "a",
            None,
        ),
        (
            ArgError::NotOneOf {
                name: name(),
                values: vec!["json".into()],
                got: json!("jsno"),
            },
            "arg",
            Some("did you mean 'json'?"),
        ),
        (
            ArgError::NotOneOf {
                name: name(),
                values: vec!["json".into()],
                got: json!("zzzzzz"),
            },
            "arg",
            None,
        ),
        (
            ArgError::NotInteger {
                name: name(),
                minimum: None,
                got: json!("x"),
            },
            "arg",
            None,
        ),
        (
            ArgError::NotString {
                name: name(),
                got: json!(5),
            },
            "arg",
            None,
        ),
        (
            ArgError::NotBool {
                name: name(),
                got: json!("x"),
            },
            "arg",
            None,
        ),
        (
            ArgError::Unknown {
                name: name(),
                suggestion: Some("limit".into()),
            },
            "arg",
            Some("did you mean 'limit'?"),
        ),
        (
            ArgError::Unknown {
                name: name(),
                suggestion: None,
            },
            "arg",
            None,
        ),
    ] {
        let refused = refused_args(&error);
        assert_eq!(refused.code, ErrorCode::InvalidInput, "{error:?}");
        assert_eq!(refused.argument.as_deref(), Some(argument), "{error:?}");
        let message = match &error {
            ArgError::Missing { .. } => format!("Missing required parameter: {argument}"),
            _ => error.message(),
        };
        assert_eq!(refused.message, message, "{error:?}");
        let found = refused.data.as_ref().and_then(|d| d["suggestion"].as_str());
        assert_eq!(found, suggestion, "{error:?}");
    }
}
