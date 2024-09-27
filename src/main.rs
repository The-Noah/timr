use std::{
  io::{stdout, Write},
  process::exit,
  sync::mpsc::channel,
  thread::sleep,
  time::{Duration, Instant},
};

const BAR_UPDATE_INTERVAL: u128 = 16; // milliseconds
const BAR_EMPTY_CHAR: char = '▒';
const BAR_FULL_CHAR: char = '█';

fn main() {
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

  println!();

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

  let mut last_update = Instant::now();
  loop {
    if exit_rx.try_recv().is_ok() {
      clear_line();

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

    let terminal_width = get_terminal_width() as usize;

    let bar_width = match terminal_width - 15 {
      n if n < 30 => n,
      _ => 30,
    };

    let progress = now.duration_since(start).as_millis() as f64 / duration.as_millis() as f64; // 0-1
    let progress_width = (progress * bar_width as f64).round() as usize;

    // let bar = format!(
    //   "{}{}",
    //   BAR_FULL_CHAR.to_string().repeat(progress_width),
    //   BAR_EMPTY_CHAR.to_string().repeat(bar_width - progress_width)
    // );

    let remaining = end - now;
    let seconds = remaining.as_secs() as f64;

    previous_line();
    clear_line();

    match (seconds / 3600.0).floor() {
      hours if hours > 0.0 => print!("{}h", hours),
      _ => {}
    };

    match ((seconds % 3600.0) / 60.0).floor() {
      minutes if minutes > 0.0 => print!("{}m", minutes),
      _ => {}
    }

    println!("{}s", (seconds % 60.0).ceil());

    clear_line();

    for i in 0..progress_width {
      let red = lerp(90, 123, i as f64 / bar_width as f64);
      let green = lerp(105, 90, i as f64 / bar_width as f64);

      print!("{}{}", ansi_rgb(red, green, 237), BAR_FULL_CHAR);
    }

    print!(
      "{}{}{}[39m {}%",
      ansi_rgb(100, 100, 100),
      BAR_EMPTY_CHAR.to_string().repeat(bar_width - progress_width),
      27 as char,
      (progress * 100.0).round()
    );

    // print!("{}{}{}[39m {}%", ansi_rgb(red, green, 237), bar, 27 as char, (progress * 100.0).round());

    stdout().flush().unwrap();

    last_update = now;
  }

  previous_line();
  clear_line();

  print!("{}", 7 as char); // beep/alert

  println!("Finished!");

  clear_line();
}

/// Move cursor to beginning of the previous line
fn previous_line() {
  print!("{}[F", 27 as char);
}

fn clear_line() {
  // Move the cursor to the beginning of the line
  print!("\r");

  // Print whitespace characters to clear the line
  for _ in 0..get_terminal_width() {
    print!(" ");
  }

  print!("\r");
}

fn ansi_rgb(red: u8, green: u8, blue: u8) -> String {
  format!("{}[38;2;{red};{green};{blue}m", 27 as char)
}

fn lerp(a: u8, b: u8, t: f64) -> u8 {
  ((1.0 - t) * (a as f64) + t * (b as f64)).round() as u8
}

fn get_terminal_width() -> u16 {
  termsize::get().unwrap_or(termsize::Size { rows: 10, cols: 80 }).cols
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
