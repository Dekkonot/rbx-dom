use std::{collections::HashMap, io};

use quick_xml::{
    events::{BytesEnd, BytesStart, Event},
    Reader,
};

use super::error2::{DecodeError, DecodeErrorKind};

pub type XmlReadResult = Result<XmlReadEvent, DecodeError>;

pub struct XmlReader<R: io::Read> {
    reader: Reader<io::BufReader<R>>,
    peeked: Option<XmlReadResult>,
    event_buffer: Vec<u8>,
    finished: bool,
}

// TODO: split read events into seperate structs that are returned in a wrapper enum

#[derive(Debug)]
pub enum XmlReadEvent {
    StartElement {
        name: String,
        attributes: HashMap<String, String>,
    },
    Text(String),
    EndElement {
        name: String,
    },
}

impl XmlReadEvent {
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::StartElement { .. } => "StartElement",
            Self::Text(_) => "Text",
            Self::EndElement { .. } => "EndElement",
        }
    }
}

impl<R: io::Read> Iterator for XmlReader<R> {
    type Item = XmlReadResult;

    fn next(&mut self) -> Option<Self::Item> {
        if self.peeked.is_some() {
            return self.peeked.take();
        }
        if self.finished {
            return None;
        }

        let res = match self.reader.read_event_into(&mut self.event_buffer) {
            Ok(event) => {
                match event {
                    Event::Start(bytes) => parse_start(&bytes),
                    Event::End(bytes) => parse_end(&bytes),
                    Event::Text(bytes) => bytes
                        .unescape()
                        .map(|data| XmlReadEvent::Text(data.into()))
                        .map_err(|e| e.into()),
                    Event::Eof => {
                        self.finished = true;
                        return None;
                    }
                    // TODO error here
                    _ => panic!("what"),
                }
                .map_err(|e| self.error(e))
            }
            Err(err) => Err(self.error(err)),
        };

        if res.is_err() {
            self.finished = true;
        }
        Some(res)
    }
}

impl<R: io::Read> XmlReader<R> {
    /// Returns the byte offset of the internal reader.
    ///
    /// Note that as of this moment (9 October 2023), `quick_xml` doesn't
    /// track the row or column location and it is prohibitively expensive
    /// to calculate.
    pub(crate) fn location(&self) -> usize {
        self.reader.buffer_position()
    }

    /// Creates a new `XmlReader` from the provided argument.
    pub fn from_source(source: R) -> XmlReader<R> {
        let mut reader = Reader::from_reader(io::BufReader::new(source));
        reader.trim_text(true);
        Self {
            reader,
            event_buffer: Vec::new(),
            peeked: None,
            finished: false,
        }
    }

    pub fn peek(&mut self) -> Option<&XmlReadResult> {
        if self.peeked.is_none() {
            self.peeked = self.next();
        }
        self.peeked.as_ref()
    }

    pub(crate) fn error<T: Into<DecodeErrorKind>>(&self, kind: T) -> DecodeError {
        DecodeError::from_reader(self, kind)
    }

    pub fn expect_next(&mut self) -> XmlReadResult {
        match self.next() {
            Some(inner) => inner,
            None => Err(self.error(DecodeErrorKind::UnexpectedEof)),
        }
    }

    // Previous versions included an `expect_peek` method but that method
    // had a soundness problem. It extended a lifetime without bound, so it
    // could have resulted in a dangling reference.

    pub fn expect_start_with_name(
        &mut self,
        expected_name: &str,
    ) -> Result<HashMap<String, String>, DecodeError> {
        match self.expect_next()? {
            XmlReadEvent::StartElement { name, attributes } => {
                if name != expected_name {
                    Err(self.error(DecodeErrorKind::UnexpectedElementStart {
                        expected: expected_name.into(),
                        got: name,
                    }))
                } else {
                    Ok(attributes)
                }
            }
            event => Err(self.error(DecodeErrorKind::UnexpectedXmlEvent {
                expected: "ElementStart",
                got: event.kind(),
            })),
        }
    }

    pub fn expect_end_with_name(&mut self, expected_name: &str) -> Result<(), DecodeError> {
        match self.expect_next()? {
            XmlReadEvent::EndElement { name } => {
                if name != expected_name {
                    Err(self.error(DecodeErrorKind::UnexpectedElementEnd {
                        expected: expected_name.into(),
                        got: name,
                    }))
                } else {
                    Ok(())
                }
            }
            event => Err(self.error(DecodeErrorKind::UnexpectedXmlEvent {
                expected: "ElementEnd",
                got: event.kind(),
            })),
        }
    }
}

/// Takes a borrowed quick_xml start event and parses it into a
/// set of owned data. Despite returning a generic `XmlReadEvent`,
/// this always returns an `XmlReadEvent::Start`.
fn parse_start(event: &BytesStart) -> Result<XmlReadEvent, DecodeErrorKind> {
    let name = to_string_helper(event.name().into_inner())?;
    let attributes: Result<HashMap<String, String>, DecodeErrorKind> = event
        .attributes()
        .map(|maybe| match maybe {
            Ok(attr) => Ok((
                to_string_helper(attr.key.into_inner())?,
                to_string_helper(&attr.value)?,
            )),
            Err(err) => Err(err.into()),
        })
        .collect();
    Ok(XmlReadEvent::StartElement {
        name,
        attributes: attributes?,
    })
}

/// Takes a borrow quick_xml end event and returns an owned event.
/// Despite returning a generic `XmlReadEvent`, this always returns an
/// `XmlReadEvent::End`.
#[inline]
fn parse_end(event: &BytesEnd) -> Result<XmlReadEvent, DecodeErrorKind> {
    to_string_helper(event.name().into_inner()).map(|name| XmlReadEvent::EndElement { name })
}

/// A simple helper function for converting to a string
#[inline]
fn to_string_helper(data: &[u8]) -> Result<String, DecodeErrorKind> {
    let inner = Vec::from(data);
    Ok(String::from_utf8(inner)?)
}
