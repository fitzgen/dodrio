//! Keeps track of where we are moving in a DOM tree, and shortens traversal
//! paths between mutations to their minimal number of operations.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveTo {
    /// Move from the current node up to its parent.
    Parent,

    /// Move to the current node's n^th child.
    Child(u32),

    /// Move to the current node's n^th from last child.
    ReverseChild(u32),

    /// Move to the n^th sibling. Not relative from the current
    /// location. Absolute indexed within all of the current siblings.
    Sibling(u32),

    /// Move to the n^th from last sibling. Not relative from the current
    /// location. Absolute indexed within all of the current siblings.
    ReverseSibling(u32),

    /// Move down to the given saved temporary child.
    TempChild(u32),
}

#[derive(Debug)]
pub struct Traversal {
    uncommitted: Vec<MoveTo>,
}

impl Traversal {
    /// Construct a new `Traversal` with its internal storage backed by the
    /// given bump arena.
    pub fn new() -> Traversal {
        Traversal {
            uncommitted: Vec::with_capacity(32),
        }
    }

    /// Move the traversal up in the tree.
    pub fn up(&mut self) {
        match self.uncommitted.last() {
            Some(MoveTo::Sibling(_)) | Some(MoveTo::ReverseSibling(_)) => {
                self.uncommitted.pop();
                self.uncommitted.push(MoveTo::Parent);
            }
            Some(MoveTo::TempChild(_)) | Some(MoveTo::Child(_)) | Some(MoveTo::ReverseChild(_)) => {
                self.uncommitted.pop();
                // And we're back at the parent.
            }
            _ => {
                self.uncommitted.push(MoveTo::Parent);
            }
        }
    }

    /// Move the traversal down in the tree to the first child of the current
    /// node.
    pub fn down(&mut self) {
        if let Some(&MoveTo::Parent) = self.uncommitted.last() {
            self.uncommitted.pop();
            self.sibling(0);
        } else {
            self.uncommitted.push(MoveTo::Child(0));
        }
    }

    /// Move the traversal to the n^th sibling.
    pub fn sibling(&mut self, index: usize) {
        let index = index as u32;
        match self.uncommitted.last_mut() {
            Some(MoveTo::Sibling(ref mut n)) | Some(MoveTo::Child(ref mut n)) => {
                *n = index;
            }
            Some(MoveTo::ReverseSibling(_)) => {
                self.uncommitted.pop();
                self.uncommitted.push(MoveTo::Sibling(index));
            }
            Some(MoveTo::TempChild(_)) | Some(MoveTo::ReverseChild(_)) => {
                self.uncommitted.pop();
                self.uncommitted.push(MoveTo::Child(index))
            }
            _ => {
                self.uncommitted.push(MoveTo::Sibling(index));
            }
        }
    }

    /// Move the the n^th from last sibling.
    pub fn reverse_sibling(&mut self, index: usize) {
        let index = index as u32;
        match self.uncommitted.last_mut() {
            Some(MoveTo::ReverseSibling(ref mut n)) | Some(MoveTo::ReverseChild(ref mut n)) => {
                *n = index;
            }
            Some(MoveTo::Sibling(_)) => {
                self.uncommitted.pop();
                self.uncommitted.push(MoveTo::ReverseSibling(index));
            }
            Some(MoveTo::TempChild(_)) | Some(MoveTo::Child(_)) => {
                self.uncommitted.pop();
                self.uncommitted.push(MoveTo::ReverseChild(index))
            }
            _ => {
                self.uncommitted.push(MoveTo::ReverseSibling(index));
            }
        }
    }

    /// Go to the given saved temporary.
    pub fn down_to_temp(&mut self, temp: u32) {
        match self.uncommitted.last() {
            Some(MoveTo::Sibling(_)) | Some(MoveTo::ReverseSibling(_)) => {
                self.uncommitted.pop();
            }
            Some(MoveTo::Parent)
            | Some(MoveTo::TempChild(_))
            | Some(MoveTo::Child(_))
            | Some(MoveTo::ReverseChild(_))
            | None => {
                // Can't remove moves to parents since we rely on their stack
                // pops.
            }
        }
        self.uncommitted.push(MoveTo::TempChild(temp));
    }

    /// Are all the traversal's moves committed? That is, are there no moves
    /// that have *not* been committed yet?
    #[inline]
    pub fn is_committed(&self) -> bool {
        self.uncommitted.is_empty()
    }

