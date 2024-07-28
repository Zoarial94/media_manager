mod persistence;

use iced::{Alignment, Application, Command, Element, keyboard, Pixels, Settings, Subscription, Theme, widget};
use iced::widget::{button, column, container, row, text_input, text, Text};
use once_cell::sync::Lazy;
use serde::Serialize;
use media_info::*;
use crate::persistence::media_info::{LoadError, SaveError};

static MEDIA_LOCATION_INPUT_ID: Lazy<text_input::Id> = Lazy::new(|| text_input::Id::new("Media Location"));
static MEDIA_LOCATION_NAME_INPUT_ID: Lazy<text_input::Id> = Lazy::new(|| text_input::Id::new("Media Location Name"));

fn main() {
    println!("Hello, world!");
    MediaManager::run(Settings::default()).expect("TODO: panic message");
}

mod media_info {
    use std::ops::Not;
    use std::path::{Path, PathBuf};

    use iced::{Alignment, Element, Theme};
    use iced::Length::Fill;
    use iced::widget::{button, column, Column, container, Container, row, text};
    use serde::{Deserialize, Serialize};
    use crate::{MediaPathMessage, Message};
    use crate::media_info::MediaPathError::*;

    #[derive(Debug, Default, Clone, Serialize, Deserialize)]
    pub(super) struct State {
        pub(crate) media_path_list: MediaPathList,
        pub(crate) media_location: String,
        pub(crate) media_location_name: String,
        #[serde(skip)]
        pub(crate) media_path_error: MediaPathError
    }

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

}

#[derive(Debug, Clone, Copy)]
pub enum MediaPathMessage {
    Remove,
}

#[derive(Debug, Clone)]
enum Message {
    Loaded(Result<State, LoadError>),
    Saved(Result<(), SaveError>),
    // Media Path
    AddMediaPath,
    MediaPathMessage(usize, MediaPathMessage), //TODO: made MediaPathMessage a reference (Lifetime needed)


    MediaLocationInputChanged(String),
    MediaLocationNameInputChanged(String),

    FocusTextID(text_input::Id),
    TabPressed { shift: bool },

}


#[derive(Debug)]
enum MediaManager {
    Loading(),
    Loaded(State)
}

impl Application for MediaManager {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_: Self::Flags) -> (MediaManager, Command<Message>) {
        (MediaManager::Loading(), Command::perform(State::load(), Message::Loaded))
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
                    Message::AddMediaPath => {
                        match MediaLocationInfo::new(state.media_location_name.clone(), state.media_location.clone()) {
                            Ok(location_info) => {
                                state.media_path_list.push(location_info);
                                state.media_location.clear();
                                state.media_location_name.clear();
                                state.media_path_error = MediaPathError::NoError;
                                Command::batch(vec![
                                    text_input::focus(MEDIA_LOCATION_NAME_INPUT_ID.clone()),
                                    //TODO: Potentially optimize. This might be an expensive clone.
                                    Command::perform(state.clone().save(), Message::Saved)
                                ])
                            }
                            Err(err) => {
                                eprintln!("Media error: {:?}", err);
                                state.media_path_error = err;
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
                    Message::MediaPathMessage(index, message) => {
                        match message {
                            MediaPathMessage::Remove => {
                                state.media_path_list.remove(index)
                            }
                        }
                        Command::none()
                    }

                    _ => {Command::none()}
                }
            }
            MediaManager::Loading() => {
                return match message {
                    Message::Loaded(restored_state) => {
                        match restored_state {
                            Ok(state) => {
                                println!("State successfully loaded.");
                                *self = MediaManager::Loaded(state);
                            }
                            Err(e) => {
                                eprintln!("Failed to restore state: {:?}", e);
                                *self = MediaManager::Loaded(State::default());
                            }
                        }
                        Command::none()
                    }
                    _ => {Command::none()}
                }
            }
        }
    }

    fn view(&self) -> Element<Self::Message> {

        match self {
            MediaManager::Loaded(state) => {
                // Get a view of the currently saved paths
                let paths_view = container(state.media_path_list.view());
                let path_info_valid = state.media_location.starts_with('/');
                let button_action = if path_info_valid {
                    Some(Message::AddMediaPath)
                } else {
                    None
                };

                let err_text = match state.media_path_error {
                    MediaPathError::NoError => {""}
                    MediaPathError::InvalidPath => {"Invalid path"}
                    MediaPathError::PathDoesNotExist => {"Path does not exist"}
                    MediaPathError::NoPermission => {"No permission"}
                    MediaPathError::NotADirectory => {"Not a directory"}
                };

                let add_media_path_view = column![

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
                            .on_submit(Message::AddMediaPath)
                            .id(MEDIA_LOCATION_INPUT_ID.clone()),
                        // The increment button. We tell it to produce an
                        // `Increment` message when pressed
                        button("Add")
                            .on_press_maybe(button_action)
                            .width(120),

                        // We show the value of the counter here
                        text(String::from(err_text)).size(50),


                        // The decrement button. We tell it to produce a
                        // `Decrement` message when pressed
                        //button("Remove").on_press(Message::Remove),
                    ] // column![]
                        .spacing(10)
                        .padding(20)
                        .align_items(Alignment::Start);

                //let sidebar_size = if add_media_path_view.size().width

                row!(
                    column![add_media_path_view,paths_view].width(iced::Length::FillPortion(2).enclose(Pixels(80.0).into())),
                    container(text("Test!")).width(iced::Length::FillPortion(3))
                ).into()
            }
            _ => {container(text("Loading...")).into()}
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
