#![deny(clippy::all)]

#[macro_use]
mod map_macros;

use std::fs::File;
use std::io::prelude::*;
use std::sync::Arc;

pub mod ast;
pub mod cli_options;
pub mod database;
pub mod errors;
pub mod primitives;
pub mod tribool;

pub mod interpreter;
pub mod parser;
pub mod type_checker;

mod externs;
mod location;
mod symbol_table;
mod tokens;
mod tree;

mod definition_finder;
mod pretty_print;
mod symbol_table_builder;
mod to_cpp;

#[allow(dead_code)]
mod experimental; // This is where the fun, but currently unused stuff goes

mod components;
use ast::Visitor;
use interpreter::Interpreter;
use pretty_print::PrettyPrint;

use database::{Compiler, DB};
use errors::TError;
use interpreter::ImplFn;

pub fn work<'a>(
    db: &mut DB,
    filename: &str,
    print_impl: Option<ImplFn<'a>>,
) -> Result<String, TError> {
    let mut contents = String::new();
    let mut file = File::open(filename.to_owned())?;
    file.read_to_string(&mut contents)?;

    work_on_string(db, contents, filename, print_impl)
}

pub fn work_on_string<'a>(
    db: &mut DB,
    contents: String,
    filename: &str,
    print_impl: Option<ImplFn<'a>>,
) -> Result<String, TError> {
    let module_name = db.module_name(filename.to_owned());
    db.set_file(filename.to_owned(), Ok(Arc::new(contents)));

    use cli_options::Command;
    if db.options().cmd == Command::Build {
        db.build_with_gpp(module_name)
    } else {
        let root = db.look_up_definitions(module_name)?;
        let mut interp = Interpreter::default();
        if let Some(print_impl) = print_impl {
            interp.impls.insert("print".to_string(), print_impl);
        }
        let res = interp.visit_root(db, &root)?;
        use ast::ToNode;
        PrettyPrint::process(&res.into_node(), db).or_else(|_| panic!("Pretty print failed"))
    }
}
