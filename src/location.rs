use std::fmt;

#[derive(PartialEq, Eq, Clone, Ord, PartialOrd, Hash)]
pub struct Loc {
    pub filename: Option<String>,
    pub line: i32,
    pub col: i32,
}

impl std::fmt::Debug for Loc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.filename {
            Some(file) => write!(f, "{} ", file),
            None => write!(f, ""),
        }?;
        write!(f, "at line {}, column {}", self.line, self.col)
    }
}

impl Loc {
    pub fn next(&mut self, chars: &mut std::iter::Peekable<std::str::Chars>) {
        // TODO: Consider just keeping the offsets and then recovering line
        // info later.
        let ch = chars.peek();
        if ch == None {
            return;
        }

        let nl = ch == Some(&'\n');
        let lf = ch == Some(&'\r');
        chars.next();
        self.col += 1;
        if nl || lf {
            if lf && chars.peek() == Some(&'\n') {
                chars.next();
            }
            self.line += 1;
            self.col = 1;
        }
    }
}

impl Default for Loc {
    fn default() -> Self {
        Loc {
            filename: None,
            line: 1,
            col: 1,
        }
    }
}
