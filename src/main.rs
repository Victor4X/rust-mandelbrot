use num::{Complex};
use std::str::FromStr;
use std::env;

use image::ColorType;
use image::png::PngEncoder;
use std::fs::File;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 6 {
        eprintln!("Usage: {} FILE PIXELS SEPARATOR UPPERLEFT LOWERRIGHT", args[0]);
        eprintln!("Example: {} mandel.png 1000x750 -1.20,0.35 -1,0.20", args[0]);
        std::process::exit(1);
    }
    let bounds= parse_pair(&args[2], char::from_str(&args[3]).expect("Seperator conversion failed")).expect("Parsing of image dimensions failed with given arguments");
    let upper_left = parse_complex(&args[4]).expect("Parsing of upper left complex number failed");
    let lower_right = parse_complex(&args[5]).expect("Parsing of lower right complex number failed");

    let mut pixels = vec![0; bounds.0 * bounds.1];
    
    render(&mut pixels, bounds, upper_left, lower_right);

    write_image(&args[1], &pixels, bounds).expect("Image writing failed");
}

// Write pixel buffer to file
fn write_image(filename: &str, pixels: &[u8], bounds: (usize, usize)) -> Result<(), std::io::Error> {
    let output = File::create(filename)?;

    let encoder = PngEncoder::new(output);
    match encoder.encode(&pixels, bounds.0 as u32, bounds.1 as u32, ColorType::L8) {
        Ok(()) => (),
        Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) // This seems scuffed. TODO: Figure out a better way
    }; // L8 is 8 bit luminence
    Ok(()) // Can error out through the two ? but otherwise return OK(()) -- Ok with a unit
}

// Render a portion of the mandelbrot set into a given buffer of pixels
fn render(pixels: &mut [u8], bounds: (usize, usize), upper_left: Complex<f64>, lower_right: Complex<f64>) {
    assert!(pixels.len() == bounds.0 * bounds.1); // Make sure bounds correctly define amount of pixels

    for row in 0..bounds.1 {
        for column in 0..bounds.0 {
            let point = pixel_to_point(bounds, (column, row), upper_left, lower_right);
            pixels[row * bounds.0 + column] = match escape_time(point, 255) {
                None => 0,
                Some(count) => 255 - count as u8
            };
        }
    }
}

// Parse string to coordinates
fn parse_pair<T: FromStr>(s: &str, separator: char) -> Option<(T, T)> {
    match s.find(separator) { // Find the separator location
        None => None,
        Some(index) => {
            match (T::from_str(&s[..index]), T::from_str(&s[index + 1..])) { // Match on tuple
                (Ok(l), Ok(r)) => Some((l, r)),
                _ => None
            }
        }
    }
}

// Parse string to complex number ex: 1.03,2.58 -> Complex<f64> {re: 1.03, im: 2.58}
fn parse_complex(s: &str) -> Option<Complex<f64>> {
    match parse_pair(s, ',') {
        Some((re, im)) => Some(Complex { re, im }),
        None => None
    }
}

#[test]
fn test_parse_pair() {
    assert_eq!(parse_pair::<i32>("", ','), None);
    assert_eq!(parse_pair::<i32>("10,", ','), None);
    assert_eq!(parse_pair::<i32>(",10", ','), None);
    assert_eq!(parse_pair::<i32>("10,20", ','), Some((10, 20)));
}

// Function for mapping a given pixel position in a given image size to a point on the complex plane within two given complex points
fn pixel_to_point(bounds: (usize, usize), pixel: (usize,usize), upper_left: Complex<f64>, lower_right: Complex<f64>) -> Complex<f64> {
    // Calculate (width, height) on complex plane
    let (width, height) = (lower_right.re - upper_left.re, upper_left.im - lower_right.im);

    Complex {
        re: upper_left.re + pixel.0 as f64 * width / bounds.0 as f64,
        im: upper_left.im - pixel.1 as f64 * height / bounds.1 as f64 // Negative to flip the reversed axis in the pixel world
    }
}

#[test]
fn test_pixel_to_point() {
    assert_eq!(pixel_to_point((100, 200), (25, 175), Complex {re: -1.0, im: 1.0}, Complex {re: 1.0, im: -1.0}), Complex { re: -0.5, im: -0.75})
}

// Function for determining the mandelbrot set escape time of a given point on the complex plane
fn escape_time(c: Complex<f64>, limit: usize) -> Option<usize> {
    let mut z = Complex {re: 0.0, im: 0.0};
    for i in 0..limit {
        if z.norm_sqr() > 4.0 { // Square of the distance to the origin of the complex plane
            return Some(i);
        }
        z = z * z + c;
    }

    None
}