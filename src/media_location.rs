use iced::widget::{button, column, container, row, scrollable, text, Column};
use iced::Length::Fill;
use iced::{Alignment, Border, Element, Theme};
use serde::{Deserialize, Serialize};
use std::ops::Not;
use std::path::{Path, PathBuf};

use crate::media_location::MediaPathError::*;
use crate::Message;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaLocationInfo {
    name: String,
    path: PathBuf,
    #[serde(skip)]
    dropdown_opened: bool,
    #[serde(skip)]
    items: MediaLocationItems,
}

#[derive(Clone, Debug)]
pub enum MediaLocationItems {
    Unscanned,
    Scanning,
    Scanned(Scanned),
    Error(String),
}

impl Default for MediaLocationItems {
    fn default() -> Self { MediaLocationItems::Unscanned }
}

#[derive(Clone, Debug)]
pub struct Scanned {
    pub number: usize,
}

impl Scanned {
    pub fn new(number: usize) -> Self {
        Scanned { number }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MediaPathMessage {
    Remove, // Remove path
    ExpandAccordion,
    CollapseAccordion,
    ToggleAccordion,
    Scan,
    ScanAll,
}

impl MediaLocationInfo {
    // TODO: Somehow let this assume ownership of the parameters
    pub fn new(name: String, location: String) -> Result<MediaLocationInfo, MediaPathError> {
        match Path::new(&location).canonicalize() {
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
                                    items: MediaLocationItems::Unscanned,
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
        }
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
                .align_y(Alignment::Center)
                .spacing(4)
            ]
            .padding(4)
            .align_y(Alignment::Center),
        )
        .into()
    }

    fn view_media(&self) -> Element<MediaPathMessage> {
        let scanned_status = match &self.items {
            MediaLocationItems::Unscanned => text("Unscanned"),
            MediaLocationItems::Scanning => text("Scanning"),
            MediaLocationItems::Scanned(scanned) => text!("Number of Children: {}", scanned.number),
            MediaLocationItems::Error(err) => text!("Error: {}", err),
        };
        self.view_as_accordion(
            text(self.name.to_string()).size(25).width(Fill).into(),
            column![scanned_status,text("Option1"), text("Option2")].into(),
        )
    }

    fn view_as_accordion<'a>(
        &self,
        header: Element<'a, MediaPathMessage>,
        body: Element<'a, MediaPathMessage>,
    ) -> Element<'a, MediaPathMessage> {
        let header = row![
            header,
            button("Toggle").on_press(MediaPathMessage::ToggleAccordion)
        ]
        .align_y(Alignment::Center);
        let wrapper = if self.dropdown_opened {
            container(column![header, body].spacing(4))
        } else {
            container(header)
        };

        wrapper
            .padding(4)
            .width(Fill)
            .style(|theme: &Theme| {
                let palette = theme.extended_palette();

                container::Style::default().background(palette.background.weak.color)
                //TODO: Implement a stylesheet to round the corner of the container
            })
            .into()
    }

    fn scan(&mut self) {
        match self.path.read_dir() {
            Ok(dir) => {
                self.items = MediaLocationItems::Scanned(Scanned::new(dir.count()));
            }
            Err(err) => self.items = MediaLocationItems::Error(err.to_string())
        }
    }

}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MediaPathList {
    list: Vec<MediaLocationInfo>,
}

impl MediaPathList {

    fn get_mut(&mut self, index: usize) -> &mut MediaLocationInfo {
        &mut self.list[index]
    }
    pub fn push(&mut self, path: MediaLocationInfo) {
        self.list.push(path)
    }

    pub fn view_headers(&self) -> Element<Message> {
        if self.list.is_empty().not() {
            container(
                Column::with_children(self.list.iter().enumerate().map(|(i, path)| {
                    path.view_header()
                        .map(move |message| Message::MediaPathMessage(i, message))
                }))
                .spacing(10),
            )
            .style(|theme: &Theme| {
                let palette = theme.extended_palette();

                container::Style::default().border(Border::default().color(palette.background.strong.color).width(1))
            })
            .into()
        } else {
            container(column!(text("No paths...").size(25)).height(200))
        }
        .padding(20)
        .into()
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
        self.get_mut(index).dropdown_opened = true;
    }

    pub fn collapse_accordion(&mut self, index: usize) {
        self.get_mut(index).dropdown_opened = false;
    }

    pub async fn scan(&mut self, index: usize) {
        self.get_mut(index).scan()
    }

    pub async fn scan_all(mut self) -> Self {
        self.list.iter_mut().for_each(|location| location.scan());
        self

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
