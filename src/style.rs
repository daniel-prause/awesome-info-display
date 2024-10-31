use iced::{border::Radius, Border, Color};

/*
const SURFACE: Color = Color::from_rgb(
    0x40 as f32 / 255.0,
    0x44 as f32 / 255.0,
    0x4B as f32 / 255.0,
);

const ACCENT: Color = Color::from_rgb(
    0x6F as f32 / 255.0,
    0xFF as f32 / 255.0,
    0xE9 as f32 / 255.0,
);

const ACTIVE: Color = Color::from_rgb(
    0x72 as f32 / 255.0,
    0x89 as f32 / 255.0,
    0xDA as f32 / 255.0,
);
 */
const HOVERED: Color = Color::from_rgb(
    0x67 as f32 / 255.0,
    0x7B as f32 / 255.0,
    0xC4 as f32 / 255.0,
);

pub fn checkbox_style(status: iced::widget::checkbox::Status) -> iced::widget::checkbox::Style {
    match status {
        iced::widget::checkbox::Status::Hovered { is_checked: false } => return checkbox_hovered(),
        iced::widget::checkbox::Status::Hovered { is_checked: true } => return checkbox_hovered(),
        _ => return checkbox_active(),
    }
}

fn checkbox_hovered() -> iced::widget::checkbox::Style {
    iced::widget::checkbox::Style {
        background: iced::Background::Color(iced::Color {
            a: 0.8,
            ..iced::Color::WHITE
        }),
        icon_color: iced::Color::BLACK,

        border: Border {
            color: iced::Color::BLACK,
            width: 1.0,
            radius: Radius::from(0),
        },
        text_color: None,
    }
}
fn checkbox_active() -> iced::widget::checkbox::Style {
    iced::widget::checkbox::Style {
        background: iced::Color::WHITE.into(),
        icon_color: iced::Color::BLACK,
        border: Border::from(Border {
            color: iced::Color::BLACK,
            width: 1f32,
            radius: iced::border::Radius::from(0),
        }),
        text_color: None,
    }
}

pub fn text_field() -> iced::widget::text_input::Style {
    return iced::widget::text_input::Style {
        background: iced::Color::WHITE.into(),
        border: Border {
            color: iced::Color::BLACK,
            width: 1.0,
            radius: Radius::from(0),
        },
        icon: iced::Color::WHITE,
        placeholder: iced::Color::from_rgba8(0, 0, 0, 0.7),
        selection: {
            let mut color = HOVERED;
            color.a = 0.5;
            color
        },
        value: iced::Color::BLACK,
    };
}

/*
impl iced::widget::text_input::StyleSheet for TextInput {
    type Style = iced::Theme;

    fn active(&self, _style: &Self::Style) -> iced::widget::text_input::Appearance {
        iced::widget::text_input::Appearance {
            background: iced::Color::WHITE.into(),
            border: Border {
                color: iced::Color::BLACK,
                width: 1.0,
                radius: Radius::from(0),
            },
            icon_color: iced::Color::BLACK,
        }
    }

    fn value_color(&self, _style: &Self::Style) -> Color {
        iced::Color::BLACK
    }

    /// Produces the style of a focused text input.
    fn focused(&self, _style: &Self::Style) -> iced::widget::text_input::Appearance {
        iced::widget::text_input::Appearance {
            background: iced::Color::WHITE.into(),
            border: Border {
                color: iced::Color::BLACK,
                width: 1.0,
                radius: Radius::from(0),
            },
            icon_color: iced::Color::WHITE,
        }
    }

    /// Produces the [`Color`] of the placeholder of a text input.
    fn placeholder_color(&self, _style: &Self::Style) -> Color {
        iced::Color::BLACK
    }

    /// Produces the [`Color`] of the selection of a text input.
    fn selection_color(&self, _style: &Self::Style) -> Color {
        let mut color = HOVERED;
        color.a = 0.5;
        color
    }

    fn disabled_color(&self, _style: &Self::Style) -> Color {
        iced::Color::from_rgb8(235, 235, 228)
    }

    fn disabled(&self, style: &Self::Style) -> iced::widget::text_input::Appearance {
        iced::widget::text_input::Appearance {
            background: self.disabled_color(style).into(),
            border: Border {
                color: iced::Color::BLACK,
                width: 1.0,
                radius: Radius::from(0),
            },
            icon_color: iced::Color::WHITE,
        }
    }
}
 */
