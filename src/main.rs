use num::Complex;
use std::env;
use std::str::FromStr;

use crossbeam;

use image::png::PngEncoder;
use image::ColorType;
use std::fs::File;

use log::{debug, error};

use pixels::{Error, Pixels, SurfaceTexture};
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalSize};
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit_input_helper::WinitInputHelper;

const SCREEN_WIDTH: usize = 1000;
const SCREEN_HEIGHT: usize = 1000;

fn main() -> Result<(), Error> {
    // Argument parsing here. (Custom window size) TODO: Better argument handling
    // let args: Vec<String> = env::args().collect();
    // if args.len() != 6 {
    //     eprintln!("Usage: {} FILE PIXELS SEPARATOR UPPERLEFT LOWERRIGHT", args[0]);
    //     eprintln!("Example: {} mandel.png 1000x750 x -1.20,0.35 -1,0.20", args[0]);
    //     std::process::exit(1);
    // }

    // let bounds= parse_pair(&args[2], char::from_str(&args[3]).expect("Seperator conversion failed")).expect("Parsing of image dimensions failed with given arguments");
    // let upper_left = parse_complex(&args[4]).expect("Parsing of upper left complex number failed");
    // let lower_right = parse_complex(&args[5]).expect("Parsing of lower right complex number failed");

    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let window = create_window("Mandelbrot Explorer", &event_loop);

    let bounds = (SCREEN_WIDTH, SCREEN_HEIGHT);
    let mut upper_left = Complex { re: -1.0, im: 1.0 };
    let mut lower_right = Complex { re: 1.0, im: -1.0 };

    //let mut pixels = vec![0; bounds.0 * bounds.1];

    let surface_texture = SurfaceTexture::new(SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32, &window);
    let mut pixels = Pixels::new(SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32, surface_texture)
        .expect("Pixels failed to initialize");

    println!("{}", pixels.get_frame().len()); // Make sure bounds correctly define amount of pixels

    render_multi(pixels.get_frame(), bounds, upper_left, lower_right);

    event_loop.run(move |event, _, control_flow| {
        // The one and only event that winit_input_helper doesn't have for us...
        if let Event::RedrawRequested(_) = event {
            render_multi(pixels.get_frame(), bounds, upper_left, lower_right);
            if pixels
                .render()
                .map_err(|e| error!("pixels.render() failed: {}", e)) // I probably broke this :P
                .is_err()
            {
                *control_flow = ControlFlow::Exit;
                return;
            }
        }

        // For everything else, for let winit_input_helper collect events to build its state.
        // It returns `true` when it is time to update our game state and request a redraw.
        if input.update(&event) {
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            }
            if input.key_pressed(VirtualKeyCode::W) {
                let displacement = Complex {
                    re: 0.0 * (upper_left.re - lower_right.re),
                    im: 0.05 * (upper_left.im - lower_right.im),
                };

                upper_left += displacement;
                lower_right += displacement;
                window.request_redraw();
            }
            if input.key_pressed(VirtualKeyCode::A) {
                let displacement = Complex {
                    re: 0.05 * (upper_left.re - lower_right.re),
                    im: 0.0 * (upper_left.im - lower_right.im),
                };

                upper_left += displacement;
                lower_right += displacement;
                window.request_redraw();
            }
            if input.key_pressed(VirtualKeyCode::S) {
                let displacement = Complex {
                    re: 0.0 * (upper_left.re - lower_right.re),
                    im: -0.05 * (upper_left.im - lower_right.im),
                };

                upper_left += displacement;
                lower_right += displacement;
                window.request_redraw();
            }
            if input.key_pressed(VirtualKeyCode::D) {
                let displacement = Complex {
                    re: -0.05 * (upper_left.re - lower_right.re),
                    im: 0.0 * (upper_left.im - lower_right.im),
                };
                
                upper_left += displacement;
                lower_right += displacement;
                window.request_redraw();
            }

            // Zooming
            if input.key_pressed(VirtualKeyCode::Z) {
                let scalar = 0.10;
                
                upper_left -= scalar*(upper_left-lower_right)/2.0;
                lower_right += scalar*(upper_left-lower_right)/2.0;
                window.request_redraw();
            }
            if input.key_pressed(VirtualKeyCode::X) {
                let scalar = 0.10;
                
                upper_left += scalar*(upper_left-lower_right)/2.0;
                lower_right -= scalar*(upper_left-lower_right)/2.0;
                window.request_redraw();
            }

            // Resetting
            if input.key_pressed(VirtualKeyCode::Space) {
                upper_left = Complex { re: -1.0, im: 1.0 };
                lower_right = Complex { re: 1.0, im: -1.0 };
                window.request_redraw();
            }
        }
    });
    //write_image(&args[1], &pixels, bounds).expect("Image writing failed"); TODO: Use somewhere else
}

