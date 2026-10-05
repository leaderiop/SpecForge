//! The greet extension, authored entirely with the SpecForge extension SDK:
//! its declarations (`contributions`) and the component glue the SDK
//! generates for them.

mod contributions;

specforge_extension_sdk::component_guest!(build = contributions::build);
