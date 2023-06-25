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

use std::fmt::Formatter;

use slab::Slab;
use willow_protocol::*;

#[derive(Debug, PartialEq, Eq)]
pub enum NodeUpdateError {}

impl std::fmt::Display for NodeUpdateError {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            _ => panic!("unimplemented update error display"),
        }
    }
}

pub type NodeUpdateResult<T> = Result<T, NodeUpdateError>;

#[derive(Clone, Debug, PartialEq)]
pub enum NodeKind {
    Shape(Shape),
    Operation { inner: Operation, child: usize },
    Group(Vec<usize>),
}

pub struct Node {
    kind: NodeKind,
}

impl Node {
    pub fn new(kind: NodeKind) -> Self {
        Self { kind }
    }

    pub fn get_kind(&self) -> &NodeKind {
        &self.kind
    }
}

/// A Willow UI tree.
pub struct Tree {
    nodes: Slab<Node>,
}

impl Tree {
    /// Creates a new tree. The initial node (at index 0) is a [Shape::Empty].
    pub fn new() -> Self {
        let mut nodes = Slab::new();
        let empty = NodeKind::Shape(Shape::Empty);
        nodes.insert(Node::new(empty));

        Self { nodes }
    }

    /// Creates a new tree with an initial content.
    pub fn new_with_content(content: NodeContent) -> NodeUpdateResult<(Self, NodeUpdateResponse)> {
        let mut tree = Self::new();
        let response = tree.update_node(NodeUpdate { target: 0, content })?;
        Ok((tree, response))
    }

    pub fn update_node(&mut self, update: NodeUpdate) -> NodeUpdateResult<NodeUpdateResponse> {
        let node = self.nodes.get_mut(update.target).unwrap();

        let original_children: Vec<usize>;
        match node.kind.clone() {
            NodeKind::Shape(_shape) => {
                original_children = Vec::new();
            }
            _ => unimplemented!("{:?}", node.kind),
        }

        let new_node: Node;
        match update.content {
            NodeContent::Shape(shape) => {
                new_node = Node::new(NodeKind::Shape(shape));
            }
            _ => unimplemented!("{:?}", update.content),
        }

        let _ = std::mem::replace(node, new_node);

        Ok(NodeUpdateResponse { new_nodes: vec![] })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use glam::Vec2;

    #[test]
    fn create_tree() {
        let _tree = Tree::new();
    }

    #[test]
    fn update_root_shape() {
        let mut tree = Tree::new();
        let shape = Shape::Circle { radius: 1.0 };
        let content = NodeContent::Shape(shape.clone());
        let update = NodeUpdate { target: 0, content };
        tree.update_node(update).unwrap();
        let kind = NodeKind::Shape(shape);
        assert_eq!(tree.nodes[0].kind, kind);
    }

    #[test]
    fn invalid_update_target() {
        let mut tree = Tree::new();
        let content = NodeContent::Shape(Shape::Empty);
        let update = NodeUpdate { target: 1, content };
        assert!(tree.update_node(update).is_err());
    }

    #[test]
    fn update_root_operation() {
        let mut tree = Tree::new();
        tree.update_node(NodeUpdate {
            target: 0,
            content: NodeContent::Operation {
                operation: Operation::Translate { offset: Vec2::ONE },
                child: ChildUpdate::NewNode(NewNode::Shape(Shape::Circle { radius: 1.0 })),
            },
        })
        .unwrap();
    }
}
