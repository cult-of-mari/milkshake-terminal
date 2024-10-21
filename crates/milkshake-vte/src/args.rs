/// Convenience wrapper over [`vte::ParamsIter`](vte::ParamsIter).
pub struct Args<'a> {
    iter: vte::ParamsIter<'a>,
}

impl<'a> Args<'a> {
    pub fn new(iter: vte::ParamsIter<'a>) -> Self {
        Self { iter }
    }

    /// Return the next parameter.
    pub fn slice(&mut self) -> Option<&'a [u16]> {
        self.iter.next()
    }

    /// Return the next parameter as an array of `N` values.
    pub fn many<const N: usize>(&mut self) -> Option<[u16; N]> {
        self.slice().and_then(|slice| slice.try_into().ok())
    }

    /// Return the first value of the next parameter.
    pub fn single(&mut self) -> Option<u16> {
        self.many::<1>().map(|[single]| single)
    }

    /// Return the first value of the next parameter or `default` if it isn't present.
    pub fn single_or(&mut self, default: u16) -> u16 {
        self.single().unwrap_or(default)
    }

    /// Return the first value of the next parameter as a 1-based value.
    pub fn one_based(&mut self) -> u16 {
        self.single_or(1).max(1)
    }
}
