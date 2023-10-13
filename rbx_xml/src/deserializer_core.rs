use std::{collections::HashMap, io};

use quick_xml::{
    events::{BytesEnd, BytesStart, Event},
    Reader,
};

use crate::core::XmlType;

use super::error2::{DecodeError, DecodeErrorKind};

pub type XmlReadResult = Result<XmlReadEvent, DecodeError>;

pub struct XmlEventReader<R: io::Read> {
    reader: Reader<io::BufReader<R>>,
    peeked: Option<XmlReadResult>,
    event_buffer: Vec<u8>,
    finished: bool,
}

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

impl<R: io::Read> Iterator for XmlEventReader<R> {
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
                    Event::CData(bytes) => to_string_helper(&bytes).map(XmlReadEvent::Text),
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

impl<R: io::Read> XmlEventReader<R> {
    /// Returns the byte offset of the internal reader.
    ///
    /// Note that as of this moment (9 October 2023), `quick_xml` doesn't
    /// track the row or column location and it is prohibitively expensive
    /// to calculate.
    pub(crate) fn location(&self) -> usize {
        self.reader.buffer_position()
    }

    /// Creates a new `XmlReader` from the provided argument.
    pub fn from_source(source: R) -> XmlEventReader<R> {
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
        DecodeError::from_reader::<R, T>(self, kind)
    }

    pub fn expect_next(&mut self) -> XmlReadResult {
        match self.next() {
            Some(inner) => inner,
            None => Err(self.error(DecodeErrorKind::UnexpectedEof)),
        }
    }
    pub fn expect_peek(&mut self) -> XmlReadResult {
        match self.peek() {
            Some(Err(_)) => Err(self.next().unwrap().unwrap_err()),
            Some(Ok(event)) => Ok(self.next().unwrap().unwrap()),
            None => Err(self.error(DecodeErrorKind::UnexpectedEof)),
        }
    }

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

    fn read_characters_inner(&mut self) -> Result<Option<String>, DecodeError> {
        // `peek` returns Option<&Result>, so we can't just use the inner
        // values. So, we have to use `next` in the event we need the value.
        match self.peek() {
            Some(Ok(XmlReadEvent::Text(_))) => match self.next().unwrap().unwrap() {
                XmlReadEvent::Text(data) => Ok(Some(data)),
                _ => unreachable!(),
            },
            Some(Err(_)) => {
                let kind = self.next().unwrap().unwrap_err();
                Err(kind)
            }
            _ => Ok(None),
        }
    }

    pub fn read_characters(&mut self) -> Result<String, DecodeError> {
        let mut buffer = match self.read_characters_inner()? {
            Some(buffer) => buffer,
            None => return Ok(String::new()),
        };

        while let Some(part) = self.read_characters_inner()? {
            buffer.push_str(&part);
        }

        Ok(buffer)
    }

    pub fn read_base64_characters(&mut self) -> Result<Vec<u8>, DecodeError> {
        let contents: String = self
            .read_characters()?
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();

        base64::decode(contents).map_err(|e| self.error(e))
    }

    pub fn read_tag_contents(&mut self, expected_name: &str) -> Result<String, DecodeError> {
        self.expect_start_with_name(expected_name)?;
        let contents = self.read_characters()?;
        self.expect_end_with_name(expected_name)?;

        Ok(contents)
    }

    /// Read a value that implements XmlType.
    pub(crate) fn read_value<T: XmlType>(&mut self) -> Result<T, DecodeError> {
        // T::read_xml(self)
        todo!()
    }

    /// Read a value that implements XmlType, expecting it to be enclosed in an
    /// outer tag.
    pub(crate) fn read_value_in_tag<T: XmlType>(
        &mut self,
        tag_name: &str,
    ) -> Result<T, DecodeError> {
        // self.expect_start_with_name(tag_name)?;
        // let value = self.read_value()?;
        // self.expect_end_with_name(tag_name)?;

        // Ok(value)
        todo!()
    }

    pub fn eat_unknown_tag(&mut self) -> Result<(), DecodeError> {
        let mut depth = 0;

        log::trace!("Starting unknown block");

        loop {
            match self.expect_next()? {
                XmlReadEvent::StartElement { name, .. } => {
                    log::trace!("Eat unknown start: {:?}", name);
                    depth += 1;
                }
                XmlReadEvent::EndElement { name } => {
                    log::trace!("Eat unknown end: {:?}", name);
                    depth -= 1;

                    if depth == 0 {
                        log::trace!("Reached end of unknown block");
                        break;
                    }
                }
                other => {
                    log::trace!("Eat unknown: {:?}", other);
                }
            }
        }

        Ok(())
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
