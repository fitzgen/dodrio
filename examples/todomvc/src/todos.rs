//! Type definitions and `dodrio::Render` implementation for a collection of
//! TODO items.

use crate::controller::Controller;
use crate::todo::{Todo, TodoActions};
use crate::visibility::Visibility;
use crate::{keys, utils};
use dodrio::{
    bumpalo::{self, Bump},
    on, Attribute, Node, Render, RootRender, VdomWeak,
};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::mem;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// A collection of TODOs.
#[derive(Default, Serialize, Deserialize)]
#[serde(rename = "todos-dodrio", bound = "")]
pub struct Todos<C = Controller> {
    todos: Vec<Todo<C>>,

    #[serde(skip)]
    draft: String,

    #[serde(skip)]
    visibility: Visibility,

    #[serde(skip)]
    _controller: PhantomData<C>,
}

/// Actions for `Todos` that can be triggered by UI interactions.
pub trait TodosActions: TodoActions {
    /// Toggle the completion state of all TODO items.
    fn toggle_all(root: &mut dyn RootRender, vdom: VdomWeak);

    /// Update the draft TODO item's text.
    fn update_draft(root: &mut dyn RootRender, vdom: VdomWeak, draft: String);

    /// Finish the current draft TODO item and add it to the collection of
    /// TODOs.
    fn finish_draft(root: &mut dyn RootRender, vdom: VdomWeak);

    /// Change the TODO item visibility filtering to the given `Visibility`.
    fn change_visibility(root: &mut dyn RootRender, vdom: VdomWeak, vis: Visibility);

    /// Delete all completed TODO items.
    fn delete_completed(root: &mut dyn RootRender, vdom: VdomWeak);
}

impl<C> Todos<C> {
    /// Construct a new TODOs set.
    ///
    /// If an existing set is available in local storage, then us that,
    /// otherwise create a new set.
    pub fn new() -> Self
    where
        C: Default,
    {
        Self::from_local_storage().unwrap_or_default()
    }

    /// Deserialize a set of TODOs from local storage.
    pub fn from_local_storage() -> Option<Self> {
        utils::local_storage()
            .get("todomvc-dodrio")
            .ok()
            .and_then(|opt| opt)
            .and_then(|json| serde_json::from_str(&json).ok())
    }

    /// Serialize this set of TODOs to local storage.
    pub fn save_to_local_storage(&self) {
        let serialized = serde_json::to_string(self).unwrap_throw();
        utils::local_storage()
            .set("todomvc-dodrio", &serialized)
            .unwrap_throw();
    }

    /// Add a new TODO item to this collection.
    pub fn add_todo(&mut self, todo: Todo<C>) {
        self.todos.push(todo);
    }

    /// Delete the TODO with the given id.
    pub fn delete_todo(&mut self, id: usize) {
        self.todos.remove(id);
        self.fix_ids();
    }

    /// Delete all completed TODO items.
    pub fn delete_completed(&mut self) {
        self.todos.retain(|t| !t.is_complete());
        self.fix_ids();
    }

    // Fix all TODO identifiers so that they match their index once again.
    fn fix_ids(&mut self) {
        for (id, todo) in self.todos.iter_mut().enumerate() {
            todo.set_id(id);
        }
    }

    /// Get a shared slice of the underlying set of TODO items.
    pub fn todos(&self) -> &[Todo<C>] {
        &self.todos
    }

    /// Get an exclusive slice of the underlying set of TODO items.
    pub fn todos_mut(&mut self) -> &mut [Todo<C>] {
        &mut self.todos
    }

    /// Set the draft TODO item text.
    pub fn set_draft<S: Into<String>>(&mut self, draft: S) {
        self.draft = draft.into();
    }

    /// Take the current draft text and replace it with an empty string.
    pub fn take_draft(&mut self) -> String {
        mem::replace(&mut self.draft, String::new())
    }

    /// Get the current visibility for these TODOs.
    pub fn visibility(&self) -> Visibility {
        self.visibility
    }

    /// Set the visibility for these TODOS.
    pub fn set_visibility(&mut self, vis: Visibility) {
        self.visibility = vis;
    }
}

/// Rendering helpers.
impl<C: TodosActions> Todos<C> {
    fn header<'a, 'bump>(&'a self, bump: &'bump Bump) -> Node<'bump>
    where
        'a: 'bump,
    {
        Node::element(
            bump,
            "header",
            [],
            [Attribute {
                name: "class",
                value: "header",
            }],
            [
                Node::element(bump, "h1", [], [], [Node::text("todos")]),
                Node::element(
                    bump,
                    "input",
                    [
                        on(bump, "input", |root, vdom, event| {
                            let input = event
                                .target()
                                .unwrap_throw()
                                .unchecked_into::<web_sys::HtmlInputElement>();
                            C::update_draft(root, vdom, input.value());
                        }),
                        on(bump, "keydown", |root, vdom, event| {
                            let event = event.unchecked_into::<web_sys::KeyboardEvent>();
                            if event.key_code() == keys::ENTER {
                                C::finish_draft(root, vdom);
                            }
                        }),
                    ],
                    [
                        Attribute {
                            name: "class",
                            value: "new-todo",
                        },
                        Attribute {
                            name: "placeholder",
                            value: "What needs to be done?",
                        },
                        Attribute {
                            name: "autofocus",
                            value: "",
                        },
                        Attribute {
                            name: "value",
                            value: &self.draft,
                        },
                    ],
                    [],
                ),
            ],
        )
    }

