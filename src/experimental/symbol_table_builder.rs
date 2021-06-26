use crate::ast::*;
use crate::database::Compiler;
use crate::errors::TError;
use crate::primitives::Val;
use crate::symbol_table_builder::State;
use crate::symbol_table::*;
use super::tree::{to_hash_root, HashTree};
use specs::prelude::*;

// Walks the AST interpreting it.
pub struct SymbolTableBuilder<'a> {
    world: &'a mut World,
}

impl<'a> SymbolTableBuilder<'a> {
    pub fn new(world: &'a mut World) -> Self {
        Self { world }
    }
}

// TODO: Return nodes.
type Res = Result<Node, TError>;

#[derive(Debug, Clone)]
pub struct State {
    pub table: Table,
    pub path: Vec<Symbol>,
}

impl Visitor<State, Node, Root, Path> for SymbolTableBuilder {
    fn visit_root(&mut self, db: &dyn Compiler, module: &Path) -> Result<Root, TError> {
        let expr = &db.parse_file(module.clone())?;
        if db.debug_level() > 0 {
            eprintln!(
                "building symbol table for file... {}",
                path_to_string(module)
            );
        }

        let mut table = Table::default();
        let mut main_at = module.clone();
        main_at.push(Symbol::new("main"));

        let main_symb = table.get_mut(&main_at);
        main_symb.value.uses.insert(module.clone());

        // Add in the globals here!
        // TODO: Inject needs for bootstrapping here (e.g. import function).
        let globals: Vec<Path> = db
            .get_extern_names()?
            .iter()
            .map(|x| vec![Symbol::new(x)])
            .collect();
        for global in globals {
            table.get_mut(&global);
        }

        let mut state = State {
            table,
            path: module.clone(),
        };

        if db.debug_level() > 0 {
            eprintln!("table: {:?}", state.table);
        }

        Ok(Root {
            ast: self.visit(db, &mut state, expr)?,
            table: state.table,
        })
    }

    fn visit_sym(&mut self, _db: &dyn Compiler, _state: &mut State, expr: &Sym) -> Res {
        Ok(expr.clone().into_node())
    }

    fn visit_val(&mut self, _db: &dyn Compiler, _state: &mut State, expr: &Val) -> Res {
        Ok(expr.clone().into_node())
    }

    fn visit_apply(&mut self, db: &dyn Compiler, state: &mut State, expr: &Apply) -> Res {
        state.path.push(Symbol::Anon());
        let args = expr
            .args
            .iter()
            .map(|arg| self.visit_let(db, state, arg)?.as_let())
            .collect::<Result<_, _>>()?;
        let inner = Box::new(self.visit(db, state, &*expr.inner)?);
        state.path.pop();

        Ok(Apply {
            inner,
            args,
            info: expr.get_info(),
        }
        .into_node())
    }

    fn visit_abs(&mut self, db: &dyn Compiler, state: &mut State, expr: &Abs) -> Res {
        if db.debug_level() > 1 {
            eprintln!("visiting {} {}", path_to_string(&state.path), &expr.name);
        }

        // Visit definition.
        let mut info = expr.get_info();
        state.path.push(Symbol::new(&expr.name));
        info.defined_at = Some(state.path.clone());
        state.table.get_mut(&state.path);

        let value = Box::new(self.visit(db, state, &expr.value)?);
        state.path.pop();

        Ok(Abs {
            name: expr.name.clone(),
            value,
            info,
        }
        .into_node())
    }

    fn visit_let(&mut self, db: &dyn Compiler, state: &mut State, expr: &Let) -> Res {
        if db.debug_level() > 1 {
            eprintln!("visiting {} {}", path_to_string(&state.path), &expr.name);
        }

        // Visit definition.
        let mut info = expr.get_info();
        state.path.push(Symbol::new(&expr.name));
        info.defined_at = Some(state.path.clone());
        state.table.get_mut(&state.path);

        // Consider the function arguments defined in this scope.
        let args = if let Some(args) = &expr.args {
            Some(
                args.iter()
                    .map(|arg| self.visit_let(db, state, arg)?.as_let())
                    .collect::<Result<_, _>>()?,
            )
        } else {
            None
        };
        let value = Box::new(self.visit(db, state, &expr.value)?);
        state.path.pop();

        Ok(Let {
            name: expr.name.clone(),
            value,
            args,
            info,
        }
        .into_node())
    }

    fn visit_un_op(&mut self, db: &dyn Compiler, state: &mut State, expr: &UnOp) -> Res {
        let inner = Box::new(self.visit(db, state, &expr.inner)?);
        Ok(UnOp {
            name: expr.name.clone(),
            inner,
            info: expr.get_info(),
        }
        .into_node())
    }

    fn visit_bin_op(&mut self, db: &dyn Compiler, state: &mut State, expr: &BinOp) -> Res {
        let left = Box::new(self.visit(db, state, &expr.left)?);
        let right = Box::new(self.visit(db, state, &expr.right)?);
        Ok(BinOp {
            name: expr.name.clone(),
            left,
            right,
            info: expr.get_info(),
        }
        .into_node())
    }

    fn handle_error(&mut self, _db: &dyn Compiler, _state: &mut State, expr: &TError) -> Res {
        Err(expr.clone())
    }
}

#[cfg(test)]
mod tests {}
