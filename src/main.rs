use std::{
  io::{stdout, Write},
  process::exit,
  sync::mpsc::channel,
  thread::sleep,
  time::{Duration, Instant},
};

mod terminal;

const BAR_UPDATE_INTERVAL: u128 = 16; // milliseconds
const BAR_EMPTY_CHAR: char = '▒';
const BAR_FULL_CHAR: char = '█';

fn main() {
  // encourage control characters on Windows (https://learn.microsoft.com/en-us/windows/console/console-virtual-terminal-sequences)
  #[cfg(target_os = "windows")]
  {
    use windows::Win32::System::Console::{GetConsoleMode, GetStdHandle, SetConsoleMode, CONSOLE_MODE, ENABLE_VIRTUAL_TERMINAL_PROCESSING, STD_OUTPUT_HANDLE};

    unsafe {
      let handle = GetStdHandle(STD_OUTPUT_HANDLE).expect("Failed to get stdout");
      let mut mode: CONSOLE_MODE = CONSOLE_MODE(0);

      GetConsoleMode(handle, &mut mode).expect("Failed to get console mode");

      if !mode.contains(ENABLE_VIRTUAL_TERMINAL_PROCESSING) {
        SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING).expect("Failed to set console mode");
      }
    }
  }

  let args = std::env::args().collect::<Vec<String>>();
  let args = args.split_at(1).1; // remove self from args list

  if args.is_empty() {
    print_help();
    return;
  }

  let mut duration = None;

  for arg in args {
    match arg.as_str() {
      "-v" | "--version" => {
        println!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return;
      }
      "-h" | "--help" => {
        print_help();
        return;
      }
      _ => {
        // first generic argument is duration, any after that causing the phone program to error
        if duration.is_none() {
          duration = Some(arg);
        } else {
          eprintln!("Unknown option: {}", arg);
          println!("Use '{} --help' for more information", env!("CARGO_PKG_NAME"));

          exit(1);
        }
      }
    }
  }

  if duration.is_none() {
    eprintln!("No duration specified");
    exit(1);
  }

  let duration = duration.unwrap();

  let mut seconds = 0;
  let mut current_number = String::new(); // temporary buffer to store the currently parsing number

  for character in duration.chars() {
    match character {
      // take our current buffer and store it as seconds
      's' => {
        if current_number.is_empty() {
          eprintln!("No number found before seconds");
          exit(1);
        }

        seconds += current_number.parse::<u64>().unwrap();
        current_number = String::new();
      }
      'm' => {
        if current_number.is_empty() {
          eprintln!("No number found before minutes");
          exit(1);
        }

        seconds += current_number.parse::<u64>().unwrap() * 60;
        current_number = String::new();
      }
      'h' => {
        if current_number.is_empty() {
          eprintln!("No number found before hours");
          exit(1);
        }

        seconds += current_number.parse::<u64>().unwrap() * 3600;
        current_number = String::new();
      }

      // append to our buffer
      '0'..='9' => {
        current_number.push(character);
      }

      // invalid character found
      _ => {
        eprintln!("Invalid time!");
        exit(1);
      }
    }
  }

  // if there are any remaining numbers, assume seconds
  if !current_number.is_empty() {
    seconds += current_number.parse::<u64>().unwrap();
  }

  let duration = Duration::from_secs(seconds);

  let start = Instant::now();
  let end = start + duration;

  // setup ctrl+c handler
  let (exit_tx, exit_rx) = channel();
  ctrlc::set_handler(move || exit_tx.send(()).expect("Could not send signal on channel.")).expect("Error setting Ctrl-C handler");

  terminal::set_cursor_visible(false);

  println!(); // create an empty line, as below we will move up and clear it

  let mut last_update = Instant::now();
  loop {
    if exit_rx.try_recv().is_ok() {
      terminal::clear_line();

      terminal::set_cursor_visible(true);
      println!("Exiting early!");

      stdout().flush().unwrap();

      return;
    }

    let now = Instant::now();

    if now > end {
      break;
    }

    if last_update.elapsed().as_millis() < BAR_UPDATE_INTERVAL {
      sleep(Duration::from_millis((BAR_UPDATE_INTERVAL - last_update.elapsed().as_millis()) as u64));
      continue;
    }

    let terminal_width = terminal::get_width() as usize;

    let bar_width = match terminal_width - 15 {
      n if n < 30 => n,
      _ => 30,
    };

    let progress = now.duration_since(start).as_millis() as f64 / duration.as_millis() as f64; // 0-1
    let progress_width = (progress * bar_width as f64).round() as usize;

    let remaining = end - now;
    let seconds = remaining.as_secs() as f64;

    terminal::previous_line();
    terminal::clear_line();

    // print current time (clock)
    print!("{} - ", chrono::Local::now().format("%_I:%M%P").to_string().trim());

    // print hours remaining (if any)
    match (seconds / 3600.0).floor() {
      hours if hours > 0.0 => print!("{}h", hours),
      _ => {}
    };

    // print minutes remaining (if any)
    match ((seconds % 3600.0) / 60.0).floor() {
      minutes if minutes > 0.0 => print!("{}m", minutes),
      _ => {}
    }

    // print seconds remaining
    println!("{}s", (seconds % 60.0).ceil());

    terminal::clear_line();

    // print the solid progress bar
    for i in 0..progress_width {
      let red = lerp(90, 123, i as f64 / bar_width as f64);
      let green = lerp(105, 90, i as f64 / bar_width as f64);

      print!("{}{}", terminal::ansi_rgb(red, green, 237), BAR_FULL_CHAR);
    }

    // print empty progress bar and progress percent
    print!(
      "{}{}{}[39m  {}%",
      terminal::ansi_rgb(100, 100, 100),
      BAR_EMPTY_CHAR.to_string().repeat(bar_width - progress_width),
      27 as char,
      (progress * 100.0).round()
    );

    stdout().flush().unwrap();

    last_update = now;
  }

  terminal::previous_line();
  terminal::clear_line();

  print!("{}", 7 as char); // beep/alert

  terminal::set_cursor_visible(true);

  println!("Finished!");

  terminal::clear_line();
}

fn print_help() {
  println!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
  println!("Usage: {} [options]", env!("CARGO_PKG_NAME"));
  println!();
  println!("Options:");
  println!("  duration       Start a timer for duration");
  println!("  -v, --version  Print version information");
  println!("  -h, --help     Print this help message");
}

fn lerp(a: u8, b: u8, t: f64) -> u8 {
  ((1.0 - t) * (a as f64) + t * (b as f64)).round() as u8
}
