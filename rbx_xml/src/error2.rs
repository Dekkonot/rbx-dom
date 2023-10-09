use std::io::Read;

use thiserror::Error;

use crate::deserialize_core2::{XmlReadEvent, XmlReader};

#[derive(Debug, Error)]
#[error(transparent)]
pub struct DecodeError {
    inner: Box<DecodeErrorInner>,
}

impl DecodeError {
    pub(crate) fn from_reader<R: Read, E: Into<DecodeErrorKind>>(
        reader: &XmlReader<R>,
        kind: E,
    ) -> Self {
        Self {
            inner: Box::from(DecodeErrorInner {
                location: reader.location(),
                source: kind.into(),
            }),
        }
    }
}

#[derive(Debug, Error)]
#[error("byte {location}: {source}")]
struct DecodeErrorInner {
    location: usize,
    source: DecodeErrorKind,
}

#[derive(Debug, Error)]
pub(crate) enum DecodeErrorKind {
    #[error(transparent)]
    Xml(#[from] quick_xml::Error),
    #[error(transparent)]
    AttributeError(#[from] quick_xml::events::attributes::AttrError),
    #[error(transparent)]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
    #[error(transparent)]
    ParseFloat(#[from] std::num::ParseFloatError),
    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),
    #[error(transparent)]
    DecodeBase64(#[from] base64::DecodeError),
    #[error(transparent)]
    Migration(#[from] rbx_reflection::MigrationError),
    #[error(transparent)]
    Type(#[from] rbx_dom_weak::types::Error),

    #[error("element {element} is missing attribute {name}")]
    MissingAttribute { name: String, element: String },

    #[error("unexpected element start (expected {expected}, got {got})")]
    UnexpectedElementStart { expected: String, got: String },
    #[error("unexpected element end (expected {expected}, got {got})")]
    UnexpectedElementEnd { expected: String, got: String },
    #[error("unexpected EoF")]
    UnexpectedEof,
    #[error("unexpected XML event {got} when expecting {expected}")]
    UnexpectedXmlEvent {
        expected: &'static str,
        got: &'static str,
    },
}
