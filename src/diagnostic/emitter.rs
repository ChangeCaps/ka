use crate::diagnostic::Diagnostic;

pub trait Emitter {
    fn emit(&mut self, diagnostic: Diagnostic);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DebugEmitter;

impl Emitter for DebugEmitter {
    fn emit(&mut self, diagnostic: Diagnostic) {
        eprintln!("{:?}", diagnostic);
    }
}
