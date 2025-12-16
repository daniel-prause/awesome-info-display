use exchange_format::ConfigParam;
use iced::{Length, Theme};
use indexmap::IndexMap;

use crate::{
    config_manager::ConfigManager, rgb_bytes_to_rgba_image, swap_rgb, Message, DEVICES, ICONS,
};

use super::text_manipulation::{determine_field_value, humanize_string};

pub fn special_checkbox<'a>(
    checked: bool,
    key: String,
    description: String,
) -> iced::Element<'a, Message, Theme, iced::Renderer> {
    iced::widget::checkbox(checked)
        .style(|_theme, status| crate::style::checkbox_style(status))
        .on_toggle(move |value: bool| Message::ScreenStatusChanged(value, key.clone()))
        .label(description)
        .width(Length::Fixed(200f32))
        .into()
}

pub fn device_connected_icon<'a>(
    is_connected: bool,
) -> iced::Element<'a, Message, Theme, iced::Renderer> {
    iced::widget::text(if is_connected {
        String::from("\u{f26c} \u{f058}")
    } else {
        String::from("\u{f26c} \u{f057}")
    })
    .font(ICONS)
    .shaping(iced::widget::text::Shaping::Advanced)
    .into()
}

pub fn device_status<'a>(device: &str) -> Vec<iced::Element<'a, Message, Theme, iced::Renderer>> {
    vec![
        iced::widget::Text::new(device.to_uppercase())
            .width(Length::Fixed(146f32))
            .font(iced::Font::MONOSPACE)
            .into(),
        device_connected_icon(DEVICES.get(device).unwrap().is_connected()),
    ]
}

pub fn push_config_fields<'a>(
    config_manager: std::sync::Arc<std::sync::RwLock<ConfigManager>>,
    config_params: IndexMap<String, ConfigParam>,
    screen_key: String,
) -> Vec<iced::Element<'a, Message, Theme, iced::Renderer>> {
    let mut config_column_parts: Vec<iced::Element<Message, Theme, iced::Renderer>> = vec![];

    for (key, item) in config_params {
        let screen_key = screen_key.to_string();
        let config_param_option = config_manager
            .read()
            .unwrap()
            .get_value(&screen_key, key.as_str());

        match item {
            ConfigParam::String(value) => {
                let field_value = match config_param_option {
                    Some(ConfigParam::String(saved_value)) => saved_value.clone(),
                    _ => value.to_string(),
                };

                config_column_parts.push(
                    iced::widget::TextInput::new(
                        humanize_string(&key).as_str(),
                        field_value.as_str(),
                    )
                    .on_input(move |value: String| {
                        Message::ConfigValueChanged(
                            screen_key.clone(),
                            key.clone(),
                            exchange_format::ConfigParam::String(value),
                        )
                    })
                    .style(|_theme, _status| crate::style::text_field())
                    .width(Length::Fixed(200f32))
                    .into(),
                );
            }
            ConfigParam::Integer(value) => {
                let field_value = match config_param_option {
                    Some(ConfigParam::Integer(saved_value)) => saved_value,
                    _ => value,
                };

                config_column_parts.push(
                    iced::widget::TextInput::new(
                        humanize_string(&key).as_str(),
                        determine_field_value(&field_value.to_string().as_str()).as_str(),
                    )
                    .on_input(move |value: String| {
                        Message::ConfigValueChanged(
                            screen_key.clone(),
                            key.clone(),
                            exchange_format::ConfigParam::Integer({
                                let val: String = value
                                    .to_string()
                                    .chars()
                                    .filter(|c| c.is_numeric())
                                    .collect();

                                val.parse().unwrap_or(0)
                            }),
                        )
                    })
                    .style(|_theme, _status| crate::style::text_field())
                    .width(Length::Fixed(200f32))
                    .into(),
                );
            }
            ConfigParam::Password(value) => {
                let field_value = match config_param_option {
                    Some(ConfigParam::Password(saved_value)) => saved_value.clone(),
                    _ => value.to_string(),
                };

                config_column_parts.push(
                    iced::widget::TextInput::new(
                        humanize_string(&key).as_str(),
                        field_value.as_str(),
                    )
                    .on_input(move |value: String| {
                        Message::ConfigValueChanged(
                            screen_key.clone(),
                            key.clone(),
                            exchange_format::ConfigParam::Password(value),
                        )
                    })
                    .style(|_theme, _status| crate::style::text_field())
                    .secure(true)
                    .width(Length::Fixed(200f32))
                    .into(),
                );
            }
            _ => {} // TODO: handle ConfigParam::Float(value) or other types if needed
        }
    }

    config_column_parts
}

pub fn preview_images(
    screen_bytes: IndexMap<&'_ str, Vec<u8>>,
) -> Vec<iced::Element<'_, Message, Theme, iced::Renderer>> {
    let mut preview_images: Vec<iced::Element<Message, Theme, iced::Renderer>> = vec![];
    for key in DEVICES.keys() {
        let screen_width = DEVICES.get(key).unwrap().screen_width();
        let screen_height = DEVICES.get(key).unwrap().screen_height();

        let preview_image = rgb_bytes_to_rgba_image(
            &swap_rgb(
                &screen_bytes.get(key.as_str()).unwrap(),
                screen_width,
                screen_height,
            ),
            screen_width,
            screen_height,
        )
        .width(Length::Fixed(screen_width as f32))
        .height(Length::Fixed(screen_height as f32));
        preview_images.push(preview_image.into());
    }
    return preview_images;
}
