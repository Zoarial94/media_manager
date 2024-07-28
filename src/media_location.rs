use std::ops::Not;
use std::path::{Path, PathBuf};

use iced::{Alignment, Element, Theme};
use iced::Length::Fill;
use iced::widget::{button, column, Column, container, row, text};
use serde::{Deserialize, Serialize};
use crate::{MediaPathMessage, Message};
use crate::media_location::MediaPathError::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaLocationInfo {
    name: String,
    path: PathBuf,
}


impl MediaLocationInfo {

    // TODO: Somehow let this assume ownership of the parameters
    pub fn new(name: String, location: String) -> Result<MediaLocationInfo, MediaPathError> {
        return match Path::new(&location).canonicalize() {
            Ok(path) => {
                match path.try_exists() { // Returns true, false, and Err (Err means cannot be determined due to permissions)
                    Ok(b) => {
                        if b {
                            if path.is_dir() {
                                Ok(MediaLocationInfo { name, path })
                            } else {
                                Err(NotADirectory)
                            }
                        } else {
                            Err(PathDoesNotExist)
                        }
                    },
                    Err(_err) => {
                        Err(NoPermission)
                    }
                }

            },
            Err(err) => {
                eprintln!("{}", err);
                Err(InvalidPath)
            }
        }
    }

    fn view(&self) -> Element<MediaPathMessage> {
        container(
            row![
                column![
                    text(self.name.to_string()).size(25),
                    text(self.path.to_str().unwrap_or("Error")).size(15),
                ].spacing(5).width(Fill),
                button("Remove").on_press(MediaPathMessage::Remove)
                ].padding(10).align_items(Alignment::Center)
        ).into()
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

    pub fn view(&self) -> Element<Message> {
        return if self.list.is_empty().not() {
            container(Column::with_children(self.list.iter().enumerate().map(|(i, path)| {
                path.view().map(move |message| { Message::MediaPathMessage(i, message)})
            })).spacing(10)).style(|theme: &Theme| {
                let palette = theme.extended_palette();

                container::Appearance::default()
                    .with_border(palette.background.strong.color, 1)
            }).into()
        } else {
            container(column!(text("No paths...").size(25))
                .height(200))
        }.padding(20).into()

    }

    pub fn remove(&mut self, index: usize) {
        if index < self.list.len() {
            self.list.remove(index);
        } else {
            eprintln!("Tried to remove MediaPath out of bounds");
        }
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
