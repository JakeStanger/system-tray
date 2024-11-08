use crate::client::Event;
use thiserror::Error;
use tokio::sync::broadcast::error::SendError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("dbus properties missing one or more required fields")]
    MissingProperty(&'static str),
    #[error("failed to send event through tokio broadcast channel")]
    EventSend(#[from] SendError<Event>),
    #[error("zbus error")]
    ZBus(#[from] zbus::Error),
    #[error("zbus fdo error")]
    ZBusFdo(#[from] zbus::fdo::Error),
    #[error("zbus variant error")]
    ZBusVariant(#[from] zbus::zvariant::Error),
    #[error("invalid data error")]
    InvalidData(&'static str),
}
