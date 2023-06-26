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

use glam::{Vec2, Vec3A};
use serde::{Deserialize, Serialize};

pub use glam;

/// A message sent to the Willow server to update a shape tree. The server
/// responds with a [NodeUpdateResponse] message for each updated node.
#[derive(Debug, Deserialize, Serialize)]
pub struct TreeUpdate {
    /// The ID of the targeted tree.
    pub target: u32,

    /// A list of node updates to apply to the tree.
    pub updates: Vec<NodeUpdate>,
}

/// Updates a node in a [TreeUpdate].
#[derive(Debug, Deserialize, Serialize)]
pub struct NodeUpdate {
    /// The targeted node's index.
    pub target: u32,

    /// The content of the update.
    pub content: NodeContent,
}

/// The content that [Update] writes to a targeted node.
#[derive(Debug, Deserialize, Serialize)]
pub enum NodeContent {
    /// Updates the targeted node into a [Shape].
    Shape(Shape),

    /// Updates the targeted node into an [Operation].
    Operation {
        /// The kind of operation.
        operation: Operation,

        /// The child of this operation node.
        child: ChildUpdate,
    },

    /// Makes the targeted node into a group with the given children.
    Group {
        new_children: Option<Vec<ChildUpdate>>,
    },
}

impl<T> From<Vec<T>> for NodeContent
where
    T: Into<ChildUpdate>,
{
    fn from(children: Vec<T>) -> Self {
        NodeContent::Group {
            new_children: Some(children.into_iter().map(Into::into).collect()),
        }
    }
}

/// Each group update's child.
#[derive(Debug, Deserialize, Serialize)]
pub enum ChildUpdate {
    /// Keeps an existing node by its index.
    KeepIndex(u32),

    /// Creates a node with new contents. The index for this allocated node is
    /// returned in [UpdateResponse::new_nodes].
    NewNode(NewNode),
}

impl From<NewNode> for ChildUpdate {
    fn from(new_node: NewNode) -> Self {
        ChildUpdate::NewNode(new_node)
    }
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct NodeUpdateResponse {
    pub new_nodes: Vec<u32>,
}

/// The initial contents of a new node in the tree.
#[derive(Debug, Deserialize, Serialize)]
pub enum NewNode {
    /// A [Shape].
    Shape(Shape),

    /// An [Operation] node with the given child.
    Operation {
        operation: Operation,
        child: Box<NewNode>,
    },

    /// A group node.
    Group { children: Vec<NewNode> },
}

/// A shape tree node with zero children that draws original content.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum Shape {
    /// The non-shape. Draws nothing.
    Empty,

    /// A circle with a given radius.
    Circle { radius: f32 },

    /// A rectangle with minimum and maximum bounds.
    Rectangle { min: Vec2, max: Vec2 },
}

/// A shape tree node with one child that applies a graphical operation to that
/// child.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum Operation {
    /// A stroke to apply to all children.
    Stroke(Stroke),

    /// A translation transformation.
    Translate { offset: Vec2 },

    /// A rotation transformation.
    Rotation { angle: f32 },

    /// A scale transformation.
    Scale { scale: f32 },

    /// Places the child in an opacity group with the given opacity.
    ///
    /// Note that this is applied to all children of this operation AFTER they
    /// are drawn, and not independently for each child.
    Opacity { opacity: f32 },
}

/// A stroke to apply to a [Operation::Stroke] operation.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum Stroke {
    /// A solid stroke with a given color.
    Solid { color: Vec3A },
}
