use proc_macro::TokenStream;

/// Marks a command variant as the implicit default when no subcommand is specified.
///
/// This is a marker attribute - the actual logic for handling implicit commands
/// is implemented in the Args::parse handling in the CLI dispatcher.
#[proc_macro_attribute]
pub fn implicit_command(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Pass through unchanged - this is just a marker for documentation
    item
}
