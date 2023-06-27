use hound::WavReader;
use std::io::BufReader;

fn load_wav(buffer: &[u8]) -> Vec<f32> {
    let mut reader = WavReader::new(BufReader::new(buffer)).unwrap();
    let spec = reader.spec();
    let max_value = (1 << (spec.bits_per_sample - 1)) as i64;
    let data_int = reader
        .samples::<i32>()
        .map(|x| x.unwrap())
        .collect::<Vec<_>>();
    let data_f32 = data_int
        .chunks(spec.channels as usize)
        .map(|chunk| {
            let channel_mean = chunk.iter().map(|&x| x as i64).sum::<i64>() / chunk.len() as i64;
            (channel_mean as f64 / max_value as f64) as f32
        })
        .collect::<Vec<_>>();
    data_f32
}

pub fn sn01() -> Vec<f32> {
    load_wav(include_bytes!("./sn01.wav"))
}

pub fn bd01() -> Vec<f32> {
    load_wav(include_bytes!("./bd01.wav"))
}

pub fn ch01() -> Vec<f32> {
    load_wav(include_bytes!("./ch01.wav"))
}
