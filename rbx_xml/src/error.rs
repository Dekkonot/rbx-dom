use std::{
    fmt,
    io::{self, Read, Write},
};

use rbx_dom_weak::types::VariantType;
use thiserror::Error;

use crate::deserializer_core::{XmlEventReader, XmlReadEvent};

#[derive(Debug, Error)]
#[error(transparent)]
/// An error that can occur when deserializing an XML-format model or place.
pub struct DecodeError {
    // This indirection drops the size of the error type substantially (~150
    // bytes to 8 on 64-bit), which is important since it's passed around every
    // function!
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
    #[error("property value was invalid because: {0}")]
    InvalidContent(&'static str),

    #[error("unexpected element start (expected {expected}, got {got})")]
    UnexpectedElementStart { expected: String, got: String },
    #[error("unexpected element end (expected {expected}, got {got})")]
    UnexpectedElementEnd { expected: String, got: String },
    #[error("unexpected EoF")]
    UnexpectedEof,
    #[error("unexpected XML event {0:?}")]
    UnexpectedXmlEvent(XmlReadEvent),
}

/// An error that can occur when serializing an XML-format model or place.
#[derive(Debug)]
pub struct EncodeError {
    // This Box helps reduce the size of EncodeError a lot, which is important.
    kind: Box<EncodeErrorKind>,
}

impl EncodeError {
    pub(crate) fn new_from_writer<W: Write>(
        kind: EncodeErrorKind,
        _writer: &xml::EventWriter<W>,
    ) -> EncodeError {
        EncodeError {
            kind: Box::new(kind),
        }
    }
}

impl fmt::Display for EncodeError {
    fn fmt(&self, output: &mut fmt::Formatter) -> fmt::Result {
        write!(output, "{}", self.kind)
    }
}

impl std::error::Error for EncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.kind.source()
    }
}

#[derive(Debug)]
pub(crate) enum EncodeErrorKind {
    Io(io::Error),
    Xml(xml::writer::Error),
    Type(rbx_dom_weak::types::Error),

    UnknownProperty {
        class_name: String,
        property_name: String,
    },
    UnsupportedPropertyType(VariantType),
    UnsupportedPropertyConversion {
        class_name: String,
        property_name: String,
        expected_type: VariantType,
        actual_type: VariantType,
        message: String,
    },
}

impl fmt::Display for EncodeErrorKind {
    fn fmt(&self, output: &mut fmt::Formatter) -> fmt::Result {
        use self::EncodeErrorKind::*;

        match self {
            Io(err) => write!(output, "{}", err),
            Xml(err) => write!(output, "{}", err),
            Type(err) => write!(output, "{}", err),

            UnknownProperty {
                class_name,
                property_name,
            } => write!(
                output,
                "Property {}.{} is unknown",
                class_name, property_name
            ),
            UnsupportedPropertyType(ty) => {
                write!(output, "Properties of type {:?} cannot be encoded yet", ty)
            }
            UnsupportedPropertyConversion {
                class_name,
                property_name,
                expected_type,
                actual_type,
                message,
            } => write!(
                output,
                "Property {}.{} is expected to be of type {:?}, but it was of type {:?} \
                 When trying to convert the value, this error occured: {}",
                class_name, property_name, expected_type, actual_type, message
            ),
        }
    }
}

impl std::error::Error for EncodeErrorKind {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use self::EncodeErrorKind::*;

        match self {
            Io(err) => Some(err),
            Xml(err) => Some(err),
            Type(err) => Some(err),

            UnknownProperty { .. }
            | UnsupportedPropertyType(_)
            | UnsupportedPropertyConversion { .. } => None,
        }
    }
}

impl From<xml::writer::Error> for EncodeErrorKind {
    fn from(error: xml::writer::Error) -> EncodeErrorKind {
        EncodeErrorKind::Xml(error)
    }
}

impl From<io::Error> for EncodeErrorKind {
    fn from(error: io::Error) -> EncodeErrorKind {
        EncodeErrorKind::Io(error)
    }
}

impl From<rbx_dom_weak::types::Error> for EncodeErrorKind {
    fn from(error: rbx_dom_weak::types::Error) -> EncodeErrorKind {
        EncodeErrorKind::Type(error)
    }
}
