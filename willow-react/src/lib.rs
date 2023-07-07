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

use willow_server::*;
use glam::Vec3A;

pub use willow_server;

pub struct State {
    pub tree: Tree,
}

impl State {
    pub fn new() -> Self {
        Self { tree: Tree::new() }
    }

    pub fn set_root(&mut self, mut component: Box<dyn ElementComponent>) {
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
