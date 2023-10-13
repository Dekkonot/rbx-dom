use std::io::Read;

use rbx_dom_weak::types::VariantType;
use thiserror::Error;

use crate::deserializer_core::{XmlEventReader, XmlReadEvent};

#[derive(Debug, Error)]
#[error(transparent)]
pub struct DecodeError {
    inner: Box<DecodeErrorInner>,
}

impl DecodeError {
    pub(crate) fn from_reader<R: Read, E: Into<DecodeErrorKind>>(
        reader: &XmlEventReader<R>,
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

    #[error("document was wrong version (supported version is 4, document is version {0})")]
    WrongDocVersion(String),
    #[error("element {element} is missing attribute {name}")]
    MissingAttribute { name: &'static str, element: String },

    #[error("Name property must be a String (was a {0:?})")]
    NameMustBeString(VariantType),
    #[error("Property {class_name}.{property_name} is expected to be of type {expected_type:?}, but it was of type {actual_type:?}. When trying to convert, this error occured: {message}")]
    UnsupportedPropertyConversion {
        class_name: String,
        property_name: String,
        expected_type: VariantType,
        actual_type: VariantType,
        message: String,
    },
    #[error("Property {class_name}.{property_name} is unknown")]
    UnknownProperty {
        class_name: String,
        property_name: String,
    },

    #[error("unexpected element start (expected {expected}, got {got})")]
    UnexpectedElementStart { expected: String, got: String },
    #[error("unexpected element end (expected {expected}, got {got})")]
    UnexpectedElementEnd { expected: String, got: String },
    #[error("unexpected EoF")]
    UnexpectedEof,
    #[error("unexpected XML event {0:?}")]
    UnexpectedXmlEvent(XmlReadEvent),
}
