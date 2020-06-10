use crate::encoding::ExplicitBigEndian;
use crate::encoding::ExplicitLittleEndian;
use crate::encoding::ImplicitLittleEndian;
use crate::meta_information;
use crate::meta_information::MetaInformation;
use crate::parser::data_set::parse_full;
use crate::parser::handler::Handler;

pub fn parse<'a, T: Handler>(
    callback: &'a mut T,
    bytes: &mut [u8],
) -> Result<MetaInformation, usize> {
    let meta = meta_information::parse(&bytes).unwrap();
    let remaining_bytes = &bytes[meta.end_position..];
    let result = match &meta.transfer_syntax_uid[..] {
        "1.2.840.10008.1.2" => {
            // implicit little endian
            parse_full::<ImplicitLittleEndian>(callback, remaining_bytes)
        }
        "1.2.840.10008.1.2.2" => {
            // explicit big endian
            parse_full::<ExplicitBigEndian>(callback, remaining_bytes)
        }
        "1.2.840.10008.1.2.1.99" => panic!("deflated not suported yet"),
        _ => {
            // explicit little endian
            parse_full::<ExplicitLittleEndian>(callback, remaining_bytes)
        }
    };
    match result {
        Err(bytes_remaining) => Err(bytes_remaining),
        Ok(()) => Ok(meta),
    }
}

#[cfg(test)]
mod tests {

    use super::parse;
    use crate::condition;
    use crate::data_set_handler::DataSetHandler;
    use crate::meta_information::tests::make_p10_header;
    use std::fs::File;
    use std::io::Read;

    #[allow(dead_code)]
    pub fn read_file(filepath: &str) -> Vec<u8> {
        let mut file = File::open(filepath).unwrap();
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).unwrap();
        buffer
    }

    fn make_p10_file() -> Vec<u8> {
        let mut bytes = make_p10_header();
        bytes.extend_from_slice(&vec![0x08, 0x00, 0x05, 0x00, b'C', b'S', 2, 0, b'I', b'S']);

        bytes
    }

    #[test]
    fn explicit_little_endian_parses() {
        let mut bytes = make_p10_file();
        let mut handler = DataSetHandler::new(condition::none, condition::none);
        parse(&mut handler, &mut bytes).unwrap();
        assert_eq!(handler.dataset.attributes.len(), 1);
    }

    #[test]
    fn explicit_little_endian() {
        let mut bytes = read_file("tests/fixtures/CT1_UNC.explicit_little_endian.dcm");
        let mut handler = DataSetHandler::new(condition::none, condition::none);
        //accumulator.print = true;
        parse(&mut handler, &mut bytes).unwrap();
        assert_eq!(257, handler.dataset.attributes.len());
        //println!("Parsed {:?} attributes", accumulator.attributes.len());
        //println!("{:?}", accumulator.attributes);
    }

    #[test]
    fn implicit_little_endian() {
        let mut bytes = read_file("tests/fixtures/CT1_UNC.implicit_little_endian.dcm");
        let mut handler = DataSetHandler::new(condition::none, condition::none);
        //accumulator.print = true;
        parse(&mut handler, &mut bytes).unwrap();
        assert_eq!(257, handler.dataset.attributes.len());
        //println!("Parsed {:?} attributes", accumulator.attributes.len());
        //println!("{:?}", accumulator.attributes);
    }

    #[test]
    fn explicit_big_endian() {
        let mut bytes = read_file("tests/fixtures/CT1_UNC.explicit_big_endian.dcm");
        let mut handler = DataSetHandler::new(condition::none, condition::none);
        //accumulator.print = true;
        parse(&mut handler, &mut bytes).unwrap();
        assert_eq!(257, handler.dataset.attributes.len());
        //println!("Parsed {:?} attributes", accumulator.attributes.len());
    }
    #[test]
    fn sequences() {
        //(0008,9121) @ position 0x376 / 886
        let mut bytes = read_file("tests/fixtures/CT0012.fragmented_no_bot_jpeg_ls.80.dcm");
        let mut handler = DataSetHandler::new(condition::none, condition::none);
        //handler.print = true;
        match parse(&mut handler, &mut bytes) {
            Err(remaining) => println!("remaining {}", remaining),
            Ok(_) => {}
        }
        //println!("Parsed {:?} attributes", accumulator.attributes.len());
    }
}
