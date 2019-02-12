//! Type definition and `dodrio::Render` implementation for a single TODO item.

use crate::keys;
use dodrio::{bumpalo::Bump, on, Attribute, Node, Render, RootRender, VdomWeak};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use wasm_bindgen::{prelude::*, JsCast};

/// A single TODO item.
#[derive(Serialize, Deserialize)]
pub struct Todo<C> {
    id: usize,
    title: String,
    completed: bool,

    #[serde(skip)]
    edits: Option<String>,

    #[serde(skip)]
    _controller: PhantomData<C>,
}

/// Actions on a single TODO item that can be triggered from the UI.
pub trait TodoActions {
    /// Toggle the completion state of the TODO item with the given id.
    fn toggle_completed(root: &mut dyn RootRender, vdom: VdomWeak, id: usize);

    /// Delete the TODO item with the given id.
    fn delete(root: &mut dyn RootRender, vdom: VdomWeak, id: usize);

    /// Begin editing the TODO item with the given id.
    fn begin_editing(root: &mut dyn RootRender, vdom: VdomWeak, id: usize);

    /// Update the edits for the TODO with the given id.
    fn update_edits(root: &mut dyn RootRender, vdom: VdomWeak, id: usize, edits: String);

    /// Finish editing the TODO with the given id.
    fn finish_edits(root: &mut dyn RootRender, vdom: VdomWeak, id: usize);

    /// Cancel editing the TODO with the given id.
    fn cancel_edits(root: &mut dyn RootRender, vdom: VdomWeak, id: usize);
}

impl<C> Todo<C> {
    /// Construct a new `Todo` with the given identifier and title.
    pub fn new<S: Into<String>>(id: usize, title: S) -> Self {
        let title = title.into();
        let completed = false;
        let edits = None;
        Todo {
            id,
            title,
            completed,
            edits,
            _controller: PhantomData,
        }
    }

    /// Set this TODO item's id.
    pub fn set_id(&mut self, id: usize) {
        self.id = id;
    }

    /// Is this `Todo` complete?
    pub fn is_complete(&self) -> bool {
        self.completed
    }

    /// Mark the `Todo` as complete or not.
    pub fn set_complete(&mut self, to: bool) {
        self.completed = to;
    }

    /// Get this TODO's title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Set this TODO item's title.
    pub fn set_title<S: Into<String>>(&mut self, title: S) {
        self.title = title.into();
    }

    /// Set the edits for this TODO.
    pub fn set_edits<S: Into<String>>(&mut self, edits: Option<S>) {
        self.edits = edits.map(Into::into);
    }

    /// Take this TODO's edits, leaving `None` in their place.
    pub fn take_edits(&mut self) -> Option<String> {
        self.edits.take()
    }
}

impl<C: TodoActions> Render for Todo<C> {
    fn render<'a, 'bump>(&'a self, bump: &'bump Bump) -> dodrio::Node<'bump>
    where
        'a: 'bump,
    {
        use dodrio::bumpalo::{self, collections::String};

        let mut class = String::new_in(bump);
        if self.completed {
            class.push_str("completed ");
        }
        if self.edits.is_some() {
            class.push_str("editing");
        }

        let id = self.id;
        let title = self.edits.as_ref().unwrap_or(&self.title);
        let elem_id = bumpalo::format!(in bump, "todo-{}", id);

        let mut input_attrs = bumpalo::vec![
            in bump;
            Attribute {
                name: "class",
                value: "toggle",
            },
            Attribute {
                name: "type",
                value: "checkbox",
            }
        ];
        if self.completed {
            input_attrs.push(Attribute {
                name: "checked",
                value: "",
            });
        }
        let input_attrs = input_attrs.into_bump_slice();

        Node::element(
            bump,
            "li",
            [],
            [Attribute {
                name: "class",
                value: class.into_bump_str(),
            }],
            [
                Node::element(
                    bump,
                    "div",
                    [],
                    [Attribute {
                        name: "class",
                        value: "view",
                    }],
                    [
                        Node::element(
                            bump,
                            "input",
                            [on(bump, "click", move |root, vdom, _event| {
                                C::toggle_completed(root, vdom, id);
                            })],
                            input_attrs,
                            [],
                        ),
                        Node::element(
                            bump,
                            "label",
                            [on(bump, "dblclick", move |root, vdom, _event| {
                                C::begin_editing(root, vdom, id);
                            })],
                            [],
                            [Node::text(title)],
                        ),
                        Node::element(
                            bump,
                            "button",
                            [on(bump, "click", move |root, vdom, _event| {
                                C::delete(root, vdom, id);
                            })],
                            [Attribute {
                                name: "class",
                                value: "destroy",
                            }],
                            [],
                        ),
                    ],
                ),
                Node::element(
                    bump,
                    "input",
                    [
                        on(bump, "input", move |root, vdom, event| {
                            let input = event
                                .target()
                                .unwrap_throw()
                                .unchecked_into::<web_sys::HtmlInputElement>();
                            C::update_edits(root, vdom, id, input.value());
                        }),
                        on(bump, "blur", move |root, vdom, _event| {
                            C::finish_edits(root, vdom, id);
                        }),
                        on(bump, "keydown", move |root, vdom, event| {
                            let event = event.unchecked_into::<web_sys::KeyboardEvent>();
                            match event.key_code() {
                                keys::ENTER => C::finish_edits(root, vdom, id),
                                keys::ESCAPE => C::cancel_edits(root, vdom, id),
                                _ => {}
                            }
                        }),
                    ],
                    [
                        Attribute {
                            name: "class",
                            value: "edit",
                        },
                        Attribute {
                            name: "value",
                            value: title,
                        },
                        Attribute {
                            name: "name",
                            value: "title",
                        },
                        Attribute {
                            name: "id",
                            value: elem_id.into_bump_str(),
                        },
                    ],
                    [],
                ),
            ],
        )
    }
}
