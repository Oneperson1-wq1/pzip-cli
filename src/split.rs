//! An application for the encoding phase of the compression algorithm.
//!
//! This application should help identifying and optimizing methods for actual
//! encoding of the data to be compressed.

use super::graycodeanalysis::read_u32;
use bit_vec::BitVec;
use byteorder::{BigEndian, ByteOrder};
use pzip_huffman;

/// Transforms a Vector of u32 to u8 and eliminates of zero values at the end of the Vector.
fn truncate(data: Vec<u32>) -> Vec<u8> {
    let src = &data[..];
    let mut ds: Vec<u8> = vec![0; 4 * data.len()];
    BigEndian::write_u32_into(src, &mut ds[..]);
    // TODO: Why do I need to use BigEndian? Are the other implementations using LittleEndian wrong?

    let mut last_element;
    loop {
        last_element = ds.pop().unwrap();
        if last_element != 0 {
            break;
        }
    }
    ds.push(last_element);
    ds
}

/// Split truth and prediction datasets to LZC, FZ and Residual datasets.
///
/// 1. Reads two separate files: Truth file and predictions file.
/// 2. Calculates the LZC, FZ and Residual (via Difference) of both files.
/// 3. Encodes LZC & FZ via Huffman Codes and via Arithmetic Encoder
/// 4. Prints "outbytes" with Huff(LZC) + Huff(FZ) + Residuals
pub fn split(matches: &clap::ArgMatches) {
    let pfile = String::from(matches.value_of("prediction").unwrap());
    let tfile = String::from(matches.value_of("truth").unwrap());

    let predictions = read_u32(&pfile);
    let truth = read_u32(&tfile);

    let lzc: Vec<u8> = predictions
        .iter()
        .zip(truth.iter())
        .map(|(&p, &t)| (p ^ t).leading_zeros() as u8)
        .collect();
    let lzc_encoded = pzip_huffman::hufbites::encode_itself_to_bytes(&lzc);
    let arlzc_encoded = pzip_redux::encode(&lzc, 8, 10, 12);

    let fz: Vec<u8> = predictions
        .iter()
        .zip(truth.iter())
        .map(|(&p, &t)| (p.max(t) - p.min(t)).leading_zeros() as u8)
        .zip(lzc.iter())
        .filter(|(_d, &xor)| xor != 32) // or where d != 0 (only accept values where LZC != 32 )
        .map(|(d, &xor)| d - xor)
        .collect();
    let fz_encoded = pzip_huffman::hufbites::encode_itself_to_bytes(&fz);
    let arfz_encoded = pzip_redux::encode(&fz, 8, 10, 12);

    let diff: Vec<u32> = predictions
        .iter()
        .zip(truth.iter())
        .map(|(&p, &t)| p.max(t) - p.min(t))
        .collect();
    let compact_residuals = to_u8(pack(&diff, true));


    // Follwing is just output formatting

    let nbytes = lzc.len() + fz.len() + compact_residuals.len();
    let onbytes = predictions.len() * 4;

    println!(
        "{} + {} + {} = {} of {} ({}% | {:.2})",
        lzc.len(),
        fz.len(),
        compact_residuals.len(),
        nbytes,
        onbytes,
        nbytes as f64 / onbytes as f64,
        onbytes as f64 / nbytes as f64
    );

    let cnbytes = lzc_encoded.len() + fz_encoded.len() + compact_residuals.len();
    let conbytes = predictions.len() * 4;
    println!(
        "{} + {} + {} = {} of {} ({}% | {:.2})",
        lzc_encoded.len(),
        fz_encoded.len(),
        compact_residuals.len(),
        cnbytes,
        conbytes,
        cnbytes as f64 / conbytes as f64,
        conbytes as f64 / cnbytes as f64,
    );

    let arcnbytes = arlzc_encoded.len() + arfz_encoded.len() + compact_residuals.len();
    let arconbytes = predictions.len() * 4;
    println!(
        "{} + {} + {} = {} of {} ({}% | {:.2})",
        arlzc_encoded.len(),
        arfz_encoded.len(),
        compact_residuals.len(),
        arcnbytes,
        arconbytes,
        arcnbytes as f64 / arconbytes as f64,
        arconbytes as f64 / arcnbytes as f64,
    );
}

/// Transforms BitVec into a Vector of u8 values with padding on the last element
fn to_u8(bv: BitVec) -> Vec<u8> {
    bv.to_bytes()
}

/// Transforms BitVec into a Vector of u32 values with padding on the last element
fn to_u32(bv: BitVec) -> Vec<u32> {
    let data = bv.to_bytes();
    let mut result : Vec<u32> = vec![0; data.len() / 4];
    BigEndian::read_u32_into(&data, &mut result);
    result
}

/// Packing of bits
/// All the set/unset bits of a vector are being squashed/packed together to represent the
/// most minimal representation of the data. The `skip` flag defines if the
/// first value (which will always be set) should be included or not.
fn pack(data: &Vec<u32>, skip: bool) -> BitVec {
    let mut result = BitVec::new();
    result.push(true);  // necessary for cases where the first value in the following is a false

    for value in data.iter() {
        let mut next = value.next_power_of_two() >> 1 + skip as usize;
        while next != 0 {
            result.push(next & value > 0);
            next >>= 1;
        }
    }
    result
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_truncate_numbers() {
        let data: Vec<u32> = vec![164787381, 1 << 30, 4036830976, 3778694784];
        let expected_length = vec![4, 1, 3, 4];

        for (val, exp) in data.into_iter().zip(expected_length.into_iter()) {
            println!("{}", val);
            let single: Vec<u32> = vec![val];
            let result = truncate(single);
            println!("{:032b}", val);
            for s in result.iter() {
                print!("{:08b}", s);
            }
            print!("\n\n");
            assert_eq!(result.len(), exp);
        }
    }

    #[test]
    fn test_to_bitvec() {
        let data: Vec<u32> = vec![0b101010_01001001_01111101_11010111]; //
        let result = pack(&data, false);

        assert_eq!(to_u8(result), vec![169, 37, 247, 92]);
    }

    #[test]
    fn test_to_bitvec_skipped() {
        let data: Vec<u32> = vec![0b101010_01001001_01111101_11010111]; //
        let result = pack(&data, true);

        assert_eq!(to_u8(result), vec![82, 75, 238, 92 << 1]);
    }

    #[test]
    fn test_bitvec_to_u32() {
        let data: Vec<u32> = vec![62736423];
        let lz = data[0].leading_zeros();
        let result = pack(&data, false);
        println!("{:?}", result);
        let result = to_u32(result);

        for r in result.iter() {
            println!("{:#032b}", r);
        }

        assert_eq!(result[0], data[0] << lz);  // bitvec will fill the values with zeros
    }

    #[test]
    fn test_bitvec_to_u32_skipped() {
        let data: Vec<u32> = vec![62736423];
        let lz = data[0].leading_zeros() + 1;  // because first is skipped
        let result = pack(&data, true);
        println!("{:?}", result);
        let result = to_u32(result);

        for r in result.iter() {
            println!("{:#032b}", r);
        }

        assert_eq!(result[0], data[0] << lz);  // bitvec will fill the values with zeros
    }
}
