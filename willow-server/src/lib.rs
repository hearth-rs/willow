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
    /// This update's target node index was invalid.
    InvalidTarget,

    /// A [ChildUpdate::KeepIndex] contained an invalid node index.
    InvalidKeepIndex(u32),

    /// A [ChildUpdate::KeepIndex] contained a node index not owned by the target.
    UnownedKeepIndex(u32),

    /// Two instances of [ChildUpdate::KeepIndex] refer to the same index.
    DuplicateKeepIndex(u32),
}

impl std::fmt::Display for NodeUpdateError {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
        use NodeUpdateError::*;
        match self {
            InvalidTarget => write!(fmt, "invalid update target index"),
            InvalidKeepIndex(idx) => write!(fmt, "invalid kept index: {}", idx),
            UnownedKeepIndex(idx) => write!(fmt, "unowned kept index: {}", idx),
            DuplicateKeepIndex(idx) => write!(fmt, "attempt to keep an index twice: {}", idx),
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

    /// Whether this value is marked as originally belonging to the target node
    /// during an update.
    owned: bool,

    /// Whether this value has been reused by the target node during an update.
    reused: bool,
}

impl Node {
    pub fn new(kind: NodeKind) -> Self {
        Self {
            kind,
            owned: false,
            reused: false,
        }
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
            .ok_or(NodeUpdateError::InvalidTarget)?;

        let original_children = match node.kind.clone() {
            NodeKind::Shape(_shape) => Vec::new(),
            NodeKind::Operation { child, .. } => vec![child],
            NodeKind::Group(children) => children,
        };

        drop(node);

        for child in original_children.iter() {
            self.nodes.get_mut(*child).unwrap().owned = true;
        }

        let new_target: Node;
        let mut new_nodes = Vec::new();
        match update.content {
            NodeContent::Shape(shape) => {
                new_target = Node::new(NodeKind::Shape(shape));
            }
            NodeContent::Operation { operation, child } => {
                let child = self.update_child(&mut new_nodes, child)? as usize;
                let node = NodeKind::Operation { operation, child };
                new_target = Node::new(node);
            }
            NodeContent::Group { new_children } => {
                let mut children_idxs = Vec::new();
                for child in new_children.unwrap_or_default() {
                    let child = self.update_child(&mut new_nodes, child)?;
                    children_idxs.push(child as usize);
                }

                new_target = Node::new(NodeKind::Group(children_idxs));
            }
        }

        for child in original_children {
            let node = self.nodes.get_mut(child).unwrap();
            if node.reused {
                node.owned = false;
                node.reused = false;
                continue;
            }

            self.nodes.remove(child);
        }

        let node = self.nodes.get_mut(update.target as usize).unwrap();
        let _ = std::mem::replace(node, new_target);

        Ok(NodeUpdateResponse { new_nodes })
    }

    /// Consumes a [ChildUpdate] during a node update.
    fn update_child(
        &mut self,
        new_indices: &mut Vec<u32>,
        child: ChildUpdate,
    ) -> NodeUpdateResult<u32> {
        match child {
            ChildUpdate::KeepIndex(idx) => {
                let node = self
                    .nodes
                    .get_mut(idx as usize)
                    .ok_or(NodeUpdateError::InvalidKeepIndex(idx))?;

                if !node.owned {
                    Err(NodeUpdateError::UnownedKeepIndex(idx))
                } else if node.reused {
                    Err(NodeUpdateError::DuplicateKeepIndex(idx))
                } else {
                    node.reused = true;
                    Ok(idx)
                }
            }
            ChildUpdate::NewNode(new_node) => Ok(self.add_new_node(new_indices, new_node)),
        }
    }

    /// Directly adds a new node to the tree, writing the allocated ID of the
    /// node and its children to the given buffer. Returns the ID of the new
    /// node.
    pub fn add_new_node(&mut self, new_indices: &mut Vec<u32>, node: NewNode) -> u32 {
        match node {
            NewNode::Shape(shape) => {
                let node = Node::new(NodeKind::Shape(shape));
                let index = self.nodes.insert(node) as u32;
                new_indices.push(index);
                index
            }
            NewNode::Operation { operation, child } => {
                let child = self.add_new_node(new_indices, *child) as usize;
                let op_node = Node::new(NodeKind::Operation { operation, child });
                let index = self.nodes.insert(op_node) as u32;
                new_indices.push(index);
                index
            }
            NewNode::Group { children } => {
                let children: Vec<usize> = children
                    .into_iter()
                    .map(|child| self.add_new_node(new_indices, child) as usize)
                    .collect();

                let group_node = Node::new(NodeKind::Group(children));
                let index = self.nodes.insert(group_node) as u32;
                new_indices.push(index);
                index
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
    fn keep_group_index() {
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

    #[test]
    fn invalid_keep_index() {
        let mut tree = Tree::new();
        let new_nodes = tree
            .update_node(NodeUpdate {
                target: 0,
                content: vec![NewNode::Shape(Shape::Empty)].into(),
            })
            .unwrap()
            .new_nodes;

        assert_eq!(new_nodes, vec![1]);

        let result = tree.update_node(NodeUpdate {
            target: 0,
            content: vec![ChildUpdate::KeepIndex(2)].into(),
        });

        assert_eq!(result, Err(NodeUpdateError::InvalidKeepIndex(2)));
    }

    #[test]
    fn self_keep_index() {
        let mut tree = Tree::new();
        let result = tree.update_node(NodeUpdate {
            target: 0,
            content: vec![ChildUpdate::KeepIndex(0)].into(),
        });

        assert_eq!(result, Err(NodeUpdateError::UnownedKeepIndex(0)));
    }
}
