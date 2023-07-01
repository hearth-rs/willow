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

use glam::Vec2;
use slab::Slab;

use willow_protocol::glam::{vec2, Mat2, Mat3};
pub use willow_protocol::*;

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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Aabb {
    pub min: Vec2,
    pub max: Vec2,
}

impl Aabb {
    pub const INVALID: Self = Self {
        min: Vec2::INFINITY,
        max: Vec2::NEG_INFINITY,
    };

    pub fn union(&self, other: &Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    pub fn is_intersecting(&self, other: &Self) -> bool {
        self.min.x < other.max.x
            && self.max.x > other.min.x
            && self.min.y < other.max.y
            && self.max.y > other.min.y
    }

    pub fn corners(&self) -> [Vec2; 4] {
        [
            self.min,
            vec2(self.min.x, self.max.y),
            vec2(self.max.x, self.min.y),
            self.max,
        ]
    }
}

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

    /// The bounding box of this node and its children.
    aabb: Aabb,
}

impl Node {
    pub fn new(kind: NodeKind, aabb: Aabb) -> Self {
        Self {
            kind,
            owned: false,
            reused: false,
            aabb,
        }
    }

    pub fn get_kind(&self) -> &NodeKind {
        &self.kind
    }
}

/// A Willow shape tree.
pub struct Tree {
    nodes: Slab<Node>,
}

impl Tree {
    /// Creates a new tree. The initial node (at index 0) is a [Shape::Empty].
    pub fn new() -> Self {
        let mut nodes = Slab::new();
        let empty = NodeKind::Shape(Shape::Empty);
        nodes.insert(Node::new(empty, Aabb::default()));

        Self { nodes }
    }

    /// Creates a new tree with an initial content.
    pub fn new_with_content(content: NodeContent) -> NodeUpdateResult<(Self, NodeUpdateResponse)> {
        let mut tree = Self::new();
        let response = tree.update_node(NodeUpdate { target: 0, content })?;
        Ok((tree, response))
    }

    pub fn update_node(&mut self, update: NodeUpdate) -> NodeUpdateResult<NodeUpdateResponse> {
        let original_children = self.begin_children_update(update.target as usize)?;
        let update_result = self.update_node_inner(update);
        let remove_unused = !update_result.is_err();
        self.end_children_update(original_children, remove_unused); // always clean up update
        update_result
    }

    /// Updates a [Node].
    ///
    /// Requires an update to be a progress using [Self::begin_children_update].
    fn update_node_inner(&mut self, update: NodeUpdate) -> NodeUpdateResult<NodeUpdateResponse> {
        let mut new_nodes = Vec::new();

        let node_kind = match update.content {
            NodeContent::Shape(shape) => NodeKind::Shape(shape),
            NodeContent::Operation { operation, child } => {
                let child = self.update_child(&mut new_nodes, child)? as usize;
                NodeKind::Operation { operation, child }
            }
            NodeContent::Group { new_children } => {
                let mut children_idxs = Vec::new();
                for child in new_children.unwrap_or_default() {
                    let child = self.update_child(&mut new_nodes, child)?;
                    children_idxs.push(child as usize);
                }

                NodeKind::Group(children_idxs)
            }
        };

        let new_node = self.create_new_node(node_kind);
        let node = self.nodes.get_mut(update.target as usize).unwrap();
        let _ = std::mem::replace(node, new_node);

        Ok(NodeUpdateResponse { new_nodes })
    }

    /// Retrieves the children of a node, sets their update flags, and returns
    /// their indices.
    fn begin_children_update(&mut self, parent: usize) -> NodeUpdateResult<Vec<usize>> {
        let node = self
            .nodes
            .get_mut(parent)
            .ok_or(NodeUpdateError::InvalidTarget)?;

        let children = match node.kind.clone() {
            NodeKind::Shape(_shape) => Vec::new(),
            NodeKind::Operation { child, .. } => vec![child],
            NodeKind::Group(children) => children,
        };

        drop(node);

        for child in children.iter() {
            self.nodes.get_mut(*child).unwrap().owned = true;
        }

        Ok(children)
    }

    /// Unsets the update flags of a node's children and optionally frees
    /// unused children.
    fn end_children_update(&mut self, children: Vec<usize>, remove_unused: bool) {
        for child in children {
            let node = self.nodes.get_mut(child).unwrap();
            node.owned = false;

            if node.reused {
                node.reused = false;
                continue;
            }

            if remove_unused {
                self.nodes.remove(child);
            }
        }
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
        let kind = match node {
            NewNode::Shape(shape) => NodeKind::Shape(shape),
            NewNode::Operation { operation, child } => {
                let child = self.add_new_node(new_indices, *child) as usize;
                NodeKind::Operation { operation, child }
            }
            NewNode::Group { children } => {
                let children: Vec<usize> = children
                    .into_iter()
                    .map(|child| self.add_new_node(new_indices, child) as usize)
                    .collect();

                NodeKind::Group(children)
            }
        };

        let node = self.create_new_node(kind);
        let index = self.nodes.insert(node) as u32;
        new_indices.push(index);
        index
    }

