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
pub enum NodeUpdateError {
    /// An update's target node ID was invalid.
    InvalidTarget(u32),
}

impl std::fmt::Display for NodeUpdateError {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeUpdateError::InvalidTarget(id) => write!(fmt, "invalid update target ID: {}", id),
        }
    }
}

pub type NodeUpdateResult<T> = Result<T, NodeUpdateError>;

#[derive(Clone, Debug, PartialEq)]
pub enum NodeKind {
    Shape(Shape),
    Operation { operation: Operation, child: usize },
    Group(Vec<usize>),
}

#[derive(Clone, Debug, PartialEq)]
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
        let node = self
            .nodes
            .get_mut(update.target as usize)
            .ok_or(NodeUpdateError::InvalidTarget(update.target))?;

        let original_children: Vec<usize>;
        match node.kind.clone() {
            NodeKind::Shape(_shape) => {
                original_children = Vec::new();
            }
            _ => unimplemented!("{:?}", node.kind),
        }

        drop(node);

        let new_target: Node;
        let mut new_nodes = Vec::new();
        match update.content {
            NodeContent::Shape(shape) => {
                new_target = Node::new(NodeKind::Shape(shape));
            }
            NodeContent::Group { new_children } => {
                let mut children_ids = Vec::new();
                for child in new_children.unwrap_or_default() {
                    match child {
                        ChildUpdate::KeepIndex(_) => unimplemented!("group node re-use"),
                        ChildUpdate::NewNode(new_node) => {
                            let child_nodes = self.add_new_node(new_node).new_nodes;
                            let child = *child_nodes.last().unwrap() as usize;
                            children_ids.push(child);
                            new_nodes.extend_from_slice(&child_nodes);
                        }
                    }
                }

                new_target = Node::new(NodeKind::Group(children_ids));
            }
            _ => unimplemented!("{:?}", update.content),
        }

        let node = self.nodes.get_mut(update.target as usize).unwrap();
        let _ = std::mem::replace(node, new_target);

        Ok(NodeUpdateResponse { new_nodes })
    }

    /// Directly adds a new node to the tree.
    pub fn add_new_node(&mut self, new_node: NewNode) -> NodeUpdateResponse {
        match new_node {
            NewNode::Shape(shape) => NodeUpdateResponse {
                new_nodes: vec![self.nodes.insert(Node::new(NodeKind::Shape(shape))) as u32],
            },
            NewNode::Operation { operation, child } => {
                let mut new_nodes = self.add_new_node(*child).new_nodes;
                let child = *new_nodes.last().unwrap() as usize;
                let op_node = Node::new(NodeKind::Operation { operation, child });
                new_nodes.push(self.nodes.insert(op_node) as u32);
                NodeUpdateResponse { new_nodes }
            }
            NewNode::Group { children } => {
                let new_nodes: Vec<Vec<u32>> = children
                    .into_iter()
                    .map(|child| self.add_new_node(child).new_nodes)
                    .collect();

                let children: Vec<usize> = new_nodes
                    .iter()
                    .map(|nodes| *nodes.last().unwrap() as usize)
                    .collect();

                let group_node = Node::new(NodeKind::Group(children));
                let mut new_nodes: Vec<u32> = new_nodes.into_iter().flatten().collect();
                new_nodes.push(self.nodes.insert(group_node) as u32);

                NodeUpdateResponse { new_nodes }
            }
        }
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
    fn invalid_update_target() {
        let mut tree = Tree::new();
        let content = NodeContent::Shape(Shape::Empty);
        let update = NodeUpdate { target: 1, content };
        assert!(tree.update_node(update).is_err());
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
    fn update_root_operation() {
        let mut tree = Tree::new();
        tree.update_node(NodeUpdate {
            target: 0,
            content: NodeContent::Operation {
                operation: Operation::Translate { offset: Vec2::ONE },
                child: NewNode::Shape(Shape::Circle { radius: 1.0 }).into(),
            },
        })
        .unwrap();
    }

    #[test]
    fn update_root_group() {
        let mut tree = Tree::new();
        let response = tree
            .update_node(NodeUpdate {
                target: 0,
                content: vec![
                    NewNode::Shape(Shape::Empty),
                    NewNode::Shape(Shape::Empty),
                    NewNode::Shape(Shape::Empty),
                ]
                .into(),
            })
            .unwrap();

        assert_eq!(
            response,
            NodeUpdateResponse {
                new_nodes: vec![1, 2, 3]
            }
        );
    }

    #[test]
    fn keep_group_ids() {
        let mut tree = Tree::new();
        let response = tree
            .update_node(NodeUpdate {
                target: 0,
                content: vec![
                    NewNode::Shape(Shape::Empty),
                    NewNode::Shape(Shape::Empty),
                    NewNode::Shape(Shape::Empty),
                ]
                .into(),
            })
            .unwrap();

        let response = tree
            .update_node(NodeUpdate {
                target: 0,
                content: vec![
                    ChildUpdate::NewNode(NewNode::Shape(Shape::Circle { radius: 1.0 })),
                    ChildUpdate::KeepIndex(response.new_nodes[1]),
                    ChildUpdate::KeepIndex(response.new_nodes[2]),
                ]
                .into(),
            })
            .unwrap();

        assert_eq!(response.new_nodes, vec![4]);
        assert_eq!(tree.nodes.get(1), None);
    }
}
