use crate::attribute::Attribute;
use crate::encoding::Encoding;
use crate::parser::attribute::AttributeParser;
use crate::parser::data_set::parse_full;
use crate::parser::data_set::Parser;
use crate::parser::handler::Control;
use crate::parser::handler::Handler;
use crate::tag;
use crate::tag::Tag;
use std::marker::PhantomData;

pub struct SequenceUndefinedLengthParser<T: Encoding> {
    pub attribute: Attribute,
    pub phantom: PhantomData<T>,
}

impl<T: Encoding> SequenceUndefinedLengthParser<T> {}

impl<T: 'static + Encoding> Parser<T> for SequenceUndefinedLengthParser<T> {
    fn parse(
        &mut self,
        handler: &mut dyn Handler,
        bytes: &[u8],
    ) -> Result<(usize, Box<dyn Parser<T>>), ()> {
        let mut remaining_bytes = bytes;

        let mut bytes_consumed = 0;

        while !remaining_bytes.is_empty() {
            if remaining_bytes.len() < 8 {
                return Err(());
            }
            let item_tag = Tag::from_bytes::<T>(&remaining_bytes[0..4]);
            let item_length = T::u32(&remaining_bytes[4..8]) as usize;

            if item_tag != tag::ITEM {
                panic!("expecting item");
            }

            if item_length != 0xFFFF_FFFF {
                panic!("undefined length expected");
            }

            remaining_bytes = &remaining_bytes[8..];
            bytes_consumed += 8;

            handler.start_sequence_item(&self.attribute);

            let mut sequence_item_handler = SequenceItemHandler { handler };

            let consumed = match parse_full::<T>(&mut sequence_item_handler, remaining_bytes) {
                Ok(consumed) => consumed,
                Err(remaining) => remaining_bytes.len() - remaining + 8,
            };

            handler.end_sequence_item(&self.attribute);

            remaining_bytes = &remaining_bytes[consumed..];
            bytes_consumed += consumed;

            // check for end of sequence
            if remaining_bytes.len() < 8 {
                return Err(());
            }
            let item_tag = Tag::from_bytes::<T>(&remaining_bytes[0..4]);
            let _item_length = T::u32(&remaining_bytes[4..8]) as usize;
            if item_tag == tag::SEQUENCEDELIMITATIONITEM {
                // end of sequence
                bytes_consumed += 8;
                break;
            }
        }

        let attribute_parser = Box::new(AttributeParser::<T> {
            phantom: PhantomData,
        });

        Ok((bytes_consumed, attribute_parser))
    }
}

pub fn parse_sequence_item<T: Encoding>(bytes: &[u8]) -> Result<usize, ()> {
    let group = T::u16(&bytes[0..2]);
    let element = T::u16(&bytes[2..4]);
    if group != 0xFFFE || element != 0xE000 {
        return Err(());
    }
    let length = T::u32(&bytes[4..8]) as usize;
    Ok(length)
}

struct SequenceItemHandler<'t> {
    handler: &'t mut dyn Handler,
}

impl Handler for SequenceItemHandler<'_> {
    fn element(&mut self, attribute: &Attribute) -> Control {
        if attribute.tag == tag::ITEMDELIMITATIONITEM {
            return Control::Stop;
        }
        self.handler.element(&attribute)
    }
    fn start_sequence_item(&mut self, attribute: &Attribute) {
        self.handler.start_sequence_item(&attribute)
    }
    fn end_sequence_item(&mut self, attribute: &Attribute) {
        self.handler.end_sequence_item(&attribute)
    }
    fn basic_offset_table(&mut self, attribute: &Attribute, data: &[u8]) -> Control {
        self.handler.basic_offset_table(&attribute, data)
    }
    fn pixel_data_fragment(&mut self, attribute: &Attribute, data: &[u8]) -> Control {
        self.handler.pixel_data_fragment(&attribute, data)
    }
}
