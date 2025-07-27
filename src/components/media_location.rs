use crate::components::media_location::MediaPathError::*;
use crate::Message;
use async_std::fs::{DirEntry, ReadDir};
use async_std::path::PathBuf;
use async_std::sync::Mutex;
use async_std::task::yield_now;
use exiftool::ExifTool;
use iced::futures::StreamExt;
use iced::widget::{button, column, container, row, scrollable, text, Column};
use iced::Length::Fill;
use iced::{futures, Alignment, Border, Element, Theme};
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::ffi::OsString;
use std::fmt::Formatter;
use std::io;
use std::ops::Not;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaLocationInfo {
    name: String,
    #[serde(serialize_with = "serialize_path_buf", deserialize_with = "deserialize_path_buf")]
    path: PathBuf,
    #[serde(skip)]
    dropdown_opened: bool,
    #[serde(skip)]
    items: MediaLocationItems,
}

/**
Serialization and Deserialization for Serde

*/
struct PathBufVisitor;

impl<'de> Visitor<'de> for PathBufVisitor {
    type Value = PathBuf;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("a string that represents a path")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error
    {
        Ok(PathBuf::from(v))
    }
}
fn serialize_path_buf<S>(path: &PathBuf, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer
{
    s.serialize_str(path.to_str().unwrap()) // TODO: Do I need to handle invalid strings?
}

fn deserialize_path_buf<'de, D>( d: D) -> Result<PathBuf, D::Error>
where
    D: Deserializer<'de>
{
    d.deserialize_str(PathBufVisitor)
}

/**
Media Location

*/
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

/**
Scanned Data

*/
#[derive(Clone, Debug)]
pub struct Scanned {
    pub number: usize,
    pub entries: Vec<ScannedMedia>,
}

#[derive(Clone, Debug)]
pub struct ScannedMedia {
    entry: DirEntry,
    date_time_original: String,
    pub data: String,
}

impl ScannedMedia {
    pub fn file_name(&self) -> OsString {
        self.entry.file_name()
    }

    pub fn new(entry: DirEntry, exif_tool: &mut ExifTool) -> Self {
        let path = entry.path();
        let metadata = exif_tool.json(path.as_path().as_ref(), &["-AllDate"]);
        //TODO Make sure to fix this
        Self {entry, data: metadata.unwrap().to_string(), date_time_original: "Test".to_string()}
    }

    pub async fn new_batch(entries: Vec<DirEntry>, exif_tool: Arc<Mutex<ExifTool>>) -> Vec<Self> {
        let mut ret_list: Vec<Self> = Vec::new();
        let path_list: Vec<PathBuf> = entries.iter().map(|e| e.path()).collect();
        let mut exif_tool = exif_tool.lock().await;


        let mut dates_batch = exif_tool.json_batch(path_list.clone(), &["-AllDate"]).unwrap().into_iter();


        for entry in entries {
            let metadata = dates_batch.next();
                match metadata {
                Some(data) => {
                    #[cfg(debug_assertions)]
                    let data_string = data.to_string();
                    #[cfg(not(debug_assertions))]
                    let data_string = String::new();
                    println!("File: {}", entry.file_name().to_string_lossy());
                    println!("Data: {}", data);
                    let date_time_opt = data.get("DateTimeOriginal");
                    match date_time_opt {
                        Some(date_time) => {
                            ret_list.push(ScannedMedia{entry, data: data_string, date_time_original: date_time.to_string()})
                        }
                        _ => {
                            ret_list.push(ScannedMedia{entry, data: data_string, date_time_original: "No Original Date/Time".to_string()})
                        }
                    }
                }
                _ => { }
            }
        }

        ret_list
    }
}

impl Scanned {
    pub async fn new(dir: ReadDir, exif_tool: Arc<Mutex<ExifTool>> ) -> Self {
        let list: Vec<io::Result<DirEntry>> = dir.collect::<Vec<io::Result<DirEntry>>>().await;
        let number = list.len();
        let list: Vec<DirEntry> = futures::future::join_all(list.into_iter().map(async |e: io::Result<DirEntry>| {
            return match e {
                Ok(e) => {
                    if e.file_type().await.unwrap().is_file(){
                        return Some(e)
                    }
                    None
                }
                Err(_) => {
                    None
                }
            }
        })).await.into_iter().filter_map(|e| e).collect();
        Scanned { number , entries: ScannedMedia::new_batch(list, exif_tool).await}
    }
}

/**
Event Messages

*/
#[derive(Debug, Clone, Copy)]
pub enum MediaPathMessage {
    Remove, // Remove path
    ExpandAccordion,
    CollapseAccordion,
    ToggleAccordion,
    Scan,
    ScanAll,
}

/**
MediaLocationInfo

*/
impl MediaLocationInfo {
    // TODO: Somehow let this assume ownership of the parameters
    pub fn new(name: String, location: String) -> Result<MediaLocationInfo, MediaPathError> {
        match std::path::Path::new(&location).canonicalize() {
            Ok(path) => {
                match path.try_exists() {
                    // Returns true, false, and Err (Err means cannot be determined due to permissions)
                    Ok(b) => {
                            if b {
                                Ok(MediaLocationInfo {
                                    name,
                                    path: PathBuf::from(path.canonicalize().unwrap()),
                                    dropdown_opened: false,
                                    items: MediaLocationItems::Unscanned,
                                })
                            } else {
                                Err(NotADirectory)
                            }
                    }
                    Err(err) => Err(NoPermission),
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
        let header = row![
            column![text(self.name.to_string()).size(25).width(Fill),
            scanned_status],
            button("Toggle").on_press(MediaPathMessage::ToggleAccordion),
        ]
            .align_y(Alignment::Center);
        let wrapper = if self.dropdown_opened {
            let mut body: Column<MediaPathMessage> = column![].into();
            match &self.items {
                MediaLocationItems::Unscanned => {
                    body = body.push(text!("Unscanned!"));
                }
                MediaLocationItems::Scanning => {
                    body = body.push(text!("Scanning!"));
                }
                MediaLocationItems::Scanned(list) => {
                    if list.number <= 0 {
                        body = body.push(text!("Empty!"))
                    }
                    for (i, e) in list.entries.iter().enumerate() {
                        body = body.push(text(format!("{i}: {}\r\n    DateTimeOriginal: {}", e.file_name().into_string().unwrap(), e.date_time_original)));
                    };
                }
                MediaLocationItems::Error(err) => {
                    body = body.push(text!("Error: {}", err))
                }
            }
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

    async fn scan(&mut self, exif_tool: Arc<Mutex<ExifTool>> ) {
        match self.path.read_dir().await {
            Ok(dir) => {
                self.items = MediaLocationItems::Scanned(Scanned::new(dir, exif_tool).await);
            }
            Err(err) => self.items = MediaLocationItems::Error(err.to_string())
        }
        yield_now().await
    }

}

/**
MediaPathList

*/
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

    pub async fn scan(&mut self, index: usize, exif_tool: Arc<Mutex<ExifTool>>) {
        self.get_mut(index).scan(exif_tool).await
    }

    pub async fn scan_all(mut self, exif_tool: Arc<Mutex<ExifTool>>) -> Self {
        for info in self.list.iter_mut() {
            info.scan(exif_tool.clone()).await
        }
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
