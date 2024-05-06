use std::mem;
use std::ops::Not;
use std::path::{Path, PathBuf};
use iced::{Alignment, Application, Command, Element, keyboard, Settings, Subscription, Theme, widget};

use once_cell::sync::Lazy;
use iced::widget::{button, column, Column, container, row, text, text_input};
use crate::MediaPathError::{InvalidPath, NoError, NotADirectory};


static MEDIA_LOCATION_INPUT_ID: Lazy<text_input::Id> = Lazy::new(|| text_input::Id::new("Media Location"));
static MEDIA_LOCATION_NAME_INPUT_ID: Lazy<text_input::Id> = Lazy::new(|| text_input::Id::new("Media Location Name"));

fn main() {
    println!("Hello, world!");
    MediaManager::run(Settings::default()).expect("TODO: panic message");
}


#[derive(Debug, Clone)]
pub struct MediaLocationInfo {
    name: String,
    path: PathBuf,
}


impl MediaLocationInfo {

    // TODO: Somehow let this assume ownership of the parameters
    fn new(name: String, location: String) -> Result<MediaLocationInfo, MediaPathError> {
        return match Path::new(&location).canonicalize() {
            Ok(path) => {
                match path.try_exists() {
                    Ok(b) => {
                        if b {
                            Ok(MediaLocationInfo{name, path})
                        } else {
                            Err(NotADirectory)
                        }
                    },
                    Err(err) => {
                        Err(MediaPathError::NoPermission)
                    }
                }

            },
            Err(err) => {
                eprintln!("{}", err);
                Err(InvalidPath)
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
struct MediaPathList {
    list: Vec<MediaLocationInfo>,
}

#[derive(Debug, Clone, Copy)]
pub enum MediaLocationMessage {
    Add,
    Remove,
}

#[derive(Debug, Clone, Copy, Default)]
enum MediaPathError {
    #[default]
    NoError,
    InvalidPath,
    PathDoesNotExist,
    NoPermission,
    NotADirectory,
}

#[derive(Debug, Clone)]
enum Message {
    AddMediaLocation,
    RemoveMediaLocation,
    MediaPathMessage(usize, MediaLocationMessage),
    MediaLocationInputChanged(String),
    MediaLocationNameInputChanged(String),
    FocusTextID(text_input::Id),

    TabPressed { shift: bool }

}


#[derive(Debug, Default)]
struct State {
    media_path_list: MediaPathList,
    media_location: String,
    media_location_name: String,
    media_path_error: MediaPathError
}

#[derive(Debug)]
enum MediaManager {
    Loaded(State)
}

impl Application for MediaManager {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_: Self::Flags) -> (MediaManager, Command<Message>) {
        (MediaManager::Loaded(State::default()), Command::none())
    }


    fn title(&self) -> String {
        String::from("Media Manager")
    }

    fn update(&mut self, message: Self::Message) -> Command<Message> {
        match self {
            MediaManager::Loaded(state) => {
                match message {
                    Message::MediaLocationInputChanged(new_text) => {
                        state.media_location = new_text;
                        Command::none()
                    },
                    Message::MediaLocationNameInputChanged(new_text) => {
                        state.media_location_name = new_text;
                        Command::none()
                    },
                    Message::AddMediaLocation => {
                        match MediaLocationInfo::new(state.media_location_name.clone(), state.media_location.clone()) {
                            Ok(locationInfo) => {
                                state.media_path_list.list.push(locationInfo);
                                state.media_location.clear();
                                state.media_location_name.clear();
                                text_input::focus(MEDIA_LOCATION_NAME_INPUT_ID.clone())
                            },
                            Err(err) => {
                                eprintln!("{:?}", err);
                                state.media_path_error = InvalidPath;
                                return Command::none()
                            }
                        }
                    },
                    Message::FocusTextID(id) => {
                        text_input::focus(id)
                    }
                    Message::TabPressed { shift } => {
                        if shift {
                            widget::focus_previous()
                        } else {
                            widget::focus_next()
                        }
                    }
                    _ => {
                        Command::none()
                    }
                }
            }
        }
    }

    fn view(&self) -> Element<Self::Message> {

        match self {
            MediaManager::Loaded(state) => {
                // Get a view of the currently saved paths
                let paths = container(if state.media_path_list.list.is_empty().not() {
                    Column::with_children(state.media_path_list.list.iter().enumerate().map(|(i, path)| {
                        path.view().map(move |message| { Message::MediaPathMessage(i, message)})
                    })).spacing(10)
                } else {
                    column!(text("No paths...").size(25))
                        .height(200)
                }).padding(20);

                let path_info_valid = state.media_location.starts_with('/');
                let button_action = if path_info_valid {
                    Some(Message::AddMediaLocation)
                } else {
                    None
                };

                let rows = row![
                    // We use a column: a simple vertical layout
                    column![

                        text("Media Location Info"),
                        text_input("SD Card", &state.media_location_name)
                            .width(440)
                            .padding(10)
                            .on_input(Message::MediaLocationNameInputChanged)
                            .on_submit(Message::FocusTextID(MEDIA_LOCATION_INPUT_ID.clone()))
                            .id(MEDIA_LOCATION_NAME_INPUT_ID.clone()),
                        text_input("/media/...", &state.media_location)
                            .width(440)
                            .padding(10)
                            .on_input(Message::MediaLocationInputChanged)
                            .on_submit(Message::AddMediaLocation)
                            .id(MEDIA_LOCATION_INPUT_ID.clone()),
                        // The increment button. We tell it to produce an
                        // `Increment` message when pressed
                        button("Add")
                            .on_press_maybe(button_action)
                            .width(120),

                        // We show the value of the counter here
                        text(String::from("Placeholder!")).size(50),


                        // The decrement button. We tell it to produce a
                        // `Decrement` message when pressed
                        //button("Remove").on_press(Message::Remove),
                    ] // column![]
                        .spacing(10)
                ] //row![]
                    .padding(20)
                    .align_items(Alignment::Start);

                container(row![rows,paths]).into()
            }
        }

    }

    fn subscription(&self) -> Subscription<Message> {
        use iced::keyboard::key;

        keyboard::on_key_press(|key, modifiers| {
            let keyboard::Key::Named(key) = key else {
                return None;
            };

            match (key, modifiers) {
                (key::Named::Tab, _) => Some(Message::TabPressed {
                    shift: modifiers.shift(),
                }),
                _ => None,
            }
        })
    }

}

impl MediaLocationInfo {

    fn view(&self) -> Element<MediaLocationMessage> {
        row![
            column![
                text(self.name.to_string()).size(25),
                text(self.path.to_str().unwrap_or("Error")).size(15),
            ].width(400).spacing(5),
            button("Remove").on_press(MediaLocationMessage::Remove)
        ].align_items(Alignment::Center).into()
    }

}