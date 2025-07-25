mod components {
    pub(crate) mod media_location;
}
mod persistence;

use crate::components::media_location::*;
use crate::persistence::*;
use iced::widget::{button, column, container, row, text, text_input};
use iced::{keyboard, widget, Alignment, Element, Pixels, Subscription, Task};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::mem;

static MEDIA_LOCATION_INPUT_ID: Lazy<text_input::Id> =
    Lazy::new(|| text_input::Id::new("Media Location"));
static MEDIA_LOCATION_NAME_INPUT_ID: Lazy<text_input::Id> =
    Lazy::new(|| text_input::Id::new("Media Location Name"));

fn main() {
    println!("Hello, world!");
    iced::application("Media Manager", MediaManager::update, MediaManager::view)
        .subscription(MediaManager::subscription)
        .run_with(|| MediaManager::new(()))
        .expect("TODO: panic message");
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub(crate) struct State {
    #[serde(skip)]
    pub(crate) saving: bool,
    #[serde(skip)]
    pub(crate) save_state_changed: bool,
    pub(crate) media_path_list: Box<MediaPathList>,
    pub(crate) media_location: String,
    pub(crate) media_location_name: String,
    #[serde(skip)]
    pub(crate) media_path_error: MediaPathError,
}

#[derive(Debug, Clone)]
enum Message {
    LoadState,
    StateLoaded(Result<State, LoadError>),
    StateSaved(Result<(), SaveError>),
    // Media Path
    AddMediaPath,
    MediaPathMessage(usize, MediaPathMessage), //TODO: made MediaPathMessage a reference (Lifetime needed)

    MediaPathsScanned(Box<MediaPathList>),

    MediaLocationInputChanged(String),
    MediaLocationNameInputChanged(String),

    FocusTextID(text_input::Id),
    TabPressed { shift: bool },
}

#[derive(Debug)]
enum MediaManager {
    Loading(),
    Loaded(State),
}

impl MediaManager {

    fn new(_flags: ()) -> (Self, Task<Message>) {
        (
            MediaManager::Loading(),
            Task::perform(async {}, |_| Message::LoadState),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match self {
            MediaManager::Loaded(state) => {
                let task = match message {
                    Message::MediaLocationInputChanged(new_text) => {
                        state.media_location = new_text;
                        None
                    }
                    Message::MediaLocationNameInputChanged(new_text) => {
                        state.media_location_name = new_text;
                        None
                    }
                    Message::AddMediaPath => {
                        match MediaLocationInfo::new(
                            state.media_location_name.clone(),
                            state.media_location.clone(),
                        ) {
                            Ok(location_info) => {
                                state.media_path_list.push(location_info);
                                state.media_location.clear();
                                state.media_location_name.clear();
                                state.media_path_error = MediaPathError::NoError;
                                state.save_state_changed = true;
                                Some(text_input::focus(MEDIA_LOCATION_NAME_INPUT_ID.clone()))
                            }
                            Err(err) => {
                                eprintln!("Media error: {:?}", err);
                                state.media_path_error = err;
                                None
                            }
                        }
                    }
                    Message::FocusTextID(id) => Some(text_input::focus(id)),
                    Message::TabPressed { shift } => {
                        if shift {
                            Some(widget::focus_previous())
                        } else {
                            Some(widget::focus_next())
                        }
                    }
                    Message::MediaPathMessage(index, message) => {
                        match message {
                            MediaPathMessage::Remove => {
                                state.media_path_list.remove(index);
                                state.save_state_changed = true;
                                None
                            }
                            MediaPathMessage::ExpandAccordion => {
                                state.media_path_list.expand_accordion(index);
                                None
                            }
                            MediaPathMessage::CollapseAccordion => {
                                state.media_path_list.collapse_accordion(index);
                                None
                            }
                            MediaPathMessage::ToggleAccordion => {
                                state.media_path_list.toggle_accordion(index);
                                None
                            }
                            MediaPathMessage::Scan => {
                                //Some(Task::perform(state.media_path_list.clone().scan(index), |_| Message::MediaPathsScanned()))
                                todo!();
                                None
                            }
                            MediaPathMessage::ScanAll => {
                                let list = mem::replace(&mut state.media_path_list, Box::new(Default::default()));
                                Some(Task::perform(list.scan_all(), |list: MediaPathList| Message::MediaPathsScanned(Box::from(list))))
                            }
                        }
                    }
                    Message::MediaPathsScanned(list) => {
                        state.media_path_list = list;
                        None
                    }
                    Message::StateSaved(result) => {
                        state.saving = false;
                        match result {
                            Err(e) => {
                                eprintln!("Saving Error: {:?}", e);
                            }
                            Ok(_) => {
                                println!("Saved state!")
                            }
                        }
                        None
                    }
                    _ => None,
                };

                match (task, state.saving, state.save_state_changed) {
                    (None, false, true) => { // Initiate saving
                        state.saving = true;
                        state.save_state_changed = false;
                        Task::perform(state.clone().save(), Message::StateSaved)
                    }
                    (Some(task), false, true) => { // Initiate saving and do task
                        state.saving = true;
                        state.save_state_changed = false;
                        Task::batch(vec![
                            task,
                            Task::perform(state.clone().save(), Message::StateSaved),
                        ])
                    }
                    (Some(task), _, false) => task, // Do task
                    _ => Task::none(), // Do nothing
                }
            }
            MediaManager::Loading() => {
                match message {
                    Message::LoadState => Task::perform(State::load(), Message::StateLoaded),
                    Message::StateLoaded(restored_state) => {
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
                        Task::none()
                    }
                    _ => Task::none(),
                }
            }
        }
    }

    fn view(&self) -> Element<Message> {
        match self {
            MediaManager::Loaded(state) => {
                // Get a view of the currently saved paths
                let paths_view = container(state.media_path_list.view_headers());
                let media_view = container(state.media_path_list.view_media());
                let button_action = if state.media_location.len() > 0 {
                    Some(Message::AddMediaPath)
                } else {
                    None
                };

                let err_text = match state.media_path_error {
                    MediaPathError::NoError => "",
                    MediaPathError::InvalidPath => "Invalid path",
                    MediaPathError::PathDoesNotExist => "Path does not exist",
                    MediaPathError::NoPermission => "No permission",
                    MediaPathError::NotADirectory => "Not a directory",
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
                    button("Add").on_press_maybe(button_action).width(120),
                    // We show the value of the counter here
                    text(String::from(err_text)).size(50),
                    // The decrement button. We tell it to produce a
                    // `Decrement` message when pressed
                    //button("Remove").on_press(Message::Remove),
                ] // column![]
                .spacing(10)
                .padding(20)
                .align_x(Alignment::Start);

                //let sidebar_size = if add_media_path_view.size().width

                row!(
                    column![add_media_path_view, paths_view, button("Scan").on_press(Message::MediaPathMessage(0, MediaPathMessage::ScanAll)).width(120)]
                        .width(iced::Length::FillPortion(1).enclose(Pixels(80.0).into())),
                    container(media_view).width(iced::Length::FillPortion(2))
                )
                .into()
            }
            _ => container(text("Loading...")).into(),
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
