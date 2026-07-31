use std::collections::HashSet;

pub struct Interner {
    items: HashSet<&'static str>,
}

impl Default for Interner {
    fn default() -> Self {
        Self::new()
    }
}

impl Interner {
    pub fn new() -> Self {
        Self {
            items: HashSet::new(),
        }
    }

    pub fn intern(&mut self, s: impl AsRef<str>) -> &'static str {
        if let Some(s) = self.items.get(s.as_ref()) {
            return s;
        };

        let string = s.as_ref().to_string();
        let boxed = Box::new(string);
        let s = Box::leak(boxed);
        self.items.insert(s);

        s
    }
}