    /// Commit this traversals moves and return the optimized path from the last
    /// commit.
    #[inline]
    pub fn commit(&mut self) -> Moves {
        Moves {
            inner: self.uncommitted.drain(..),
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.uncommitted.clear();
    }
}

pub struct Moves<'a> {
    inner: std::vec::Drain<'a, MoveTo>,
}

impl Iterator for Moves<'_> {
    type Item = MoveTo;

    #[inline]
    fn next(&mut self) -> Option<MoveTo> {
        self.inner.next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_traversal() {
        fn t<F>(f: F) -> Box<dyn FnMut(&mut Traversal)>
        where
            F: 'static + FnMut(&mut Traversal),
        {
            Box::new(f) as _
        }

        for (mut traverse, expected_moves) in vec![
            (
                t(|t| {
                    t.down();
                }),
                vec![MoveTo::Child(0)],
            ),
            (
                t(|t| {
                    t.up();
                }),
                vec![MoveTo::Parent],
            ),
            (
                t(|t| {
                    t.sibling(42);
                }),
                vec![MoveTo::Sibling(42)],
            ),
            (
                t(|t| {
                    t.down();
                    t.up();
                }),
                vec![],
            ),
            (
                t(|t| {
                    t.down();
                    t.sibling(2);
                    t.up();
                }),
                vec![],
            ),
            (
                t(|t| {
                    t.down();
                    t.sibling(3);
                }),
                vec![MoveTo::Child(3)],
            ),
            (
                t(|t| {
                    t.down();
                    t.sibling(4);
                    t.sibling(8);
                }),
                vec![MoveTo::Child(8)],
            ),
            (
                t(|t| {
                    t.sibling(1);
                    t.sibling(1);
                }),
                vec![MoveTo::Sibling(1)],
            ),
            (
                t(|t| {
                    t.reverse_sibling(3);
                }),
                vec![MoveTo::ReverseSibling(3)],
            ),
            (
                t(|t| {
                    t.down();
                    t.reverse_sibling(3);
                }),
                vec![MoveTo::ReverseChild(3)],
            ),
            (
                t(|t| {
                    t.down();
                    t.reverse_sibling(3);
                    t.up();
                }),
                vec![],
            ),
            (
                t(|t| {
                    t.down();
                    t.reverse_sibling(3);
                    t.reverse_sibling(6);
                }),
                vec![MoveTo::ReverseChild(6)],
            ),
            (
                t(|t| {
                    t.up();
                    t.reverse_sibling(3);
                    t.reverse_sibling(6);
                }),
                vec![MoveTo::Parent, MoveTo::ReverseSibling(6)],
            ),
            (
                t(|t| {
                    t.up();
                    t.sibling(3);
                    t.sibling(6);
                }),
                vec![MoveTo::Parent, MoveTo::Sibling(6)],
            ),
            (
                t(|t| {
                    t.sibling(3);
                    t.sibling(6);
                    t.up();
                }),
                vec![MoveTo::Parent],
            ),
            (
                t(|t| {
                    t.reverse_sibling(3);
                    t.reverse_sibling(6);
                    t.up();
                }),
                vec![MoveTo::Parent],
            ),
            (
                t(|t| {
                    t.down();
                    t.down_to_temp(3);
                }),
                vec![MoveTo::Child(0), MoveTo::TempChild(3)],
            ),
            (
                t(|t| {
                    t.down_to_temp(3);
                    t.sibling(5);
                }),
                vec![MoveTo::Child(5)],
            ),
            (
                t(|t| {
                    t.down_to_temp(3);
                    t.reverse_sibling(5);
                }),
                vec![MoveTo::ReverseChild(5)],
            ),
            (
                t(|t| {
                    t.down_to_temp(3);
                    t.up();
                }),
                vec![],
            ),
            (
                t(|t| {
                    t.sibling(2);
                    t.up();
                    t.down_to_temp(3);
                }),
                vec![MoveTo::Parent, MoveTo::TempChild(3)],
            ),
            (
                t(|t| {
                    t.up();
                    t.down_to_temp(3);
                }),
                vec![MoveTo::Parent, MoveTo::TempChild(3)],
            ),
        ] {
            let mut traversal = Traversal::new();
            traverse(&mut traversal);
            let actual_moves: Vec<_> = traversal.commit().collect();
            assert_eq!(actual_moves, expected_moves);
        }
    }
}
