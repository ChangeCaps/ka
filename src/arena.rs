use std::{
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::{Index, IndexMut},
};

#[derive(Clone)]
pub struct Arena<T> {
    items: Vec<Option<T>>,
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Arena<T> {
    pub const fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn add(&mut self, item: T) -> Id<T> {
        let index = self.items.len();
        self.items.push(Some(item));

        Id {
            index,
            marker: PhantomData,
        }
    }

    pub fn reserve(&mut self) -> Id<T> {
        let index = self.items.len();
        self.items.push(None);

        Id {
            index,
            marker: PhantomData,
        }
    }

    pub fn insert(&mut self, id: Id<T>, item: T) {
        if let Some(entry) = self.items.get_mut(id.index) {
            *entry = Some(item);
        }
    }

    pub fn get(&self, id: Id<T>) -> Option<&T> {
        self.items.get(id.index)?.as_ref()
    }

    pub fn get_mut(&mut self, id: Id<T>) -> Option<&mut T> {
        self.items.get_mut(id.index)?.as_mut()
    }

    pub fn keys(&self) -> impl DoubleEndedIterator<Item = Id<T>> {
        self.items.iter().enumerate().filter_map(|(idx, item)| {
            item.as_ref().map(|_| Id {
                index: idx,
                marker: PhantomData,
            })
        })
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (Id<T>, &T)> {
        self.items.iter().enumerate().filter_map(|(idx, item)| {
            let id = Id {
                index: idx,
                marker: PhantomData,
            };

            Some((id, item.as_ref()?))
        })
    }

    pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = (Id<T>, &mut T)> {
        self.items.iter_mut().enumerate().filter_map(|(idx, item)| {
            let id = Id {
                index: idx,
                marker: PhantomData,
            };

            Some((id, item.as_mut()?))
        })
    }
}

impl<T> Index<Id<T>> for Arena<T> {
    type Output = T;

    #[track_caller]
    fn index(&self, id: Id<T>) -> &Self::Output {
        self.get(id).unwrap()
    }
}

impl<T> IndexMut<Id<T>> for Arena<T> {
    fn index_mut(&mut self, id: Id<T>) -> &mut Self::Output {
        self.get_mut(id).unwrap()
    }
}

impl<T> fmt::Debug for Arena<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

pub struct Id<T> {
    index: usize,
    marker: PhantomData<T>,
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Id<T> {}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Id").field(&self.index).finish()
    }
}