    fn todos_list<'a, 'bump>(&'a self, bump: &'bump Bump) -> Node<'bump>
    where
        'a: 'bump,
    {
        use dodrio::bumpalo::collections::Vec;

        let mut todos = Vec::new_in(bump);
        todos.extend(
            self.todos
                .iter()
                .filter(|t| match self.visibility {
                    Visibility::All => true,
                    Visibility::Active => !t.is_complete(),
                    Visibility::Completed => t.is_complete(),
                })
                .map(|t| t.render(bump)),
        );
        let todos = todos.into_bump_slice();

        let mut input_attrs = bumpalo::vec![
            in bump;
            Attribute {
                name: "class",
                value: "toggle-all",
            },
            Attribute {
                name: "id",
                value: "toggle-all",
            },
            Attribute {
                name: "type",
                value: "checkbox",
            },
            Attribute {
                name: "name",
                value: "toggle",
            }
        ];
        if self.todos.iter().all(|t| t.is_complete()) {
            input_attrs.push(Attribute {
                name: "checked",
                value: "",
            });
        }
        let input_attrs = input_attrs.into_bump_slice();

        Node::element(
            bump,
            "section",
            [],
            [
                Attribute {
                    name: "class",
                    value: "main",
                },
                Attribute {
                    name: "visibility",
                    value: if self.todos.is_empty() {
                        "hidden"
                    } else {
                        "visible"
                    },
                },
            ],
            [
                Node::element(
                    bump,
                    "input",
                    [on(bump, "click", |root, vdom, _event| {
                        C::toggle_all(root, vdom);
                    })],
                    input_attrs,
                    [],
                ),
                Node::element(
                    bump,
                    "label",
                    [],
                    [Attribute {
                        name: "for",
                        value: "toggle-all",
                    }],
                    [Node::text("Mark all as complete")],
                ),
                Node::element(
                    bump,
                    "ul",
                    [],
                    [Attribute {
                        name: "class",
                        value: "todo-list",
                    }],
                    todos,
                ),
            ],
        )
    }

    fn footer<'a, 'bump>(&'a self, bump: &'bump Bump) -> Node<'bump>
    where
        'a: 'bump,
    {
        let completed_count = self.todos.iter().filter(|t| t.is_complete()).count();
        let incomplete_count = self.todos.len() - completed_count;
        let items_left = if incomplete_count == 1 {
            " item left"
        } else {
            " items left"
        };
        let incomplete_count = bumpalo::format!(in bump, "{}", incomplete_count);

        let attrs = if self.todos.is_empty() {
            &bump.alloc([
                Attribute {
                    name: "class",
                    value: "footer",
                },
                Attribute {
                    name: "hidden",
                    value: "",
                },
            ])[..]
        } else {
            &bump.alloc([Attribute {
                name: "class",
                value: "footer",
            }])[..]
        };

        let clear_completed_text = bumpalo::format!(
            in bump,
            "Clear completed ({})",
            self.todos.iter().filter(|t| t.is_complete()).count()
        );

        Node::element(
            bump,
            "footer",
            [],
            attrs,
            [
                Node::element(
                    bump,
                    "span",
                    [],
                    [Attribute {
                        name: "class",
                        value: "todo-count",
                    }],
                    [
                        Node::element(
                            bump,
                            "strong",
                            [],
                            [],
                            [Node::text(incomplete_count.into_bump_str())],
                        ),
                        Node::text(items_left),
                    ],
                ),
                Node::element(
                    bump,
                    "ul",
                    [],
                    [Attribute {
                        name: "class",
                        value: "filters",
                    }],
                    [
                        self.visibility_swap(bump, "#/", Visibility::All),
                        self.visibility_swap(bump, "#/active", Visibility::Active),
                        self.visibility_swap(bump, "#/completed", Visibility::Completed),
                    ],
                ),
                Node::element(
                    bump,
                    "button",
                    [on(bump, "click", |root, vdom, _event| {
                        C::delete_completed(root, vdom);
                    })],
                    {
                        let mut attrs = bumpalo::vec![
                            in bump;
                            Attribute {
                                name: "class",
                                value: "clear-completed",
                            }
                        ];
                        if self.todos.iter().all(|t| !t.is_complete()) {
                            attrs.push(Attribute {
                                name: "hidden",
                                value: "",
                            });
                        }
                        attrs
                    },
                    [Node::text(clear_completed_text.into_bump_str())],
                ),
            ],
        )
    }

    fn visibility_swap<'a, 'bump>(
        &'a self,
        bump: &'bump Bump,
        url: &'static str,
        target_vis: Visibility,
    ) -> Node<'bump>
    where
        'a: 'bump,
    {
        Node::element(
            bump,
            "li",
            [on(bump, "click", move |root, vdom, _event| {
                C::change_visibility(root, vdom, target_vis);
            })],
            [],
            [Node::element(
                bump,
                "a",
                [],
                [
                    Attribute {
                        name: "href",
                        value: url,
                    },
                    Attribute {
                        name: "class",
                        value: if self.visibility == target_vis {
                            "selected"
                        } else {
                            ""
                        },
                    },
                ],
                [Node::text(target_vis.label())],
            )],
        )
    }
}

impl<C: TodosActions> Render for Todos<C> {
    fn render<'a, 'bump>(&'a self, bump: &'bump Bump) -> Node<'bump>
    where
        'a: 'bump,
    {
        Node::element(
            bump,
            "div",
            [],
            [],
            [self.header(bump), self.todos_list(bump), self.footer(bump)],
        )
    }
}
