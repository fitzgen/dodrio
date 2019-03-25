use bumpalo::Bump;

/// Common context available to all `Render` implementations.
///
/// Notably, the `RenderContext` gives access to the bump arena that the virtual
/// DOM should be allocated within. This is available via the `bump` field.
#[derive(Debug)]
pub struct RenderContext<'bump> {
    /// The underlying bump arena that virtual DOMs are rendered into.
    ///
    /// ## Example
    ///
    /// ```
    /// use dodrio::RenderContext;
    ///
    /// // Given a rendering context, allocate an i32 inside its bump arena.
    /// fn foo<'a>(cx: &mut RenderContext<'a>) -> &'a mut i32 {
    ///     cx.bump.alloc(42)
    /// }
    /// ```
    pub bump: &'bump Bump,

    // Prevent exhaustive matching on the rendering context, so we can add more
    // members in a semver-compatible way.
    _non_exhaustive: (),
}

impl<'bump> RenderContext<'bump> {
    // Don't use this! For internal use only! Subject to change in breaking ways
    // without a breaking version bump!
    #[doc(hidden)]
    pub fn new(bump: &'bump Bump) -> Self {
        RenderContext {
            bump,
            _non_exhaustive: (),
        }
    }
}

impl<'a, 'bump> From<&'a RenderContext<'bump>> for &'bump Bump {
    #[inline]
    fn from(cx: &'a RenderContext<'bump>) -> &'bump Bump {
        cx.bump
    }
}

impl<'a, 'b, 'bump> From<&'a &'b mut RenderContext<'bump>> for &'bump Bump {
    #[inline]
    fn from(cx: &'a &'b mut RenderContext<'bump>) -> &'bump Bump {
        cx.bump
    }
}
