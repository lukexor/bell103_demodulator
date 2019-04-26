use hound;
use std::f64::consts::PI;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use structopt::StructOpt;

const ORIG_MARK_FREQUENCY: f64 = 1270.0;
const ORIG_SPACE_FREQUENCY: f64 = 1070.0;
const ANS_MARK_FREQUENCY: f64 = 2225.0;
const ANS_SPACE_FREQUENCY: f64 = 2025.0;

#[derive(StructOpt, Debug)]
#[structopt(
    name = "bell103_demodulator",
    about = "Decodes messages made using the Bell 103 modem protocol using a Goertzel filter",
    version = "0.1.0",
    author = "Luke Petherbridge <me@lukeworks.tech>"
)]
struct Opt {
    #[structopt(parse(from_os_str), help = "The PCM WAV file to be decoded")]
    file: PathBuf,
    #[structopt(parse(from_os_str), help = "The output file to store the message")]
    output: Option<PathBuf>,
    #[structopt(
        short = "s",
        long = "sampling_rate",
        default_value = "48000",
        help = "Audio sampling rate"
    )]
    sampling_rate: f64,
    #[structopt(
        short = "l",
        long = "filter_length",
        default_value = "160",
        help = "Goertzel filter length N"
    )]
    filter_length: usize,
    #[structopt(
        short = "o",
        long = "origin",
        help = "Use originating mark/space frequencies (default uses answering frequencies"
    )]
    origin: bool,
}

fn main() {
    let opt = Opt::from_args();
    decode_file(opt);
}

fn decode_file(opt: Opt) {
    let (mark_frequency, space_frequency) = if opt.origin {
        (ORIG_MARK_FREQUENCY, ORIG_SPACE_FREQUENCY)
    } else {
        (ANS_MARK_FREQUENCY, ANS_SPACE_FREQUENCY)
    };
    // Create two filters for mark and space frequencies
    let mut mark = GoertzelFilter::new(opt.filter_length, mark_frequency, opt.sampling_rate);
    let mut space = GoertzelFilter::new(opt.filter_length, space_frequency, opt.sampling_rate);

    // Read our sample data
    let file = File::open(opt.file).unwrap();
    let mut reader = hound::WavReader::new(file).unwrap();
    let samples: Vec<i16> = reader.samples::<i16>().map(Result::unwrap).collect();

    // Loop in chunks over our sample, applying our filters and building a list of bits
    let mut bits: Vec<u8> = Vec::with_capacity(samples.len() / opt.filter_length);
    for chunk in samples.chunks(opt.filter_length) {
        mark.process(chunk);
        space.process(chunk);
        let bit = if mark.get_mag_sq() >= space.get_mag_sq() {
            1
        } else {
            0
        };
        bits.push(bit);
        mark.reset();
        space.reset();
    }

    // Loop over chunks of 10 bits to create char bytes for our decoded message
    let mut message = String::new();
    for chunk in bits.chunks(10) {
        if chunk[0] == 0 && chunk[9] == 1 {
            let int = chunk[1..8]
                .iter()
                .rev()
                .fold(0, |acc, &b| (acc << 1) | u32::from(b));
            let char = std::char::from_u32(int).unwrap();
            message.push(char);
        }
    }

    // Print and save our message
    if let Some(file) = opt.output {
        let mut file = std::fs::File::create(file).unwrap();
        file.write_all(message.as_bytes()).unwrap();
    } else {
        println!("{}", message);
    }
}

#[derive(Debug)]
struct GoertzelFilter {
    k: u32,
    n: usize,
    coeff: f64,
    q1: f64,
    q2: f64,
    sin: f64,
    cos: f64,
}

impl GoertzelFilter {
    fn new(block_size: usize, target_freq: f64, sampling_rate: f64) -> Self {
        let k = (block_size as f64 * target_freq) / sampling_rate;
        let omega = (2.0 * PI * k as f64) / block_size as f64;
        let cos = omega.cos();
        Self {
            k: k as u32,
            n: block_size,
            coeff: 2.0 * cos,
            q1: 0.0,
            q2: 0.0,
            sin: omega.sin(),
            cos,
        }
    }

    fn process(&mut self, samples: &[i16]) {
        for v in samples {
            let q0 = self.coeff * self.q1 - self.q2 + f64::from(*v);
            self.q2 = self.q1;
            self.q1 = q0;
        }
    }

