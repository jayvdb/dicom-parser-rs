use crate::attribute::Attribute;
use crate::encoding::Encoding;
use crate::handler::Control;
use crate::handler::Handler;
use crate::parser::data::DataParser;
use crate::parser::data_undefined_length::DataUndefinedLengthParser;
use crate::parser::encapsulated_pixel_data::EncapsulatedPixelDataParser;
use crate::parser::sequence::SequenceParser;
use crate::parser::ParseResult;
use crate::parser::Parser;
use crate::tag::Tag;
use crate::vr::VR;

pub struct AttributeParser<T: Encoding> {
    parser: Option<Box<dyn Parser<T>>>,
}

impl<T: Encoding> AttributeParser<T> {
    pub fn default() -> AttributeParser<T> {
        AttributeParser::<T> { parser: None }
    }
}

impl<T: 'static + Encoding> Parser<T> for AttributeParser<T> {
    fn parse(&mut self, handler: &mut dyn Handler, bytes: &[u8]) -> Result<ParseResult<T>, ()> {
        match &mut self.parser {
            None => {
                let (bytes_consumed, attribute) = match parse_attribute::<T>(bytes) {
                    Ok((bytes_consumed, attribute)) => (bytes_consumed, attribute),
                    Err(()) => {
                        return Ok(ParseResult::incomplete(0));
                    }
                };

                match handler.element(&attribute) {
                    Control::Continue => {}
                    Control::Filter => {
                        // TODO: Skip data
                    }
                    Control::Cancel => {
                        return Ok(ParseResult::cancelled(0));
                    }
                }

                let remaining_byes = &bytes[bytes_consumed..];

                // return incomplete if we have undefined length and less than 8 bytes
                // to parse (we need 8 to look ahead to see if its a sequence)
                if attribute.length == 0xFFFF_FFFF && remaining_byes.len() < 8 {
                    return Ok(ParseResult::incomplete(bytes_consumed));
                }

                self.parser = Some(make_parser::<T>(handler, attribute, remaining_byes));
                let mut parse_result = self
                    .parser
                    .as_mut()
                    .unwrap()
                    .parse(handler, remaining_byes)?;
                parse_result.bytes_consumed += bytes_consumed;
                Ok(parse_result)
            }
            Some(parser) => parser.parse(handler, bytes),
        }
    }
}

fn make_parser<T: 'static + Encoding>(
    handler: &mut dyn Handler,
    attribute: Attribute,
    bytes: &[u8],
) -> Box<dyn Parser<T>> {
    if attribute.vr == Some(VR::SQ) {
        handler.start_sequence(&attribute);
        Box::new(SequenceParser::<T>::new(attribute))
    } else if is_encapsulated_pixel_data(&attribute) {
        Box::new(EncapsulatedPixelDataParser::new(attribute))
    } else if attribute.length == 0xFFFF_FFFF {
        // TODO: Consider moving sequence parsing into dataundefinedlengthparser
        if is_sequence::<T>(bytes) {
            handler.start_sequence(&attribute);
            Box::new(SequenceParser::<T>::new(attribute))
        } else {
            Box::new(DataUndefinedLengthParser::<T>::new(attribute))
        }
    } else {
        Box::new(DataParser::<T>::new(attribute))
    }
}

fn parse_attribute<T: Encoding>(bytes: &[u8]) -> Result<(usize, Attribute), ()> {
    if bytes.len() < 6 {
        return Err(());
    }
    let group = T::u16(&bytes[0..2]);
    let element = T::u16(&bytes[2..4]);
    let tag = Tag::new(group, element);

    let (vr, length, bytes_consumed) = if is_sequence_tag(tag) {
        if bytes.len() < 8 {
            return Err(());
        }
        let length = T::u32(&bytes[4..8]) as usize;
        (None, length, 4)
    } else {
        T::vr_and_length(&bytes)?
    };

    let attribute = Attribute {
        tag: Tag::new(group, element),
        vr,
        length,
    };
    Ok((bytes_consumed, attribute))
}

fn is_sequence_tag(tag: Tag) -> bool {
    tag.group == 0xFFFE && (tag.element == 0xE000 || tag.element == 0xE00D || tag.element == 0xE0DD)
}

fn is_encapsulated_pixel_data(attribute: &Attribute) -> bool {
    attribute.tag == Tag::new(0x7fe0, 0x0010) && attribute.length == 0xffff_ffff
}

fn is_sequence<T: Encoding>(bytes: &[u8]) -> bool {
    // peek ahead to see if it looks like a sequence
    if bytes.len() >= 8 {
        let item_tag = Tag::from_bytes::<T>(&bytes[0..4]);
        if item_tag.group == 0xFFFE && item_tag.element == 0xE000 {
            return true;
        }
    }
    false
}
