// Copyright (C) 2023 Marceline Cramer
//
// Willow is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// Willow is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with Willow.  If not, see <https://www.gnu.org/licenses/>.

use willow_server::{
    glam::{vec2, Vec2, Vec3A, Vec4},
    ChildUpdate, NewNode, NodeContent, NodeUpdate, Operation, Shape, Stroke, Tree,
};

pub struct State {
    pub tree: Tree,
}

impl State {
    pub fn new() -> Self {
        Self { tree: Tree::new() }
    }

    pub fn set_root(&mut self, mut component: impl ElementComponent) {
        let mut hooks = Hooks {};
        let rendered = component.render(&mut hooks).render_whole(&mut hooks);

        let mut tree = Tree::new();
        tree.update_node(NodeUpdate {
            target: 0,
            content: NodeContent::Group {
                new_children: Some(vec![ChildUpdate::NewNode(rendered)]),
            },
        })
        .unwrap();

        self.tree = tree;
    }
}

pub enum Element {
    Shape {
        shape: Shape,
    },
    Operation {
        operation: Operation,
        child: Box<Element>,
    },
    Group {
        children: Vec<Element>,
    },
    Component {
        component: Box<dyn ElementComponent>,
    },
}

impl From<Shape> for Element {
    fn from(shape: Shape) -> Element {
        Element::Shape { shape }
    }
}

impl<T: Into<Element>> From<Vec<T>> for Element {
    fn from(children: Vec<T>) -> Element {
        Element::Group {
            children: children.into_iter().map(Into::into).collect(),
        }
    }
}

impl<T: ElementComponent> From<T> for Element {
    fn from(component: T) -> Element {
        Element::Component {
            component: Box::new(component),
        }
    }
}

impl Element {
    pub fn operation(operation: Operation, child: impl Into<Element>) -> Element {
        Element::Operation {
            operation,
            child: Box::new(child.into()),
        }
    }

    pub fn render_whole(self, hooks: &mut Hooks) -> NewNode {
        use Element::*;
        match self {
            Shape { shape } => NewNode::Shape(shape),
            Operation { operation, child } => NewNode::Operation {
                operation,
                child: Box::new(child.render_whole(hooks)),
            },
            Group { children } => NewNode::Group {
                children: children
                    .into_iter()
                    .map(|child| child.render_whole(hooks))
                    .collect(),
            },
            Component { mut component } => component.render(hooks).render_whole(hooks),
        }
    }
}

pub type Color = Vec3A;

pub fn stroke_color(color: Color) -> Operation {
    Operation::Stroke(Stroke::Solid { color })
}

pub struct Theme {
    pub base: Color,
    pub surface: Color,
    pub overlay: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
}

pub struct Hooks {}

impl Hooks {
    pub fn use_theme(&mut self) -> Theme {
        fn rgb(rgb: u32) -> Color {
            let r = (rgb >> 16) as f32;
            let g = ((rgb >> 8) & 0xff) as f32;
            let b = (rgb & 0xff) as f32;
            Color::new(r, g, b) / 255.0
        }

        Theme {
            base: rgb(0x191724),
            surface: rgb(0x1f1d2e),
            overlay: rgb(0x26233a),
            text: rgb(0xe0def4),
            muted: rgb(0x6e6a86),
            accent: rgb(0x31748f),
        }
    }
}

pub trait ElementComponent: 'static {
    fn render(&mut self, hooks: &mut Hooks) -> Element;
}

impl<F> ElementComponent for F
where
    F: FnMut(&mut Hooks) -> Element + 'static,
{
    fn render(&mut self, hooks: &mut Hooks) -> Element {
        self(hooks)
    }
}

#[derive(Debug, Clone)]
pub struct MessageContent {
    pub text: String,
    pub sender: String,
    pub timestamp: String,
}

pub struct Message {
    pub content: MessageContent,
    pub size: Vec2,
}