    #[allow(unused)]
    fn get_real_imag(&self) -> (f64, f64) {
        let real = self.q1 - self.q2 * self.cos;
        let imag = self.q2 * self.sin;
        (real, imag)
    }

    fn get_mag_sq(&self) -> f64 {
        self.q1 * self.q1 + self.q2 * self.q2 - self.q1 * self.q2 * self.coeff
    }

    fn reset(&mut self) {
        self.q2 = 0.0;
        self.q1 = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLING_RATE: f64 = 8_000.0; // 8 kHz
    const BLOCK_SIZE: usize = 205;
    const TARGET_FREQUENCY: f64 = 941.0; // 941 Hz

    fn generate_test_samples(frequency: f64) -> Vec<u8> {
        let step = frequency * 2.0 * PI / SAMPLING_RATE;
        let mut samples = vec![0u8; BLOCK_SIZE];
        for i in 0..BLOCK_SIZE {
            samples[i] = (100.0 * (i as f64 * step).sin() + 100.0) as u8;
        }
        samples
    }

    fn run_test(filter: &mut GoertzelFilter, frequency: f64) {
        eprintln!("For test frequency {:.6}:", frequency);

        let samples = generate_test_samples(frequency);
        let samples: Vec<i16> = samples.iter().map(|s| *s as i16).collect();
        filter.process(&samples);

        let (real, imag) = filter.get_real_imag();
        eprintln!("real = {:.6} imag = {:.6}", real, imag);

        let mag_sq = real * real + imag * imag;
        eprintln!("Relative magnitude squared = {:.6}", mag_sq);
        eprintln!("Relative magnitude = {:.6}", mag_sq.sqrt());

        eprintln!("Relative magnitude squared = {:.6}", filter.get_mag_sq());
        eprintln!("Relative magnitude = {:.6}\n", filter.get_mag_sq().sqrt());
    }

    #[test]
    fn test_goertzel_filter_target() {
        let mut filter = GoertzelFilter::new(BLOCK_SIZE, TARGET_FREQUENCY, SAMPLING_RATE);
        eprint!("\nFor SAMPLING_RATE = {:.6}", SAMPLING_RATE);
        eprint!(" N = {}", BLOCK_SIZE);
        eprintln!(" and FREQUENCY = {:.6},", TARGET_FREQUENCY);
        eprintln!("k = {} and coeff = {:.6}\n", filter.k, filter.coeff);

        run_test(&mut filter, TARGET_FREQUENCY - 250.0);
        let (real, imag) = filter.get_real_imag();
        assert_eq!(real.floor(), -316.0);
        assert_eq!(imag.floor(), -187.0);
        assert_eq!(filter.get_mag_sq().floor(), 134338.0);
        filter.reset();

        run_test(&mut filter, TARGET_FREQUENCY);
        let (real, imag) = filter.get_real_imag();
        assert_eq!(real.floor(), -191.0);
        assert_eq!(imag.floor(), -10196.0);
        assert_eq!(filter.get_mag_sq().floor(), 103981719.0);
        filter.reset();

        run_test(&mut filter, TARGET_FREQUENCY + 250.0);
        let (real, imag) = filter.get_real_imag();
        assert_eq!(real.floor(), 596.0);
        assert_eq!(imag.floor(), -177.0);
        assert_eq!(filter.get_mag_sq().floor(), 387565.0);
        filter.reset();
    }

    #[test]
    fn test_goertzel_filter_sweep() {
        let mut filter = GoertzelFilter::new(BLOCK_SIZE, TARGET_FREQUENCY, SAMPLING_RATE);
        let mut freq = TARGET_FREQUENCY - 300.0;
        let end = TARGET_FREQUENCY + 300.0;
        while freq <= end {
            eprint!("Freq={:7.1}   ", freq);

            let samples = generate_test_samples(freq);
            let samples: Vec<i16> = samples.iter().map(|s| *s as i16).collect();
            filter.process(&samples);

            let (real, imag) = filter.get_real_imag();
            let mag_sq = real * real + imag * imag;
            eprint!("rel mag^2={:16.5}   ", mag_sq);
            eprintln!("rel mag={:12.5}", mag_sq.sqrt());

            freq += 15.0;
            filter.reset();
        }
    }
}
