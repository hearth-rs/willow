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

use chrono::{DateTime, Utc};
use glam::{vec2, Vec2, Vec4};
use willow_react::{stroke_color, Element, ElementComponent, Hooks};
use willow_server::*;

#[derive(Debug, Clone)]
pub struct MessageContent {
    pub text: String,
    pub sender: String,
    pub timestamp: DateTime<Utc>,
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
                Shape::RoundedRectangle {
                    min: Vec2::ZERO,
                    max: self.size,
                    radii: Vec4::splat(5.0),
                },
            ),
            Element::operation(
                Operation::Translate {
                    offset: vec2(5.0, 15.0),
                },
                vec![
                    Element::operation(
                        stroke_color(theme.muted),
                        Shape::Text {
                            content: self.content.timestamp.format("%d/%m/%Y %H:%M").to_string(),
                            font: "unused".to_string(),
                        },
                    ),
                    Element::operation(
                        Operation::Translate {
                            offset: vec2(100.0, 0.0),
                        },
                        vec![
                            Element::operation(
                                stroke_color(theme.text),
                                Shape::Text {
                                    content: self.content.sender.clone(),
                                    font: "unused".to_string(),
                                },
                            ),
                            Element::operation(
                                Operation::Translate {
                                    offset: vec2(100.0, 0.0),
                                },
                                Element::operation(
                                    stroke_color(theme.text),
                                    Shape::Text {
                                        content: self.content.text.clone(),
                                        font: "unused".to_string(),
                                    },
                                ),
                            ),
                        ],
                    ),
                ],
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
            messages,
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
                    offset: Vec2::new(0.0, height)
                        + (-padding - border - text_anchor) * Vec2::new(-1.0, 1.0),
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
        let theme = hooks.use_theme();

        vec![
            Element::operation(
                stroke_color(theme.base),
                Shape::Rectangle {
                    min: Vec2::ZERO,
                    max: Vec2::new(self.size.x, self.size.y - TextPrompt::HEIGHT),
                },
            ),
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
            ),
        ]
        .into()
    }
}
