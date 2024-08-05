mod cmd_extract_config;
mod cmd_inspect_config;

pub mod prelude {
    pub use super::cmd_extract_config::extract_config;
    pub use super::cmd_inspect_config::inspect_config;
}
