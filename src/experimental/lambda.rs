type Delta = i32;
type Ind = Delta; // Should be unsigned

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Debug)]
pub enum Term {
    Var { ind: Ind },
    App { inner: Box<Term>, arg: Box<Term> },
    Abs { inner: Box<Term> },
}
use Term::*;

fn var(ind: Ind) -> Term {
    Var { ind }
}

fn app(inner: Term, arg: Term) -> Term {
    App {
        inner: Box::new(inner),
        arg: Box::new(arg),
    }
}

fn abs(inner: Term) -> Term {
    Abs {
        inner: Box::new(inner),
    }
}

impl Term {
    fn shift(&self, delta: Delta) -> Term {
        self.shift_with_cutoff(delta, 0)
    }

    fn shift_with_cutoff(&self, delta: Delta, cutoff: Ind) -> Term {
        match self {
            Var { ind } => {
                if *ind < cutoff {
                    self.clone()
                } else {
                    var(ind
                        .checked_add(delta)
                        .expect("Should not run out of indexes"))
                }
            }
            App { inner, arg } => app(
                inner.shift_with_cutoff(delta, cutoff),
                arg.shift_with_cutoff(delta, cutoff),
            ),
            Abs { inner } => abs(inner.shift_with_cutoff(delta, cutoff + 1)),
        }
    }

    fn substitute(&self, x: Ind, with: &Term) -> Term {
        match self {
            Var { ind } => if *ind == x { with } else { self }.clone(),
            App { inner, arg } => app(inner.substitute(x, with), arg.substitute(x, with)),
            Abs { inner } => abs(inner.substitute(x + 1, &with.shift(1))),
        }
    }

    fn beta_reduce(&self) -> Term {
        if let App { inner, arg } = self {
            let inner: &Term = &*inner; // TODO: Work out why I can't write this in one line.
            if let Abs { inner } = inner {
                return inner.substitute(0, &arg.shift(1)).shift(-1);
            }
        }
        self.clone()
    }

    fn eval(&self) -> Term {
        match self {
            Var { .. } => self.clone(),
            App { inner, arg } => {
                let simplified = app(inner.eval(), arg.eval());
                let reduced = simplified.beta_reduce();
                if reduced != simplified {
                    return reduced.eval();
                }
                simplified
            }
            Abs { inner } => abs(inner.eval()),
        }
    }
}

mod util {
    use super::*;

    pub fn church_bool(b: bool) -> Term {
        abs(abs(var(if b { 1 } else { 0 })))
    }

    pub fn church_not() -> Term {
        abs(app(app(var(0), church_bool(false)), church_bool(true)))
    }

    pub fn church_nat(n: i32) -> Term {
        let f = || var(1);
        let x = || var(0);

        let mut curr = x();
        for _ in 0..n {
            curr = app(f(), curr);
        }
        abs(abs(curr))
    }

    pub fn church_plus() -> Term {
        //          m.  n.  f.  x.          ((m f)((n f) x))
        abs(abs(abs(abs(app(
            app(var(3), var(1)),
            app(app(var(2), var(1)), var(0)),
        )))))
    }
}

#[cfg(test)]
mod test {
    use super::util::*;
    use super::*;

    #[test]
    fn basic_shift() {
        let before = abs(abs(app(var(1), app(var(0), var(2)))));
        let after = abs(abs(app(var(1), app(var(0), var(4)))));
        assert_eq!(before.shift(2), after);
    }

    #[test]
    fn simple_shift() {
        let before = abs(app(
            app(var(0), var(1)),
            abs(app(app(var(0), var(1)), var(2))),
        ));
        let after = abs(app(
            app(var(0), var(3)),
            abs(app(app(var(0), var(1)), var(4))),
        ));
        assert_eq!(before.shift(2), after);
    }

    #[test]
    fn substitute_noop() {
        assert_eq!(var(1).substitute(0, &var(2)), var(1));
    }

    #[test]
    fn substitute_x() {
        assert_eq!(var(0).substitute(0, &var(2)), var(2));
    }

    #[test]
    fn substitute_id() {
        assert_eq!(abs(var(1)).substitute(0, &var(2)), abs(var(3)));
    }

    #[test]
    fn substitute_id_of_id() {
        assert_eq!(abs(var(1)).substitute(0, &abs(var(1))), abs(abs(var(2))));
    }

    #[test]
    fn substitute_x_app_y() {
        assert_eq!(
            app(var(0), var(1)).substitute(0, &abs(var(1))),
            app(abs(var(1)), var(1))
        );
    }

    #[test]
    fn substitute_x_app_x() {
        assert_eq!(
            app(var(0), var(0)).substitute(0, &abs(var(1))),
            app(abs(var(1)), abs(var(1)))
        );
    }

    #[test]
    fn basic_beta_reduction() {
        assert_eq!(
            app(abs(app(app(var(1), var(0)), var(2))), abs(var(0))).beta_reduce(),
            app(app(var(0), abs(var(0))), var(1))
        )
    }

    #[test]
    fn eval_one_step() {
        let id = abs(var(0));
        let before = app(
            app(
                app(abs(abs(abs(app(app(var(1), var(2)), var(0))))), id.clone()),
                var(5),
            ),
            var(6),
        );
        let after = app(app(var(5), id), var(6));
        assert_eq!(before.eval(), after);
    }

    #[test]
    fn eval_two_step() {
        let id = abs(var(0));
        let before = app(
            app(
                app(abs(abs(abs(app(app(var(1), var(2)), var(0))))), var(5)),
                id,
            ),
            var(6),
        );
        let after = app(var(5), var(6));
        assert_eq!(before.eval(), after);
    }

    #[test]
    fn eval_church_true() {
        assert_eq!(
            app(app(church_bool(true), church_nat(5)), church_nat(6)).eval(),
            church_nat(5)
        );
    }

    #[test]
    fn eval_church_false() {
        assert_eq!(
            app(app(church_bool(false), church_nat(5)), church_nat(6)).eval(),
            church_nat(6)
        );
    }

    #[test]
    fn eval_church_not_true() {
        assert_eq!(
            app(church_not(), church_bool(true)).eval(),
            church_bool(false)
        );
    }

    #[test]
    fn eval_church_not_false() {
        assert_eq!(
            app(church_not(), church_bool(false)).eval(),
            church_bool(true)
        );
    }

    #[test]
    fn eval_church_3_plus_4_eq_7() {
        let three = church_nat(3);
        let four = church_nat(4);
        let seven = church_nat(7);
        assert_eq!(app(app(church_plus(), three), four).eval(), seven);
    }
}