    /// Creates a [Node] of the given kind.
    pub fn create_new_node(&mut self, kind: NodeKind) -> Node {
        let aabb = match &kind {
            NodeKind::Shape(shape) => match shape.clone() {
                Shape::Empty => Aabb::INVALID,
                Shape::Circle { radius } => Aabb {
                    min: -Vec2::splat(radius),
                    max: Vec2::splat(radius),
                },
                Shape::Rectangle { min, max } => Aabb { min, max },
                Shape::Text { content, .. } => Aabb {
                    // TODO server-side shaping
                    min: Vec2::new(-5.0, -10.0),
                    max: Vec2::new(content.len() as f32 * 10.0, 5.0),
                },
            },
            NodeKind::Operation { operation, child } => {
                let child_aabb = self.nodes[*child].aabb.clone();

                match operation {
                    Operation::Translate { offset } => Aabb {
                        min: child_aabb.min + *offset,
                        max: child_aabb.max + *offset,
                    },
                    Operation::Rotation { angle } => {
                        let corners = child_aabb.corners();

                        let mat = Mat2::from_angle(*angle);
                        let mut min = Vec2::INFINITY;
                        let mut max = Vec2::NEG_INFINITY;

                        for corner in corners {
                            let corner = mat * corner;
                            min = min.min(corner);
                            max = max.max(corner);
                        }

                        Aabb { min, max }
                    }
                    Operation::Scale { scale } => Aabb {
                        min: child_aabb.min * *scale,
                        max: child_aabb.max * *scale,
                    },
                    Operation::Blur { radius } => Aabb {
                        min: child_aabb.min - *radius,
                        max: child_aabb.max + *radius,
                    },
                    _ => child_aabb,
                }
            }
            NodeKind::Group(children) => {
                let mut aabb = Aabb::INVALID;
                for child in children.iter() {
                    let child_aabb = &self.nodes[*child].aabb;
                    aabb = aabb.union(child_aabb);
                }

                aabb
            }
        };

        Node::new(kind, aabb)
    }

    /// Walks the entire tree using a type implementing [WalkTree].
    pub fn walk(&mut self, walker: &mut impl WalkTree, aabb: &Aabb) {
        let mut stack = Vec::new();
        let mut transforms = vec![Mat3::default()];
        stack.push((0, true));

        while let Some((index, ascending)) = stack.pop() {
            let node = self.nodes.get(index).unwrap();
            let current_transform = transforms.last().unwrap().clone();

            if ascending {
                let corners = node.aabb.corners();

                let mut min = Vec2::INFINITY;
                let mut max = Vec2::NEG_INFINITY;

                for corner in corners {
                    let corner = current_transform.transform_point2(corner);
                    min = min.min(corner);
                    max = max.max(corner);
                }

                let child_aabb = Aabb { min, max };
                if !aabb.is_intersecting(&child_aabb) {
                    continue;
                }

                // walker.on_aabb(&node.aabb);
            }

            match &node.kind {
                NodeKind::Shape(shape) if ascending => walker.on_shape(shape),
                NodeKind::Operation { operation, child } => {
                    if ascending {
                        walker.push_operation(operation);
                        stack.push((index, false));
                        stack.push((*child, true));

                        let new_transform = match operation {
                            Operation::Translate { offset } => {
                                Some(Mat3::from_translation(*offset))
                            }
                            Operation::Rotation { angle } => Some(Mat3::from_rotation_z(*angle)),
                            Operation::Scale { scale } => {
                                Some(Mat3::from_scale(Vec2::splat(*scale)))
                            }
                            _ => None,
                        };

                        if let Some(new_transform) = new_transform {
                            transforms.push(current_transform * new_transform);
                        }
                    } else {
                        walker.pop_operation(operation);

                        match operation {
                            Operation::Translate { .. }
                            | Operation::Rotation { .. }
                            | Operation::Scale { .. } => {
                                transforms.pop();
                            }
                            _ => {}
                        }
                    }
                }
                NodeKind::Group(children) if ascending => stack.extend_from_slice(
                    children
                        .iter()
                        .map(|child| (*child, true))
                        .rev() // stack pops in reverse order
                        .collect::<Vec<_>>()
                        .as_slice(),
                ),
                _ => {}
            }
        }
    }
}

pub trait WalkTree {
    fn on_shape(&mut self, shape: &Shape);

    fn push_operation(&mut self, operation: &Operation);

    fn pop_operation(&mut self, operation: &Operation);

    fn on_aabb(&mut self, aabb: &Aabb);
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

    #[test]
    fn failed_update_unsets_node_flags() {
        let mut tree = Tree::new();

        let new_nodes = tree
            .update_node(NodeUpdate {
                target: 0,
                content: vec![NewNode::Shape(Shape::Empty)].into(),
            })
            .unwrap()
            .new_nodes;

        assert_eq!(new_nodes, vec![1]);

        tree.update_node(NodeUpdate {
            target: 0,
            content: vec![ChildUpdate::KeepIndex(2)].into(),
        })
        .unwrap_err();

        assert!(!tree.nodes[1].owned);
    }
}
