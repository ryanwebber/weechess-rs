use std::{fmt::Debug, marker::PhantomData, ops::Deref};

pub trait ArrayKey: Into<Index> + Copy {
    const COUNT: usize;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Index(pub usize);

impl From<usize> for Index {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

pub struct ArrayMap<I, T>
where
    I: ArrayKey,
    [(); I::COUNT]:,
{
    array: [T; I::COUNT],
    _marker: PhantomData<I>,
}

impl<I, T> ArrayMap<I, T>
where
    I: ArrayKey,
    [(); I::COUNT]:,
{
    pub fn new(array: [T; I::COUNT]) -> Self
    where
        T: Default,
    {
        Self {
            array,
            _marker: PhantomData,
        }
    }
}

impl<I, T> ArrayMap<I, T>
where
    I: ArrayKey,
    [(); I::COUNT]:,
    T: Copy,
{
    pub const fn filled(value: T) -> Self {
        Self {
            array: [value; I::COUNT],
            _marker: PhantomData,
        }
    }
}

impl<I, T> Default for ArrayMap<I, T>
where
    I: ArrayKey,
    [(); I::COUNT]:,
    T: Default + Copy,
{
    fn default() -> Self {
        Self {
            array: [T::default(); I::COUNT],
            _marker: PhantomData,
        }
    }
}

impl<I, T> Deref for ArrayMap<I, T>
where
    I: ArrayKey,
    [(); I::COUNT]:,
{
    type Target = [T; I::COUNT];

    fn deref(&self) -> &Self::Target {
        &self.array
    }
}

impl<I, T> std::ops::Index<I> for ArrayMap<I, T>
where
    I: ArrayKey,
    [(); I::COUNT]:,
{
    type Output = T;

    fn index(&self, index: I) -> &Self::Output {
        &self.array[index.into().0]
    }
}

impl<I, T> std::ops::IndexMut<I> for ArrayMap<I, T>
where
    I: ArrayKey,
    [(); I::COUNT]:,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.array[index.into().0]
    }
}

impl<I, T> IntoIterator for ArrayMap<I, T>
where
    I: ArrayKey,
    [(); I::COUNT]:,
{
    type Item = T;
    type IntoIter = std::array::IntoIter<T, { I::COUNT }>;

    fn into_iter(self) -> Self::IntoIter {
        self.array.into_iter()
    }
}

impl<I, T> Clone for ArrayMap<I, T>
where
    I: ArrayKey,
    [(); I::COUNT]:,
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            array: self.array.clone(),
            _marker: PhantomData,
        }
    }
}

impl<I, T> Debug for ArrayMap<I, T>
where
    I: ArrayKey,
    [(); I::COUNT]:,
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.array.fmt(f)
    }
}

impl<I, T> PartialEq for ArrayMap<I, T>
where
    I: ArrayKey,
    [(); I::COUNT]:,
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.array.eq(&other.array)
    }
}

impl<I, T> Eq for ArrayMap<I, T>
where
    I: ArrayKey,
    [(); I::COUNT]:,
    T: Eq,
{
}
