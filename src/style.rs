use iced::Color;
#[derive(Debug, Clone, Copy)]

pub struct Checkbox {}
pub struct TextInput {}

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

impl iced::widget::checkbox::StyleSheet for Checkbox {
    type Style = iced::Theme;

    fn active(
        &self,
        _style: &Self::Style,
        _is_checked: bool,
    ) -> iced::widget::checkbox::Appearance {
        iced::widget::checkbox::Appearance {
            background: iced::Color::WHITE.into(),
            icon_color: iced::Color::BLACK,
            border_radius: 0f32,
            border_width: 1f32,
            border_color: iced::Color::BLACK,
            text_color: None,
        }
    }

    fn hovered(
        &self,
        _style: &Self::Style,
        _is_checked: bool,
    ) -> iced::widget::checkbox::Appearance {
        iced::widget::checkbox::Appearance {
            background: iced::Background::Color(iced::Color {
                a: 0.8,
                ..iced::Color::WHITE
            }),
            icon_color: iced::Color::BLACK,
            border_radius: 0f32,
            border_width: 1f32,
            border_color: iced::Color::BLACK,
            text_color: None,
        }
    }
}

impl iced::widget::text_input::StyleSheet for TextInput {
    type Style = iced::Theme;

    fn active(&self, _style: &Self::Style) -> iced::widget::text_input::Appearance {
        iced::widget::text_input::Appearance {
            background: iced::Color::WHITE.into(),
            border_radius: 0f32,
            border_width: 1f32,
            border_color: iced::Color::BLACK,
        }
    }

    fn value_color(&self, _style: &Self::Style) -> Color {
        iced::Color::BLACK.into()
    }

    /// Produces the style of a focused text input.
    fn focused(&self, _style: &Self::Style) -> iced::widget::text_input::Appearance {
        iced::widget::text_input::Appearance {
            background: iced::Color::WHITE.into(),
            border_radius: 0f32,
            border_width: 1f32,
            border_color: iced::Color::BLACK,
        }
    }

    /// Produces the [`Color`] of the placeholder of a text input.
    fn placeholder_color(&self, _style: &Self::Style) -> Color {
        iced::Color::BLACK.into()
    }

    /// Produces the [`Color`] of the selection of a text input.
    fn selection_color(&self, _style: &Self::Style) -> Color {
        let mut color = iced::Color::from(HOVERED);
        color.a = 0.5;
        color
    }
}
