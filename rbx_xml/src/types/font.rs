use std::io::{Read, Write};

use rbx_dom_weak::types::{Content, Font, FontStyle, FontWeight};

use crate::{
    core::XmlType,
    deserializer_core::{XmlEventReader, XmlReadEvent},
    error::{DecodeError, EncodeError},
    serializer_core::XmlEventWriter,
};

impl XmlType for Font {
    const XML_TAG_NAME: &'static str = "Font";

    fn write_xml<W: Write>(&self, writer: &mut XmlEventWriter<W>) -> Result<(), EncodeError> {
        writer.write_value_in_tag(&Content::from(self.family.as_str()), "Family")?;

        writer.write_value_in_tag(&self.weight.as_u16(), "Weight")?;

        let style = match self.style {
            FontStyle::Normal => "Normal",
            FontStyle::Italic => "Italic",
        };
        writer.write_tag_characters("Style", style)?;

        if let Some(ref cached_face_id) = self.cached_face_id {
            writer.write_value_in_tag(&Content::from(cached_face_id.as_str()), "CachedFaceId")?;
        }

        Ok(())
    }

    fn read_xml<R: Read>(reader: &mut XmlEventReader<R>) -> Result<Self, DecodeError> {
        // Patchwork fix for older Roblox files that were written with invalid
        // `Font` tags
        if let XmlReadEvent::EndElement { .. } = reader.expect_peek()? {
            return Ok(Font::default());
        }

        let family = reader.read_value_in_tag::<Content>("Family")?.into_string();

        let weight: u16 = reader.read_value_in_tag("Weight")?;
        let weight = FontWeight::from_u16(weight).unwrap_or_default();

        let style = match reader.read_tag_contents("Style")?.as_str() {
            "Normal" => FontStyle::Normal,
            "Italic" => FontStyle::Italic,
            _ => FontStyle::Normal,
        };

        let cached_face_id = match reader.expect_peek()? {
            XmlReadEvent::StartElement { name, .. } if name == "CachedFaceId" => Some(
                reader
                    .read_value_in_tag::<Content>("CachedFaceId")?
                    .into_string(),
            ),
            _ => None,
        };

        Ok(Font {
            family,
            weight,
            style,
            cached_face_id,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::test_util;

    #[test]
    fn round_trip_font_face() {
        test_util::test_xml_round_trip(&Font {
            family: "rbxasset://fonts/families/SourceSansPro.json".to_owned(),
            weight: FontWeight::Regular,
            style: FontStyle::Normal,
            cached_face_id: Some("rbxasset://fonts/SourceSansPro-Regular.ttf".to_owned()),
        });
    }

    #[test]
    fn deserialize_font_face() {
        test_util::test_xml_deserialize(
            r#"
                <Font name="foo">
                    <Family><url>rbxasset://fonts/families/SourceSansPro.json</url></Family>
                    <Weight>400</Weight>
                    <Style>Normal</Style>
                    <CachedFaceId><url>rbxasset://fonts/SourceSansPro-Regular.ttf</url></CachedFaceId>
                </Font>
            "#,
            &Font {
                family: "rbxasset://fonts/families/SourceSansPro.json".to_owned(),
                weight: FontWeight::Regular,
                style: FontStyle::Normal,
                cached_face_id: Some("rbxasset://fonts/SourceSansPro-Regular.ttf".to_owned()),
            },
        );
    }

    #[test]
    fn serialize_font_face() {
        test_util::test_xml_serialize(
            r#"
            <Font name="foo">
                <Family><url>rbxasset://fonts/families/SourceSansPro.json</url></Family>
                <Weight>400</Weight>
                <Style>Normal</Style>
                <CachedFaceId><url>rbxasset://fonts/SourceSansPro-Regular.ttf</url></CachedFaceId>
            </Font>
            "#,
            &Font {
                family: "rbxasset://fonts/families/SourceSansPro.json".to_owned(),
                weight: FontWeight::Regular,
                style: FontStyle::Normal,
                cached_face_id: Some("rbxasset://fonts/SourceSansPro-Regular.ttf".to_owned()),
            },
        );
    }
}
