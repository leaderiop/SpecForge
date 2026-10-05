//! The greet extension, authored entirely with the SpecForge extension SDK:
//! its declarations (`contributions`) and the component glue the SDK
//! generates for them.

mod contributions;

fn dispatch(_export: &str, _input: &[u8]) -> Option<Result<Vec<u8>, String>> {
    None
}

specforge_extension_sdk::component_guest!(build = contributions::build, handler = dispatch);
