use super::graycodeanalysis::read_u32;
use pzip_huffman::hufbites::encode_itself_to_bytes;
use super::foc::vec_diff;
use std::collections::HashMap;

use std::io::{BufWriter, BufReader, Read, Write};
use compress::bwt;
use compress::entropy::ari;
use compress::bwt::{dc, mtf};
use compress::rle;

pub fn apply_bwt(data: &[u8]) -> Vec<u8> {
    let mut e = bwt::Encoder::new(BufWriter::new(Vec::new()), 4 << 20);
    e.write(data).unwrap();
    let (encoded, _) = e.finish();
    encoded.into_inner().unwrap()
}

pub fn reverse_bwt(data: &[u8]) -> Vec<u8> {
    let mut d = bwt::Decoder::new(BufReader::new(&data[..]), true);
    let mut decoded = Vec::new();
    d.read_to_end(&mut decoded).unwrap();
    decoded
}

fn apply_dc(data: &[u8]) -> Vec<u8> {
    dc::encode_simple::<u8>(&data[..])
}

fn reverse_dc(data: &[u8], size: usize) -> Vec<u8> {
    dc::decode_simple(size, &data[..])
}

fn apply_mtf(data: &[u8]) -> Vec<u8> {
    let mut e = mtf::Encoder::new(BufWriter::new(Vec::new()));
    e.write_all(data).unwrap();
    e.finish().into_inner().unwrap()
}

fn reverse_mtf(data: &[u8]) -> Vec<u8> {
    let mut d = mtf::Decoder::new(BufReader::new(&data[..]));
    let mut decoded = Vec::new();
    d.read_to_end(&mut decoded).unwrap();
    decoded
}

pub fn apply_range_coding(data: &[u8]) -> Vec<u8> {
    let mut e = ari::ByteEncoder::new(BufWriter::new(Vec::new()));
    e.write_all(data).unwrap();
    let (encoded, _) = e.finish();
    encoded.into_inner().unwrap()
}

pub fn reverse_range_coding(data: &[u8]) -> Vec<u8> {
    let mut d = ari::ByteDecoder::new(BufReader::new(&data[..]));
    let mut decoded = Vec::new();
    d.read_to_end(&mut decoded).unwrap();
    decoded
}

fn apply_rle(data: &[u8]) -> Vec<u8> {
    let mut encoder = rle::Encoder::new(Vec::new());
    encoder.write_all(&data[..]).unwrap();
    let (buf, _): (Vec<u8>, _) = encoder.finish();
    buf
}

fn reverse_rle(data: &[u8]) -> Vec<u8> {
    let mut decoder = rle::Decoder::new(&data[..]);
    let mut decoder_buf = Vec::new();
    decoder.read_to_end(&mut decoder_buf).unwrap();
    decoder_buf
}

pub fn mtf(matches: &clap::ArgMatches) {
    let ifile = String::from(matches.value_of("input").unwrap());
    let data = read_u32(&ifile);
    let mut compressed : HashMap<String, usize> = HashMap::new();

    // base data
    let mut base : HashMap<String, Vec<u8>> = HashMap::new();
    base.insert("foc".to_string(), get_foc(&data));
    base.insert("lzc".to_string(), get_lzc(&data));
    base.insert("lzcfoc".to_string(), get_lzc_and_foc(&data));
    base.insert("foc_bwt".to_string(), apply_bwt(&base["foc"]));
    base.insert("lzc_bwt".to_string(), apply_bwt(&base["lzc"]));
    base.insert("lzcfoc_bwt".to_string(), apply_bwt(&base["lzcfoc"]));
    base.insert("foc_bwt_mtf".to_string(), apply_mtf(&base["foc_bwt"]));
    base.insert("lzc_bwt_mtf".to_string(), apply_mtf(&base["lzc_bwt"]));
    base.insert("lzcfoc_bwt_mtf".to_string(), apply_mtf(&base["lzcfoc_bwt"]));
    base.insert("foc_bwt_mtf_rle".to_string(), apply_rle(&base["foc_bwt_mtf"]));
    base.insert("lzc_bwt_mtf_rle".to_string(), apply_rle(&base["lzc_bwt_mtf"]));
    base.insert("lzcfoc_bwt_mtf_rle".to_string(), apply_rle(&base["lzcfoc_bwt_mtf"]));
    base.insert("foc_mtf".to_string(), apply_mtf(&base["foc"]));
    base.insert("lzc_mtf".to_string(), apply_mtf(&base["lzc"]));
    base.insert("lzcfoc_mtf".to_string(), apply_mtf(&base["lzcfoc"]));
    base.insert("foc_mtf_rle".to_string(), apply_rle(&base["foc_mtf"]));
    base.insert("lzc_mtf_rle".to_string(), apply_rle(&base["lzc_mtf"]));
    base.insert("lzcfoc_mtf_rle".to_string(), apply_rle(&base["lzcfoc_mtf"]));

    // Huffman compression
    for k in base.iter() {
        let c = encode_itself_to_bytes(&k.1).0.len();
        let dc = encode_itself_to_bytes(&vec_diff(k.1)).0.len();
        let mut name = k.0.clone(); name.push_str("_huff");
        compressed.insert(name, c);
        let mut name = k.0.clone(); name.push_str("_diff_huff");
        compressed.insert(name, dc);
    }

    // Range Encoding
    for k in base.iter() {
        let c = apply_range_coding(&k.1).len();
        let mut name = k.0.clone(); name.push_str("_range");
        compressed.insert(name, c);
        let dc = apply_range_coding(&vec_diff(k.1)).len();
        let mut name = k.0.clone(); name.push_str("_diff_range");
        compressed.insert(name, dc);
    }

    // Output
    let mut count_vec: Vec<_> = compressed.iter().collect();
    count_vec.sort_by(|a, b| a.1.cmp(b.1));
    println!("# {}", ifile);
    for v in count_vec.iter() {
        println!("{}, {}", v.0, v.1);
    }
}

