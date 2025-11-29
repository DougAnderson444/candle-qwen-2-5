mod ast;
pub use ast::DslCommand;

mod interpreter;
pub use interpreter::apply_commands;

mod parser;
pub use parser::parse_dsl;
