use super::ast::*;
#[derive(Debug)]
#[derive(PartialEq)]
pub enum ReScoperError {
    FailedParse(String, Info),
}

// Walks the AST interpreting it.
pub struct ReScoper {
}

impl Default for ReScoper {
    fn default() -> ReScoper {
        ReScoper {}
    }
}

// TODO: Return nodes.
type Res = Result<Node, ReScoperError>;
pub struct Namespace {
    name: String,
    defines: Vec<Sym>,
}

fn globals() -> Namespace {
    Namespace {
        name: "".to_string(),
        defines: vec!{
            Sym::new("true".to_string()),
            Sym::new("false".to_string()),
        }
    }
}

pub struct State {
    stack: Vec<Namespace>,
    requires: Vec<Sym>,
}

impl Visitor<State, Node, Root, ReScoperError> for ReScoper {

    fn visit_root(&mut self, expr: &Root) -> Result<Root, ReScoperError> {
        let mut state = State{stack: vec![globals()], requires: vec![]};
        let res = self.visit(&mut state, &expr.ast)?;
        // Check requires
        if state.requires.len() > 0 {
            println!("{:?} not declared", state.requires);
        }
        Ok(res.to_root())
    }

    fn visit_sym(&mut self, state: &mut State, expr: &Sym) -> Res {
        let mut look_depth = 0;
        let mut found = false;

        'walk_stack: for frame in state.stack.iter() {
            for name in frame.defines.iter() {
                if *name.name == expr.name {
                    // The name is in scope.
                    found = true;
                    break 'walk_stack;
                }
            }
            look_depth += 1;
        }

        let mut depth = Some(look_depth);
        if !found && !state.requires.iter().any(|r| r.name == expr.name) {
            state.requires.push(expr.clone());
            depth = None;
        }
        Ok(Sym {name: expr.name.clone(), depth, info: expr.get_info()}.to_node())
    }

    fn visit_prim(&mut self, _state: &mut State, expr: &Prim) -> Res {
        Ok(expr.clone().to_node())
    }

    fn visit_apply(&mut self, state: &mut State, expr: &Apply) -> Res {
        let mut args = vec![];
        for arg in expr.args.iter() {
            let new_arg = self.visit_let(state, arg)?;
            match new_arg {
                Node::LetNode(letter) => args.push(letter),
                letter => panic!(format!("Rescoper built {} from let", letter)),
            }
        }
        let inner = Box::new(self.visit(state, &*expr.inner)?);
        Ok(Apply{inner, args, info: expr.get_info()}.to_node())
    }

    fn visit_let(&mut self, state: &mut State, expr: &Let) -> Res {
        let frame = state.stack.last_mut().unwrap();
        // if recursive
        frame.defines.push(expr.to_sym());

        let expr_reqs = expr.requires.clone().unwrap_or(vec![]);
        frame.defines.extend(expr_reqs);

        // Find new reqyirements
        let mut requires = vec![];
        std::mem::swap(&mut requires, &mut state.requires);
        let value = Box::new(self.visit(state, &expr.value)?);
        std::mem::swap(&mut requires, &mut state.requires);

        for req in state.requires.iter() {
            if !requires.iter().any(|r| r.name == req.name) {
                requires.push(req.clone());
            }
        }
        let mut i = 0;
        while i != requires.len() {
            if state.requires.iter().any(|r| r.name == requires[i].name) {
                requires.remove(i); // Already in scope.
            } else {
                i += 1;
            }
        }

        // Now that the variable has been defined we can use it.
        for req in requires.iter() {
            if !state.requires.iter().any(|r| r.name == req.name) {
                state.requires.push(req.clone());
            }
        }
        // Now that the variable has been defined we can use it.
        // if !recursive // frame.push(expr.name.clone());

        Ok(Let{
            name: expr.name.clone(),
            is_function: expr.is_function || requires.len() != 0,
            requires: Some(requires),
            value, info: expr.get_info()
        }.to_node())
    }

    fn visit_un_op(&mut self, state: &mut State, expr: &UnOp) -> Res {
        let inner = Box::new(self.visit(state, &expr.inner)?);
        Ok(UnOp{name: expr.name.clone(), inner, info: expr.get_info()}.to_node())
    }

    fn visit_bin_op(&mut self, state: &mut State, expr: &BinOp) -> Res {
        let left = Box::new(self.visit(state, &expr.left)?);
        let right = Box::new(self.visit(state, &expr.right)?);
        Ok(BinOp{name: expr.name.clone(), left, right, info: expr.get_info()}.to_node())
    }

    fn handle_error(&mut self, _state: &mut State, expr: &Err) -> Res {
        Err(ReScoperError::FailedParse(expr.msg.to_string(), expr.get_info()))
    }
}

#[cfg(test)]
mod tests {
}