pub fn get_foc(data: &[u32]) -> Vec<u8> {
    data.iter().filter(|&&x| x != 0).map(|&x| _foc(x)).collect::<Vec<u8>>()
}

fn get_fzc(data: &[u32]) -> Vec<u8> {
    data.iter().filter(|&&x| x != 0).map(|&x| _fzc(x)).collect::<Vec<u8>>()
}

pub fn get_lzc(data: &[u32]) -> Vec<u8> {
    data.iter()
        .map(|&x| x.leading_zeros() as u8)
        .collect::<Vec<u8>>()
}

pub fn get_lzc_and_foc(data: &[u32]) -> Vec<u8> {
    data.iter()
        .map(|&x| {
            let gf = _foc(x);
            if gf > 0 {
                (x.leading_zeros() + 33 + u32::from(_foc(x))) as u8
            } else {
                x.leading_zeros() as u8
            }
        })
        .collect::<Vec<u8>>()
}

fn get_lzc_and_fzc(data: &[u32]) -> Vec<u8> {
    data.iter()
        .map(|&x| {
            let gf = _fzc(x);
            if gf > 0 {
                (x.leading_zeros() + 33 + u32::from(_fzc(x))) as u8
            } else {
                x.leading_zeros() as u8
            }
        })
        .collect::<Vec<u8>>()
}

fn _foc(val: u32) -> u8 {
    let mut result = 32 - val.leading_zeros();
    let mut ix = 0;
    while result > 0 && !((val >> ix) + 1).is_power_of_two() {
        ix += 1;
        result -= 1
    }
    result as u8
}

fn _fzc(val: u32) -> u8 {
    if val == 0 {
        return 0;
    }
    let mut value = val;
    let mut ix = 0u8;
    while !value.is_power_of_two() {
        ix += 1;
        value >>= 1
    }
    32 - val.leading_zeros() as u8 - ix - 1
}

#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use super::*;

    #[test]
    fn test_fzc() {
        let data: Vec<u32> = vec![32, 5345, 0, 21321, 0696, 3837, 1 << 31, 1 << 3, 9283];
        let result: Vec<u8> = data.iter().map(|&x| _fzc(x)).collect();
        let expected: Vec<u8> = vec![5, 1, 0, 1, 1, 0, 31, 3, 2];

        assert_eq!(result, expected);
    }

    #[test]
    fn test_range_encoding() {
        let data = "This is a test".as_bytes().to_vec();
        let result = reverse_range_coding(&apply_range_coding(&data));

        assert_eq!(data, result)
    }

    #[test]
    fn test_rle_encoding() {
        let data = "This is a test".as_bytes().to_vec();
        let result = reverse_rle(&apply_rle(&data));

        assert_eq!(data, result)
    }

    #[test]
    fn test_mtf_encoding() {
        let data = "This is a test".as_bytes().to_vec();
        let result = reverse_mtf(&apply_mtf(&data));

        assert_eq!(data, result)
    }

    #[test]
    fn test_bwt_encoding() {
        let data = "This is a test".as_bytes().to_vec();
        let result = reverse_bwt(&apply_bwt(&data));

        assert_eq!(data, result)
    }

    #[test]
    fn test_dc_encoding() {
        let data = "This is a test".as_bytes().to_vec();
        let result = reverse_dc(&apply_dc(&data), data.len());

        assert_eq!(data, result)
    }
}