// Multithreaded render
fn render_multi(
    pixels: &mut [u8],
    bounds: (usize, usize),
    upper_left: Complex<f64>,
    lower_right: Complex<f64>,
) {
    println!(
        "Rendering between {},{} and {},{}",
        upper_left.re, upper_left.im, lower_right.re, lower_right.im
    );

    // Multithreading stuff here
    let threads = 16; // Higher number = More speed
    let rows_per_band = bounds.1 / threads + 1;

    let bands: Vec<&mut [u8]> = pixels.chunks_mut(rows_per_band * bounds.0 * 4).collect();
    crossbeam::scope(|spawner| {
        for (i, band) in bands.into_iter().enumerate() {
            let top = rows_per_band * i;
            let height = band.len() / 4 / bounds.0;
            let band_bounds = (bounds.0, height);
            let band_upper_left = pixel_to_point(bounds, (0, top), upper_left, lower_right);
            let band_lower_right =
                pixel_to_point(bounds, (bounds.0, top + height), upper_left, lower_right);

            spawner.spawn(move |_| {
                render(band, band_bounds, band_upper_left, band_lower_right);
            });
        }
    })
    .unwrap();
}

// Render a portion of the mandelbrot set into a given buffer of pixels
fn render(
    pixels: &mut [u8],
    bounds: (usize, usize),
    upper_left: Complex<f64>,
    lower_right: Complex<f64>,
) {
    assert!(pixels.len() == bounds.0 * bounds.1 * 4); // Make sure bounds correctly define amount of pixels

    for row in 0..bounds.1 {
        for column in 0..bounds.0 {
            let point = pixel_to_point(bounds, (column, row), upper_left, lower_right);
            let point_shade = match escape_time(point, 255) {
                None => 0,
                Some(count) => 255 - count as u8,
            };

            let pixel_color = [0, point_shade, point_shade, point_shade];

            let pixel_start = (row * bounds.0 + column) * 4;

            pixels[pixel_start..pixel_start + 4].copy_from_slice(&pixel_color)
        }
    }
}

// Create the application window
fn create_window(title: &str, event_loop: &EventLoop<()>) -> winit::window::Window {
    // Create a hidden window so we can estimate a good default window size
    let window = winit::window::WindowBuilder::new()
        .with_visible(true)
        .with_title(title)
        .with_inner_size(LogicalSize::new(1000, 1000))
        .build(event_loop)
        .unwrap();

    window
}

// Function for mapping a given pixel position in a given image size to a point on the complex plane within two given complex points
fn pixel_to_point(
    bounds: (usize, usize),
    pixel: (usize, usize),
    upper_left: Complex<f64>,
    lower_right: Complex<f64>,
) -> Complex<f64> {
    // Calculate (width, height) on complex plane
    let (width, height) = (
        lower_right.re - upper_left.re,
        upper_left.im - lower_right.im,
    );

    Complex {
        re: upper_left.re + pixel.0 as f64 * width / bounds.0 as f64,
        im: upper_left.im - pixel.1 as f64 * height / bounds.1 as f64, // Negative to flip the reversed axis in the pixel world
    }
}

#[test]
fn test_pixel_to_point() {
    assert_eq!(
        pixel_to_point(
            (100, 200),
            (25, 175),
            Complex { re: -1.0, im: 1.0 },
            Complex { re: 1.0, im: -1.0 }
        ),
        Complex {
            re: -0.5,
            im: -0.75
        }
    )
}

// Function for determining the mandelbrot set escape time of a given point on the complex plane
fn escape_time(c: Complex<f64>, limit: usize) -> Option<usize> {
    let mut z = Complex { re: 0.0, im: 0.0 };
    for i in 0..limit {
        if z.norm_sqr() > 4.0 {
            // Square of the distance to the origin of the complex plane
            return Some(i);
        }
        z = z * z + c;
    }

    None
}
