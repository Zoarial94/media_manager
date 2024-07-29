use std::ops::Not;
use std::path::{Path, PathBuf};

use crate::media_location::MediaPathError::*;
use crate::{media_location, Message};
use iced::widget::{button, column, container, row, scrollable, text, Column, Row};
use iced::Length::Fill;
use iced::{Alignment, Element, Theme};
use iced_aw::DropDown;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaLocationInfo {
    name: String,
    path: PathBuf,
    #[serde(skip)]
    dropdown_opened: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum MediaPathMessage {
    Remove, // Remove path
    ExpandAccordion,
    CollapseAccordion,
    ToggleAccordion,
}

impl MediaLocationInfo {
    // TODO: Somehow let this assume ownership of the parameters
    pub fn new(name: String, location: String) -> Result<MediaLocationInfo, MediaPathError> {
        return match Path::new(&location).canonicalize() {
            Ok(path) => {
                match path.try_exists() {
                    // Returns true, false, and Err (Err means cannot be determined due to permissions)
                    Ok(b) => {
                        if b {
                            if path.is_dir() {
                                Ok(MediaLocationInfo {
                                    name,
                                    path,
                                    dropdown_opened: false,
                                })
                            } else {
                                Err(NotADirectory)
                            }
                        } else {
                            Err(PathDoesNotExist)
                        }
                    }
                    Err(_err) => Err(NoPermission),
                }
            }
            Err(err) => {
                eprintln!("{}", err);
                Err(InvalidPath)
            }
        };
    }

    fn view_header(&self) -> Element<MediaPathMessage> {
        container(
            row![
                column![
                    text(self.name.to_string()).size(25),
                    text(self.path.to_str().unwrap_or("Error")).size(15),
                ]
                .spacing(5)
                .width(Fill),
                row![
                    button("Edit"),
                    button("Remove").on_press(MediaPathMessage::Remove)
                ]
                .align_items(Alignment::Center)
                .spacing(4)
            ]
            .padding(4)
            .align_items(Alignment::Center),
        )
        .into()
    }

    fn view_media(&self) -> Element<MediaPathMessage> {
        container(
            DropDown::new(
                row![
                    text(self.name.to_string()).size(25).width(Fill),
                    button("Toggle").on_press(MediaPathMessage::ToggleAccordion)
                ]
                .align_items(Alignment::Center),
                column![text("Option1"), text("Option2")],
                self.dropdown_opened,
            )
            .width(Fill),
        )
        .padding(4)
        .style(|theme: &Theme| {
            let palette = theme.extended_palette();

            container::Appearance::default().with_background(palette.background.weak.color)
            //TODO: Implement a stylesheet to round the corner of the container
        })
        .width(Fill)
        .into()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MediaPathList {
    list: Vec<MediaLocationInfo>,
}

impl MediaPathList {
    pub fn push(&mut self, path: MediaLocationInfo) {
        self.list.push(path)
    }

    pub fn view_headers(&self) -> Element<Message> {
        return if self.list.is_empty().not() {
            container(
                Column::with_children(self.list.iter().enumerate().map(|(i, path)| {
                    path.view_header()
                        .map(move |message| Message::MediaPathMessage(i, message))
                }))
                .spacing(10),
            )
            .style(|theme: &Theme| {
                let palette = theme.extended_palette();

                container::Appearance::default().with_border(palette.background.strong.color, 1)
            })
            .into()
        } else {
            container(column!(text("No paths...").size(25)).height(200))
        }
        .padding(20)
        .into();
    }

    pub fn view_media(&self) -> Element<Message> {
        scrollable(
            Column::with_children(self.list.iter().enumerate().map(|(i, path)| {
                path.view_media()
                    .map(move |message| Message::MediaPathMessage(i, message))
            }))
            .spacing(10),
        )
        .into()
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.list.len() {
            self.list.remove(index);
        } else {
            eprintln!("Tried to remove MediaPath out of bounds");
        }
    }

    pub fn toggle_accordion(&mut self, index: usize) {
        let location_info = self.list.get_mut(index).expect("Invalid Index!");
        location_info.dropdown_opened = !location_info.dropdown_opened;
    }

    pub fn expand_accordion(&mut self, index: usize) {
        self.list.get_mut(index).expect("Invalid Index!").dropdown_opened = true;
    }

    pub fn collapse_accordion(&mut self, index: usize) {
        self.list.get_mut(index).expect("Invalid Index!").dropdown_opened = false;
    }

}

#[derive(Debug, Clone, Copy, Default)]
pub enum MediaPathError {
    #[default]
    NoError,
    InvalidPath,
    PathDoesNotExist,
    NoPermission,
    NotADirectory,
}