impl ElementComponent for Message {
    fn render(&mut self, hooks: &mut Hooks) -> Element {
        let theme = hooks.use_theme();

        vec![
            Element::operation(
                stroke_color(theme.surface),
                Shape::Rectangle {
                    min: Vec2::ZERO,
                    max: self.size,
                },
            ),
            Element::operation(
                Operation::Translate {
                    offset: vec2(5.0, 15.0),
                },
                Element::operation(
                    stroke_color(theme.muted),
                    Shape::Text {
                        content: self.content.timestamp.clone(),
                        font: "unused".to_string(),
                    },
                ),
            ),
            Element::operation(
                Operation::Translate {
                    offset: vec2(55.0, 15.0),
                },
                Element::operation(
                    stroke_color(theme.text),
                    Shape::Text {
                        content: self.content.sender.clone(),
                        font: "unused".to_string(),
                    },
                ),
            ),
            Element::operation(
                Operation::Translate {
                    offset: vec2(95.0, 15.0),
                },
                Element::operation(
                    stroke_color(theme.text),
                    Shape::Text {
                        content: self.content.text.clone(),
                        font: "unused".to_string(),
                    },
                ),
            ),
        ]
        .into()
    }
}

pub struct Chat {
    pub messages: Vec<MessageContent>,
    pub width: f32,
}

impl ElementComponent for Chat {
    fn render(&mut self, hooks: &mut Hooks) -> Element {
        let theme = hooks.use_theme();

        let outer_padding = Vec2::splat(10.0);
        let inner_padding = 5.0;
        let message_width = self.width - outer_padding.x * 2.0;
        let mut messages = Vec::with_capacity(self.messages.len());
        let mut used_height = inner_padding;
        for content in self.messages.iter().cloned() {
            let message_height = 25.0;
            let size = Vec2::new(message_width, message_height);

            messages.push(Element::operation(
                Operation::Translate {
                    offset: Vec2::new(outer_padding.x, used_height),
                },
                Message { content, size },
            ));

            used_height += message_height + inner_padding;
        }

        Element::operation(
            Operation::Translate {
                offset: Vec2::new(0.0, -used_height),
            },
            vec![Element::operation(
                stroke_color(theme.base),
                Shape::Rectangle {
                    min: Vec2::ZERO,
                    max: Vec2::new(self.width, used_height),
                },
            )]
            .into_iter()
            .chain(messages)
            .collect::<Vec<Element>>(),
        )
    }
}

pub struct TextPrompt {
    pub content: String,
    pub width: f32,
}

impl ElementComponent for TextPrompt {
    fn render(&mut self, hooks: &mut Hooks) -> Element {
        let theme = hooks.use_theme();
        let padding = Vec2::splat(5.0);
        let border = 1.0;
        let height = Self::HEIGHT;
        let size = Vec2::new(self.width, height);
        let text_anchor = Vec2::splat(10.0);

        vec![
            Element::operation(
                stroke_color(theme.surface),
                Shape::Rectangle {
                    min: Vec2::new(0.0, 0.0),
                    max: size,
                },
            ),
            Element::operation(
                stroke_color(theme.accent),
                Shape::Rectangle {
                    min: padding,
                    max: size - padding,
                },
            ),
            Element::operation(
                stroke_color(theme.overlay),
                Shape::Rectangle {
                    min: padding + border,
                    max: size - padding - border,
                },
            ),
            Element::operation(
                Operation::Translate {
                    offset: Vec2::new(0.0, height)+ (-padding - border - text_anchor) * Vec2::new(-1.0, 1.0),
                },
                Element::operation(
                    stroke_color(theme.text),
                    Shape::Text {
                        content: self.content.clone(),
                        font: "unused".to_string(),
                    },
                ),
            ),
        ]
        .into()
    }
}

impl TextPrompt {
    pub const HEIGHT: f32 = 40.0;
}

pub struct MessengerApp {
    pub messages: Vec<MessageContent>,
    pub input: String,
    pub size: Vec2,
}

impl ElementComponent for MessengerApp {
    fn render(&mut self, hooks: &mut Hooks) -> Element {
        Element::operation(
            Operation::Translate {
                offset: Vec2::new(0.0, self.size.y - TextPrompt::HEIGHT),
            },
            vec![
                Element::from(Chat {
                    messages: self.messages.clone(),
                    width: self.size.x,
                }),
                Element::from(TextPrompt {
                    content: self.input.clone(),
                    width: self.size.x,
                }),
            ],
        )
    }
}
