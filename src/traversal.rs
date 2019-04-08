//! Keeps track of where we are moving in a DOM tree, and shortens traversal
//! paths between mutations to their minimal number of operations.

use bumpalo::{collections::Vec as BVec, Bump};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveTo {
    /// Move from the current node up to its parent.
    Parent,

    /// Move to the current node's n^th child.
    Child(u32),

    /// Move n siblings forward from the current node. Note that n is relative
    /// to the current node, and is not an abosolute index into the parent's
    /// children.
    Sibling(u32),
}

#[derive(Debug)]
pub struct Traversal<'a> {
    uncommitted: BVec<'a, MoveTo>,
}

impl Traversal<'_> {
    /// Construct a new `Traversal` with its internal storage backed by the
    /// given bump arena.
    pub fn new(bump: &Bump) -> Traversal {
        Traversal {
            uncommitted: BVec::new_in(bump),
        }
    }

    /// Move the traversal up in the tree.
    pub fn up(&mut self) {
        match self.uncommitted.last() {
            Some(MoveTo::Sibling(_)) => {
                self.uncommitted.pop();
                self.uncommitted.push(MoveTo::Parent);
            }
            Some(MoveTo::Child(_)) => {
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
        // Given that we never back track, and always traverse the DOM tree in
        // order, this sequence of moves should never happen. And since this
        // would be the only case where we could optimize downwards traversal
        // paths, we have nothing to check for here.
        debug_assert_ne!(self.uncommitted.last(), Some(&MoveTo::Parent));

        self.uncommitted.push(MoveTo::Child(0));
    }

    /// Move the traversal forward in the tree to the next child of the current
    /// node.
    pub fn forward(&mut self) {
        match self.uncommitted.last_mut() {
            Some(MoveTo::Child(ref mut n)) | Some(MoveTo::Sibling(ref mut n)) => {
                *n += 1;
            }
            _ => {
                self.uncommitted.push(MoveTo::Sibling(1));
            }
        }
    }

    /// Are all the traversal's moves committed? That is, are there no moves
    /// that have *not* been committed yet?
    #[inline]
    pub fn is_committed(&self) -> bool {
        self.uncommitted.is_empty()
    }

    /// Commit this traversals moves and return the optimized path from the last
    /// commit.
    pub fn commit(&mut self) -> Moves {
        Moves {
            inner: self.uncommitted.drain(..),
        }
    }
}

pub struct Moves<'a, 'b> {
    inner: bumpalo::collections::vec::Drain<'a, 'b, MoveTo>,
}

impl Iterator for Moves<'_, '_> {
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
        fn t<F>(f: F) -> Box<FnMut(&mut Traversal)>
        where
            F: 'static + FnMut(&mut Traversal),
        {
            Box::new(f) as _
        }

        let mut bump = Bump::new();
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
                    t.forward();
                }),
                vec![MoveTo::Sibling(1)],
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
                    t.forward();
                    t.up();
                }),
                vec![],
            ),
            (
                t(|t| {
                    t.down();
                    t.forward();
                }),
                vec![MoveTo::Child(1)],
            ),
            (
                t(|t| {
                    t.down();
                    t.forward();
                    t.forward();
                }),
                vec![MoveTo::Child(2)],
            ),
            (
                t(|t| {
                    t.forward();
                    t.forward();
                }),
                vec![MoveTo::Sibling(2)],
            ),
        ] {
            bump.reset();
            let mut traversal = Traversal::new(&bump);
            traverse(&mut traversal);
            let actual_moves: Vec<_> = traversal.commit().collect();
            assert_eq!(actual_moves, expected_moves);
        }
    }
}
