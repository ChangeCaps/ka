use crate::{
    ir::{Alias, Ty, Visible},
    lower::Lowerer,
};

impl Lowerer<'_> {
    pub(super) fn add_prelude(&mut self) {
        self.add_prelude_alias("bool", Ty::bool());
    }

    fn add_prelude_alias(&mut self, name: &'static str, ty: Ty) {
        let alias = self.aliases.add(Alias {
            name,
            params: Vec::new(),
            ty,
        });

        self.scopes[self.prelude]
            .aliases
            .push(Visible::global(alias));
    }
}
